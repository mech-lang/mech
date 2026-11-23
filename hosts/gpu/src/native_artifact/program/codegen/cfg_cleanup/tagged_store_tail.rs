//! Keep the known edge tag across a common store without evaluating it twice.
//! Each original incoming path still executes the same ordered operations.
use super::{route_uses::Uses, word_ranges::literal};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, BlockArg, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_TAIL_BLOCKS: usize = 8;
const MAX_TAIL_INSTRUCTIONS: usize = 12;

struct Edge {
    jump: Inst,
    arguments: Vec<Value>,
    destination: Block,
    outgoing: Vec<Value>,
}

struct Tail {
    join: Block,
    instructions: Vec<Inst>,
    edges: [Edge; 2],
}

fn tag_parameter(f: &Function, value: Value, join: Block) -> Option<usize> {
    let mut value = f.dfg.resolve_aliases(value);
    if let Some(inst) = f.dfg.value_def(value).inst()
        && let Data::BinaryImm64 {
            opcode: Opcode::BandImm,
            arg,
            imm,
        } = f.dfg.insts[inst]
        && imm.bits() == 1
    {
        value = f.dfg.resolve_aliases(arg);
    }
    match f.dfg.value_def(value) {
        ValueDef::Param(block, index) if block == join => Some(usize::from(index)),
        _ => None,
    }
}

fn discover(f: &Function, cfg: &ControlFlowGraph, uses: &Uses, join: Block) -> Option<Tail> {
    if f.dfg.block_params(join).is_empty() || f.layout.entry_block() == Some(join) {
        return None;
    }
    let mut incoming = cfg.pred_iter(join);
    let first = incoming.next()?;
    let second = incoming.next()?;
    if incoming.next().is_some() {
        return None;
    }

    let mut blocks = BTreeSet::new();
    let mut instructions = Vec::new();
    let mut stores = 0;
    let mut current = join;
    let (tag, branches) = loop {
        if blocks.len() == MAX_TAIL_BLOCKS || !blocks.insert(current) {
            return None;
        }
        let end = f.layout.last_inst(current)?;
        for inst in f
            .layout
            .block_insts(current)
            .take_while(|&inst| inst != end)
        {
            match f.dfg.insts[inst].opcode() {
                Opcode::Store if stores == 0 => stores += 1,
                Opcode::Iconst
                | Opcode::Iadd
                | Opcode::IaddImm
                | Opcode::Isub
                | Opcode::Imul
                | Opcode::ImulImm
                | Opcode::Ishl
                | Opcode::IshlImm
                | Opcode::BandImm
                | Opcode::Uextend
                | Opcode::Ireduce => (),
                _ => return None,
            }
            if instructions.len() == MAX_TAIL_INSTRUCTIONS {
                return None;
            }
            instructions.push(inst);
        }
        match f.dfg.insts[end] {
            Data::Jump { destination, .. } if destination.len(&f.dfg.value_lists) == 0 => {
                let next = destination.block(&f.dfg.value_lists);
                let mut pred = cfg.pred_iter(next);
                if f.layout.entry_block() == Some(next)
                    || !f.dfg.block_params(next).is_empty()
                    || pred.next()?.inst != end
                    || pred.next().is_some()
                {
                    return None;
                }
                current = next;
            }
            Data::Brif { arg, blocks, .. } if stores == 1 => {
                break (tag_parameter(f, arg, join)?, blocks);
            }
            _ => return None,
        }
    };
    if branches
        .iter()
        .any(|call| blocks.contains(&call.block(&f.dfg.value_lists)))
    {
        return None;
    }
    // Escapes through the terminating jump arguments are remapped below. A
    // direct use after the tail would need a new phi, so decline that shape.
    if f.dfg
        .block_params(join)
        .iter()
        .copied()
        .chain(
            instructions
                .iter()
                .flat_map(|&inst| f.dfg.inst_results(inst).iter().copied()),
        )
        .any(|value| uses.used_outside(value, &blocks))
    {
        return None;
    }

    let resolve = |argument: BlockArg| argument.as_value().map(|v| f.dfg.resolve_aliases(v));
    let edge = |pred: cranelift_codegen::flowgraph::BlockPredecessor| {
        if blocks.contains(&pred.block) {
            return None;
        }
        let Data::Jump { destination, .. } = f.dfg.insts[pred.inst] else {
            return None;
        };
        let arguments = destination
            .args(&f.dfg.value_lists)
            .map(resolve)
            .collect::<Option<Vec<_>>>()?;
        let selected = match literal(f, *arguments.get(tag)?)? {
            0 => branches[1],
            1 => branches[0],
            _ => return None,
        };
        Some(Edge {
            jump: pred.inst,
            arguments,
            destination: selected.block(&f.dfg.value_lists),
            outgoing: selected
                .args(&f.dfg.value_lists)
                .map(resolve)
                .collect::<Option<Vec<_>>>()?,
        })
    };
    let edges = [edge(first)?, edge(second)?];
    (edges[0].destination != edges[1].destination).then_some(Tail {
        join,
        instructions,
        edges,
    })
}

pub(super) fn specialize(f: &mut Function, cfg: &ControlFlowGraph) -> bool {
    if f.dfg.values_labels.is_some() || !f.debug_tags.is_empty() {
        return false;
    }
    let uses = Uses::new(f);
    let Some(tail) = f
        .layout
        .blocks()
        .find_map(|block| discover(f, cfg, &uses, block))
    else {
        return false;
    };
    for edge in tail.edges {
        let mut values = f
            .dfg
            .block_params(tail.join)
            .iter()
            .copied()
            .zip(edge.arguments)
            .collect::<BTreeMap<_, _>>();
        for &inst in &tail.instructions {
            let copy = f.dfg.clone_inst(inst);
            f.dfg
                .map_inst_values(copy, |value| values.get(&value).copied().unwrap_or(value));
            f.layout.insert_inst(copy, edge.jump);
            f.set_srcloc(copy, f.srcloc(inst));
            values.extend(
                f.dfg
                    .inst_results(inst)
                    .iter()
                    .copied()
                    .zip(f.dfg.inst_results(copy).iter().copied()),
            );
        }
        let outgoing = edge
            .outgoing
            .into_iter()
            .map(|value| BlockArg::Value(values.get(&value).copied().unwrap_or(value)))
            .collect::<Vec<_>>();
        f.dfg.replace(edge.jump).jump(edge.destination, &outgoing);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::Context;
    use cranelift_codegen::ir::{AbiParam, MemFlags, condcodes::IntCC, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    fn example(module: &JITModule, escape: bool, two_stores: bool) -> Context {
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 2];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut state);
        let entry = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        let join = b.create_block();
        let after_store = b.create_block();
        let keep = b.create_block();
        let finish = b.create_block();
        b.append_block_params_for_function_params(entry);
        let [address, input] = b.block_params(entry).try_into().unwrap();
        let tag = b.append_block_param(join, types::I8);
        let payload = b.append_block_param(join, types::I64);
        let kept = b.append_block_param(keep, types::I64);
        let done = b.append_block_param(finish, types::I64);
        b.switch_to_block(entry);
        let move_value = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, input, 128);
        b.ins().brif(move_value, yes, &[], no, &[]);
        b.switch_to_block(yes);
        let one = b.ins().iconst(types::I8, 1);
        b.ins().jump(join, &[one.into(), input.into()]);
        b.switch_to_block(no);
        let zero = b.ins().iconst(types::I8, 0);
        let value = b.ins().iadd_imm(input, -10);
        b.ins().jump(join, &[zero.into(), value.into()]);
        b.switch_to_block(join);
        let word = b.ins().ireduce(types::I32, payload);
        b.ins().store(MemFlags::new(), word, address, 0);
        if two_stores {
            b.ins().store(MemFlags::new(), word, address, 4);
        }
        b.ins().jump(after_store, &[]);
        b.switch_to_block(after_store);
        let low = b.ins().band_imm(tag, 1);
        b.ins()
            .brif(low, keep, &[payload.into()], finish, &[input.into()]);
        b.switch_to_block(keep);
        b.ins().return_(&[kept]);
        b.switch_to_block(finish);
        let original = if escape { payload } else { done };
        let result = b.ins().iadd_imm(original, 1);
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
        context
    }

    #[test]
    fn specializes_one_store_and_refuses_external_definitions_and_extra_effects() {
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let mut context = example(&module, false, false);
        let changed =
            super::super::specialize_tagged_store_tails(&mut context, module.isa()).unwrap();
        assert_eq!(changed, 1);
        let id = module
            .declare_function("tagged_store_tail", Linkage::Local, &context.func.signature)
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        type Entry = unsafe extern "C" fn(*mut u32, u64) -> u64;
        let call: Entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        for value in [0_u64, 10, 128, 129, u32::MAX as u64, u64::MAX] {
            let mut words = [999; 2];
            let result = unsafe { call(words.as_mut_ptr(), value) };
            let (stored, returned) = if value > 128 {
                (value, value)
            } else {
                (value.wrapping_sub(10), value + 1)
            };
            assert_eq!((words, result), ([stored as u32, 999], returned));
        }
        for (escape, two_stores) in [(true, false), (false, true)] {
            let mut context = example(&module, escape, two_stores);
            let original = context.func.display().to_string();
            assert_eq!(
                super::super::specialize_tagged_store_tails(&mut context, module.isa()).unwrap(),
                0
            );
            assert_eq!(context.func.display().to_string(), original);
        }
    }
}

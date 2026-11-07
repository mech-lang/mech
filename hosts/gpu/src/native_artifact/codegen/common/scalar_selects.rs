//! Convert small pure scalar choices, including empty source-edge chains, to
//! SSA selects. Resource debits, memory, calls and checked error edges cannot
//! enter an arm: only explicitly nontrapping integer instructions move.
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{
    Block, BlockArg, Function, Inst, InstBuilder, InstructionData, Opcode as O, Value,
};

const MAX_BLOCKS: usize = 8;
const MAX_INSTRUCTIONS: usize = 128;
const MAX_SELECTS: usize = 8;

struct Arm {
    blocks: Vec<Block>,
    join: Block,
    arguments: Vec<Value>,
}

fn pure(function: &Function, inst: Inst) -> bool {
    matches!(
        function.dfg.insts[inst].opcode(),
        O::Iconst
            | O::Iadd
            | O::IaddImm
            | O::Isub
            | O::Imul
            | O::ImulImm
            | O::Band
            | O::BandImm
            | O::Bor
            | O::BorImm
            | O::Bxor
            | O::BxorImm
            | O::Bnot
            | O::Icmp
            | O::IcmpImm
            | O::Ishl
            | O::IshlImm
            | O::Ushr
            | O::UshrImm
            | O::Sshr
            | O::SshrImm
            | O::Select
    ) && function.dfg.inst_results(inst).len() == 1
        && function
            .dfg
            .inst_results(inst)
            .iter()
            .chain(function.dfg.inst_args(inst))
            .all(|&value| {
                let ty = function.dfg.value_type(value);
                ty.is_int() && ty.bits() <= 64
            })
}

fn arm(function: &Function, root: Block, start: Block, predecessors: &[usize]) -> Option<Arm> {
    let mut blocks = Vec::new();
    let mut current = start;
    let mut instructions = 0;
    for _ in 0..MAX_BLOCKS {
        // An arm must be reachable only from this branch and its own chain.
        // Never speculate a shared continuation, loop header or block phi.
        if current == root
            || blocks.contains(&current)
            || predecessors[current.as_u32() as usize] != 1
            || !function.dfg.block_params(current).is_empty()
        {
            return None;
        }
        let end = function.layout.last_inst(current)?;
        let InstructionData::Jump { destination, .. } = function.dfg.insts[end] else {
            return None;
        };
        for inst in function.layout.block_insts(current) {
            if inst != end {
                if !pure(function, inst) {
                    return None;
                }
                instructions += 1;
                if instructions > MAX_INSTRUCTIONS {
                    return None;
                }
            }
        }
        blocks.push(current);
        let join = destination.block(&function.dfg.value_lists);
        if predecessors[join.as_u32() as usize] != 1 {
            let arguments = destination
                .args(&function.dfg.value_lists)
                .map(|arg| arg.as_value().map(|v| function.dfg.resolve_aliases(v)))
                .collect::<Option<Vec<_>>>()?;
            return (join != root && !blocks.contains(&join)).then_some(Arm {
                blocks,
                join,
                arguments,
            });
        }
        if destination.len(&function.dfg.value_lists) != 0 {
            return None;
        }
        current = join;
    }
    None
}

pub(in crate::native_artifact) fn optimize(function: &mut Function) -> usize {
    let mut changed = 0;
    for _ in 0..4 {
        let mut predecessors = vec![0usize; function.dfg.num_blocks()];
        for block in function.layout.blocks() {
            if let Some(inst) = function.layout.last_inst(block) {
                for call in function.dfg.insts[inst]
                    .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables)
                {
                    predecessors[call.block(&function.dfg.value_lists).as_u32() as usize] += 1;
                }
            }
        }
        let mut round = 0;
        let candidates = function.layout.blocks().collect::<Vec<_>>();
        for root in candidates {
            if !function.layout.is_block_inserted(root) {
                continue;
            }
            let Some(branch) = function.layout.last_inst(root) else {
                continue;
            };
            let InstructionData::Brif { arg, blocks, .. } = function.dfg.insts[branch] else {
                continue;
            };
            if blocks
                .iter()
                .any(|call| call.len(&function.dfg.value_lists) != 0)
            {
                continue;
            }
            let Some(yes) = arm(
                function,
                root,
                blocks[0].block(&function.dfg.value_lists),
                &predecessors,
            ) else {
                continue;
            };
            let Some(no) = arm(
                function,
                root,
                blocks[1].block(&function.dfg.value_lists),
                &predecessors,
            ) else {
                continue;
            };
            if yes.join != no.join
                || yes.blocks.iter().any(|b| no.blocks.contains(b))
                || yes.arguments.len() != no.arguments.len()
                || yes.arguments.len() != function.dfg.block_params(yes.join).len()
            {
                continue;
            }
            let pairs = yes
                .arguments
                .iter()
                .copied()
                .zip(no.arguments.iter().copied());
            let mut distinct = Vec::new();
            for (a, b) in pairs {
                if a != b && !distinct.contains(&(a, b)) {
                    distinct.push((a, b));
                }
            }
            if distinct.is_empty()
                || distinct.len() > MAX_SELECTS
                || distinct.iter().any(|&(a, b)| {
                    let ty = function.dfg.value_type(a);
                    ty != function.dfg.value_type(b) || !ty.is_int() || ty.bits() > 64
                })
            {
                continue;
            }
            // Move instructions in their original arm order, before the
            // branch. The explicit whitelist excludes effects and traps.
            // Original SSA and the sole-predecessor proof give dominance.
            for block in yes.blocks.iter().chain(&no.blocks) {
                let end = function.layout.last_inst(*block).unwrap();
                for inst in function.layout.block_insts(*block).collect::<Vec<_>>() {
                    function.layout.remove_inst(inst);
                    if inst != end {
                        function.layout.insert_inst(inst, branch);
                    }
                }
                function.layout.remove_block(*block);
            }
            let mut selections = Vec::new();
            {
                let mut cursor = FuncCursor::new(function).at_inst(branch);
                for &(a, b) in &distinct {
                    let selected = cursor.ins().select(arg, a, b);
                    selections.push(((a, b), selected));
                }
            }
            let arguments = yes
                .arguments
                .iter()
                .copied()
                .zip(no.arguments.iter().copied())
                .map(|(a, b)| {
                    BlockArg::Value(if a == b {
                        a
                    } else {
                        selections
                            .iter()
                            .find(|(pair, _)| *pair == (a, b))
                            .unwrap()
                            .1
                    })
                })
                .collect::<Vec<_>>();
            function.dfg.replace(branch).jump(yes.join, &arguments);
            round += 1;
        }
        changed += round;
        if round == 0 {
            break;
        }
    }
    changed
}

// AArch64's I128 select consumes one flags producer for both I64 halves.
// Pair only adjacent independent choices: all operands already dominate the
// first select, no memory/error/work instruction moves, and nonzero truth is
// exactly the original select's truth (including raw noncanonical words).
pub(in crate::native_artifact) fn pair_selects(function: &mut Function) -> usize {
    use cranelift_codegen::ir::types;
    let mut changed = 0;
    for block in function.layout.blocks().collect::<Vec<_>>() {
        let instructions = function.layout.block_insts(block).collect::<Vec<_>>();
        let mut at = 0;
        while at + 1 < instructions.len() {
            let first = instructions[at];
            let second = instructions[at + 1];
            at += 1;
            let InstructionData::Ternary { opcode: O::Select, args: a } = function.dfg.insts[first] else { continue };
            let InstructionData::Ternary { opcode: O::Select, args: b } = function.dfg.insts[second] else { continue };
            let x = function.dfg.first_result(first);
            let y = function.dfg.first_result(second);
            let a = a.map(|v| function.dfg.resolve_aliases(v));
            let b = b.map(|v| function.dfg.resolve_aliases(v));
            if a[0] != b[0] || function.dfg.value_type(x) != types::I64
                || function.dfg.value_type(y) != types::I64 || b.contains(&x)
            { continue; }
            let (lo, hi) = {
                let mut cursor = FuncCursor::new(function).at_inst(first);
                let yes = cursor.ins().iconcat(a[1], b[1]);
                let no = cursor.ins().iconcat(a[2], b[2]);
                let selected = cursor.ins().select(a[0], yes, no);
                cursor.ins().isplit(selected)
            };
            function.dfg.clear_results(first);
            function.dfg.clear_results(second);
            function.dfg.change_to_alias(x, lo);
            function.dfg.change_to_alias(y, hi);
            function.layout.remove_inst(first);
            function.layout.remove_inst(second);
            changed += 1;
            at += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, MemFlags, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    #[test]
    fn paired_selects_preserve_full_width_lanes_and_raw_truth() {
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names()).unwrap(),
        );
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 6];
        let id = module.declare_function("paired_choices", Linkage::Local, &context.func.signature).unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let a = b.block_params(entry).to_vec();
            let x = b.ins().select(a[0], a[1], a[2]);
            let y = b.ins().select(a[0], a[3], a[4]);
            b.ins().store(MemFlags::new(), x, a[5], 0);
            b.ins().store(MemFlags::new(), y, a[5], 8);
            b.ins().return_(&[]);
            b.seal_all_blocks(); b.finalize();
        }
        assert_eq!(pair_selects(&mut context.func), 1);
        assert_eq!(pair_selects(&mut context.func), 0);
        context.compute_cfg(); context.compute_domtree(); context.verify(module.isa()).unwrap();
        module.define_function(id, &mut context).unwrap(); module.finalize_definitions().unwrap();
        // SAFETY: the live JIT function has these six word parameters, no
        // return value, and writes only the supplied two-word output bank.
        let entry: unsafe extern "C" fn(u64,u64,u64,u64,u64,*mut u64) = unsafe {core::mem::transmute(module.get_finalized_function(id))};
        let values = [0,1,2,(1u64<<53)+1,1u64<<63,u64::MAX];
        for condition in [0,1,2,1u64<<63,u64::MAX] {
            for a in values { for b in values {
                let mut output = [13u64,17,19,23];
                unsafe { entry(condition,a,b,b.wrapping_add(7),a.wrapping_add(11),output.as_mut_ptr().add(1)) };
                assert_eq!(output, if condition == 0 {[13,b,a.wrapping_add(11),23]} else {[13,a,b.wrapping_add(7),23]});
            }}
        }
    }

    #[test]
    fn paired_selects_decline_dependencies_distinct_conditions_and_effect_gaps() {
        for variant in 0..4 {
            let mut function = Function::new();
            function.signature.params = vec![AbiParam::new(types::I64); 4];
            function.signature.returns = vec![AbiParam::new(types::I64)];
            let mut fc = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut function, &mut fc);
                let entry=b.create_block(); b.append_block_params_for_function_params(entry); b.switch_to_block(entry);
                let a=b.block_params(entry).to_vec();
                let x=b.ins().select(a[0],a[1],a[2]);
                if variant==2 {b.ins().store(MemFlags::new(),a[1],a[3],0);}
                let y = if variant==3 {
                    let c=b.ins().ireduce(types::I32,a[1]); let d=b.ins().ireduce(types::I32,a[2]);
                    let v=b.ins().select(a[0],c,d); b.ins().uextend(types::I64,v)
                } else { b.ins().select(if variant==1{a[3]}else{a[0]},if variant==0{x}else{a[2]},a[1]) };
                b.ins().return_(&[y]); b.seal_all_blocks(); b.finalize();
            }
            assert_eq!(pair_selects(&mut function),0,"variant {variant}");
        }
    }

    #[test]
    fn scalar_select_empty_chains_keep_full_width_values_and_nonzero_truth() {
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let id = module
            .declare_function("select_chains", Linkage::Local, &context.func.signature)
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            let yes = b.create_block();
            let next = b.create_block();
            let no = b.create_block();
            let join = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(join, types::I64);
            b.switch_to_block(entry);
            let condition = b.block_params(entry)[0];
            let a = b.block_params(entry)[1];
            let c = b.block_params(entry)[2];
            b.ins().brif(condition, yes, &[], no, &[]);
            b.switch_to_block(yes);
            let left = b.ins().iadd_imm(a, 1);
            b.ins().jump(next, &[]);
            b.switch_to_block(next);
            b.ins().jump(join, &[left.into()]);
            b.switch_to_block(no);
            let right = b.ins().iadd_imm(c, 2);
            b.ins().jump(join, &[right.into()]);
            b.switch_to_block(join);
            let selected = b.block_params(join)[0];
            b.ins().return_(&[selected]);
            b.seal_all_blocks();
            b.finalize();
        }
        assert_eq!(optimize(&mut context.func), 1);
        context.compute_cfg();
        context.compute_domtree();
        context.verify(module.isa()).unwrap();
        assert_eq!(context.func.layout.blocks().count(), 2);
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: this matching JIT function is owned by `module`, remains
        // live during all calls and takes/returns exactly the declared words.
        let entry: unsafe extern "C" fn(u64, u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for condition in [0, 1, 2, u64::MAX] {
            for (a, b) in [(0, 1), (u64::MAX, 7), (1 << 63, u64::MAX)] {
                assert_eq!(
                    unsafe { entry(condition, a, b) },
                    if condition == 0 {
                        b.wrapping_add(2)
                    } else {
                        a.wrapping_add(1)
                    }
                );
            }
        }
    }

    #[test]
    fn scalar_select_rejects_memory_traps_and_shared_arms() {
        for variant in 0..4 {
            let mut function = Function::new();
            function.signature.params = vec![AbiParam::new(types::I64); 3];
            function.signature.returns = vec![AbiParam::new(types::I64)];
            let mut fc = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut function, &mut fc);
                let entry = b.create_block();
                let yes = b.create_block();
                let no = b.create_block();
                let join = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.append_block_param(join, types::I64);
                b.switch_to_block(entry);
                let condition = b.block_params(entry)[0];
                let a = b.block_params(entry)[1];
                let c = b.block_params(entry)[2];
                b.ins().brif(condition, yes, &[], no, &[]);
                b.switch_to_block(yes);
                let value = match variant {
                    0 => b.ins().load(types::I64, MemFlags::new(), a, 0),
                    1 => {
                        b.ins().store(MemFlags::new(), c, a, 0);
                        c
                    }
                    2 => b.ins().udiv(a, c),
                    _ => a,
                };
                b.ins().jump(join, &[value.into()]);
                b.switch_to_block(no);
                if variant == 3 {
                    b.ins().jump(yes, &[]);
                } else {
                    b.ins().jump(join, &[c.into()]);
                }
                b.switch_to_block(join);
                let selected = b.block_params(join)[0];
                b.ins().return_(&[selected]);
                b.seal_all_blocks();
                b.finalize();
            }
            assert_eq!(optimize(&mut function), 0, "variant {variant}");
        }
    }
}

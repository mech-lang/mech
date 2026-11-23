//! Prove unit-dimension tests from current SSA values, including loop phis.
use crate::native_artifact::program::codegen::cfg_cleanup::incoming_block_arguments;
use cranelift_codegen::{
    Context,
    ir::{BlockCall, Function, Inst, InstructionData, Opcode, Value, ValueDef, condcodes::IntCC},
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Guard {
    pub header: usize,
    pub array: usize,
    pub branch: Inst,
}

fn literal(f: &Function, value: Value) -> Option<i64> {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, _) = f.dfg.value_def(value) else {
        return None;
    };
    let InstructionData::UnaryImm {
        opcode: Opcode::Iconst,
        imm,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    Some(imm.bits())
}

fn compared_with_one(f: &Function, test: Value) -> Option<Value> {
    let ValueDef::Result(inst, _) = f.dfg.value_def(f.dfg.resolve_aliases(test)) else {
        return None;
    };
    match f.dfg.insts[inst] {
        InstructionData::IntCompareImm {
            cond: IntCC::Equal,
            arg,
            imm,
            ..
        } if imm.bits() == 1 => Some(arg),
        InstructionData::IntCompare {
            cond: IntCC::Equal,
            args: [a, b],
            ..
        } if literal(f, a) == Some(1) => Some(b),
        InstructionData::IntCompare {
            cond: IntCC::Equal,
            args: [a, b],
            ..
        } if literal(f, b) == Some(1) => Some(a),
        _ => None,
    }
}

fn always_one(context: &Context, value: Value) -> Result<usize, (Value, &'static str)> {
    let f = &context.func;
    let mut pending = vec![value];
    let mut seen = BTreeSet::new();
    let mut literal_seeds = Vec::new();
    let mut users = BTreeMap::<Value, Vec<Value>>::new();
    while let Some(value) = pending.pop() {
        let value = f.dfg.resolve_aliases(value);
        if !seen.insert(value) {
            continue;
        }
        if seen.len() > 4096 {
            return Err((value, "phi proof budget"));
        }
        if literal(f, value) == Some(1) {
            literal_seeds.push(value);
            continue;
        }
        let ValueDef::Param(block, ordinal) = f.dfg.value_def(value) else {
            return Err((value, "computed geometry"));
        };
        let mut has_input = false;
        for (_, source) in incoming_block_arguments(f, &context.cfg, block, ordinal)
            .filter(|(pred, _)| context.domtree.is_reachable(*pred))
        {
            let source = source.ok_or((value, "special edge operand"))?;
            users.entry(source).or_default().push(value);
            pending.push(source);
            has_input = true;
        }
        if !has_input {
            return Err((value, "entry geometry"));
        }
    }
    // A cyclic value graph with no initial literal cannot prove an equality.
    let mut initialized = BTreeSet::new();
    while let Some(value) = literal_seeds.pop() {
        if initialized.insert(value)
            && let Some(next) = users.get(&value)
        {
            literal_seeds.extend(next);
        }
    }
    if seen.len() != initialized.len() {
        return Err((value, "uninitialized phi cycle"));
    }
    Ok(seen.len())
}

fn prove(context: &Context, test: Value) -> Result<usize, (Value, &'static str)> {
    let f = &context.func;
    let test = f.dfg.resolve_aliases(test);
    if let Some(dimension) = compared_with_one(f, test) {
        return always_one(context, dimension);
    }
    if let ValueDef::Result(inst, _) = f.dfg.value_def(test)
        && let InstructionData::Binary {
            opcode: Opcode::Bor,
            args: [row, column],
        } = f.dfg.insts[inst]
        && let (Some(row), Some(column)) = (compared_with_one(f, row), compared_with_one(f, column))
    {
        return always_one(context, row).or_else(|_| always_one(context, column));
    }
    Err((test, "different guard expression"))
}

pub(super) fn fold(context: &mut Context, guards: &[Guard], apply: bool) -> usize {
    context.flowgraph();
    let mut changes = Vec::new();
    for guard in guards {
        let Some(block) = context.func.layout.inst_block(guard.branch) else {
            continue;
        };
        if !context.domtree.is_reachable(block) {
            continue;
        }
        let InstructionData::Brif { arg, blocks, .. } = context.func.dfg.insts[guard.branch] else {
            continue;
        };
        match prove(context, arg) {
            Ok(nodes) => {
                eprintln!(
                    "compact vector {} array {} proved with {nodes} current values",
                    guard.header, guard.array
                );
                changes.push((guard.branch, blocks[0]));
            }
            Err((value, reason)) => {
                let f = &context.func;
                eprintln!(
                    "compact vector {} array {} keeps guard: {reason} at {value} {:?}",
                    guard.header,
                    guard.array,
                    f.dfg.value_def(value)
                );
                if let ValueDef::Result(inst, _) = f.dfg.value_def(value) {
                    eprintln!("compact vector definition {}", f.dfg.display_inst(inst));
                }
            }
        }
    }
    if apply {
        apply_proofs(context, &changes);
    }
    changes.len()
}

pub(super) fn fold_unit_branches(context: &mut Context, apply: bool) -> usize {
    context.flowgraph();
    let changes = context
        .func
        .layout
        .blocks()
        .filter(|&block| context.domtree.is_reachable(block))
        .filter_map(|block| context.func.layout.last_inst(block))
        .filter_map(|inst| {
            let InstructionData::Brif { arg, blocks, .. } = context.func.dfg.insts[inst] else {
                return None;
            };
            prove(context, arg).is_ok().then_some((inst, blocks[0]))
        })
        .collect::<Vec<_>>();
    if apply {
        apply_proofs(context, &changes);
    }
    changes.len()
}

fn apply_proofs(context: &mut Context, changes: &[(Inst, BlockCall)]) {
    for &(inst, edge) in changes {
        context.func.dfg.insts[inst] = InstructionData::Jump {
            opcode: Opcode::Jump,
            destination: edge,
        };
    }
    if !changes.is_empty() {
        context.flowgraph();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    #[test]
    fn vector_guard_retains_unknown_or_changing_dimensions() {
        let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let mut functions = Vec::new();
        for (name, known_entry, changed) in [
            ("stable", true, false),
            ("changing", true, true),
            ("borrowed", false, false),
        ] {
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 2];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let mut builder = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
            let entry = b.create_block();
            let loop_head = b.create_block();
            let next = b.create_block();
            let check = b.create_block();
            let vector = b.create_block();
            let general = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(loop_head, types::I64);
            b.append_block_param(loop_head, types::I64);
            b.switch_to_block(entry);
            let [iterations, input] = *b.block_params(entry) else {
                unreachable!()
            };
            let one = b.ins().iconst(types::I64, 1);
            let two = b.ins().iconst(types::I64, 2);
            let seed = if known_entry { one } else { input };
            b.ins().jump(loop_head, &[iterations.into(), seed.into()]);
            b.switch_to_block(loop_head);
            let [remaining, dimension] = *b.block_params(loop_head) else {
                unreachable!()
            };
            b.ins().brif(remaining, next, &[], check, &[]);
            b.switch_to_block(next);
            let remaining = b.ins().iadd_imm(remaining, -1);
            let carry = if changed { two } else { dimension };
            b.ins().jump(loop_head, &[remaining.into(), carry.into()]);
            b.switch_to_block(check);
            let row = b.ins().icmp_imm(IntCC::Equal, dimension, 1);
            let column = b.ins().icmp_imm(IntCC::Equal, input, 1);
            let flat = b.ins().bor(row, column);
            let branch = b.ins().brif(flat, vector, &[], general, &[]);
            b.switch_to_block(vector);
            b.ins().return_(&[one]);
            b.switch_to_block(general);
            b.ins().return_(&[two]);
            b.seal_all_blocks();
            b.finalize();
            let expected = usize::from(known_entry && !changed);
            assert_eq!(fold_unit_branches(&mut context, false), expected);
            assert_eq!(
                fold(
                    &mut context,
                    &[Guard {
                        header: 0,
                        array: 0,
                        branch
                    }],
                    true
                ),
                expected
            );
            context.verify(module.isa()).unwrap();
            let id = module
                .declare_function(name, Linkage::Export, &context.func.signature)
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            functions.push(id);
        }
        module.finalize_definitions().unwrap();
        for (index, function) in functions.into_iter().enumerate() {
            // All test functions have the same two-word C ABI.
            let call = unsafe {
                std::mem::transmute::<_, extern "C" fn(u64, u64) -> u64>(
                    module.get_finalized_function(function),
                )
            };
            assert_eq!(call(3, 1), 1);
            assert_eq!(call(3, 2), if index == 0 { 1 } else { 2 });
            assert_eq!(call(0, 2), if index == 2 { 2 } else { 1 });
        }
    }
}

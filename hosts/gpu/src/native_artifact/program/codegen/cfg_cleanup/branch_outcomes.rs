//! Use the known outcomes of an integer comparison disjunction at a join.
use super::word_ranges::literal;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value,
};

fn comparison_word(f: &Function, value: Value, depth: usize) -> bool {
    let ty = f.dfg.value_type(value);
    if depth == 0 || !ty.is_int() || !matches!(ty.bits(), 8 | 16 | 32 | 64) {
        return false;
    }
    let Some(inst) = f.dfg.value_def(value).inst() else {
        return false;
    };
    match f.dfg.insts[inst] {
        Data::IntCompare { .. } | Data::IntCompareImm { .. } => true,
        Data::Unary {
            opcode: Opcode::Uextend | Opcode::Ireduce,
            arg,
        } => comparison_word(f, arg, depth - 1),
        _ => false,
    }
}

fn negated_disjunction(f: &Function, condition: Value) -> Option<Value> {
    let inst = f.dfg.value_def(condition).inst()?;
    let source = match f.dfg.insts[inst] {
        Data::BinaryImm64 {
            opcode: Opcode::BxorImm,
            arg,
            imm,
        } if imm.bits() == 1 => arg,
        Data::Binary {
            opcode: Opcode::Bxor,
            args: [left, right],
        } if literal(f, right) == Some(1) => left,
        Data::Binary {
            opcode: Opcode::Bxor,
            args: [left, right],
        } if literal(f, left) == Some(1) => right,
        _ => return None,
    };
    let inst = f.dfg.value_def(source).inst()?;
    let Data::Binary {
        opcode: Opcode::Bor,
        args,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    args.into_iter()
        .all(|arg| comparison_word(f, arg, 4))
        .then_some(source)
}

fn incoming(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    branch: Inst,
    join: Block,
    other: Block,
) -> Option<Vec<Inst>> {
    // Dominance by the other destination proves the other branch outcome only
    // if the selecting branch is its sole incoming edge.
    let mut other_preds = cfg.pred_iter(other);
    if other_preds.next()?.inst != branch || other_preds.next().is_some() {
        return None;
    }
    let edges = cfg
        .pred_iter(join)
        .map(|pred| {
            if !matches!(f.dfg.insts[pred.inst].opcode(), Opcode::Brif | Opcode::Jump)
                || (pred.inst != branch && !dom.dominates(other, pred.block, &f.layout))
            {
                return None;
            }
            Some(pred.inst)
        })
        .collect::<Option<Vec<_>>>()?;
    (edges.len() > 1).then_some(edges)
}

pub(super) fn carry(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> usize {
    let blocks = f.layout.blocks().collect::<Vec<_>>();
    let mut joins = 0;
    for block in blocks.iter().copied() {
        if !dom.is_reachable(block) {
            continue;
        }
        let Some(branch) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif {
            arg,
            blocks: destinations,
            ..
        } = f.dfg.insts[branch]
        else {
            continue;
        };
        let Some(source) = negated_disjunction(f, arg) else {
            continue;
        };
        let [taken, not_taken] = destinations.map(|call| call.block(&f.dfg.value_lists));
        if taken == not_taken || taken == block || not_taken == block {
            continue;
        }
        for (join, other, direct_value) in [(taken, not_taken, 0), (not_taken, taken, 1)] {
            if f.layout.entry_block() == Some(join) || !dom.dominates(block, join, &f.layout) {
                continue;
            }
            let Some(edges) = incoming(f, cfg, dom, branch, join, other) else {
                continue;
            };
            let users = blocks
                .iter()
                .copied()
                .filter(|&b| dom.dominates(join, b, &f.layout))
                .flat_map(|b| f.layout.block_insts(b))
                .filter(|&inst| f.dfg.inst_values(inst).any(|value| value == source))
                .collect::<Vec<_>>();
            if users.is_empty() {
                continue;
            }
            let ty = f.dfg.value_type(source);
            let param = f.dfg.append_block_param(join, ty);
            let mut cursor = FuncCursor::new(f).at_inst(branch);
            let direct = cursor.ins().iconst(ty, direct_value);
            let indirect = cursor.ins().iconst(ty, 1 - direct_value);
            for edge in edges {
                let value = if edge == branch { direct } else { indirect };
                let dfg = &mut f.dfg;
                for call in dfg.insts[edge]
                    .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
                {
                    if call.block(&dfg.value_lists) == join {
                        call.append_argument(value, &mut dfg.value_lists);
                    }
                }
            }
            for inst in users {
                f.dfg
                    .map_inst_values(inst, |value| if value == source { param } else { value });
            }
            joins += 1;
        }
    }
    joins
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, condcodes::IntCC, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    #[test]
    fn join_receives_the_outcome_only_when_every_incoming_edge_proves_it() {
        for bypass in [false, true] {
            let mut f = Function::new();
            f.signature.params = vec![AbiParam::new(types::I64); 2];
            f.signature.returns.push(AbiParam::new(types::I64));
            let mut storage = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut f, &mut storage);
            let entry = b.create_block();
            let test = b.create_block();
            let other = b.create_block();
            let join = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let [left, right] = b.block_params(entry).try_into().unwrap();
            let a = b.ins().icmp(IntCC::UnsignedLessThan, left, right);
            let c = b.ins().icmp_imm(IntCC::Equal, right, 7);
            let a = b.ins().uextend(types::I64, a);
            let c = b.ins().uextend(types::I64, c);
            let source = b.ins().bor(a, c);
            let condition = b.ins().bxor_imm(source, 1);
            if bypass {
                b.ins().brif(left, test, &[], join, &[]);
            } else {
                b.ins().jump(test, &[]);
            }
            b.switch_to_block(test);
            let branch = b.ins().brif(condition, other, &[], join, &[]);
            b.switch_to_block(other);
            let indirect = b.ins().jump(join, &[]);
            b.switch_to_block(join);
            let ret = b.ins().return_(&[source]);
            b.seal_all_blocks();
            b.finalize();

            let cfg = ControlFlowGraph::with_function(&f);
            let dom = DominatorTree::with_function(&f, &cfg);
            assert_eq!(carry(&mut f, &cfg, &dom), usize::from(!bypass));
            let flags =
                cranelift_codegen::settings::Flags::new(cranelift_codegen::settings::builder());
            cranelift_codegen::verify_function(&f, &flags).unwrap();
            if bypass {
                assert_eq!(f.dfg.inst_args(ret), &[source]);
                continue;
            }
            assert_eq!(f.dfg.inst_args(ret), f.dfg.block_params(join));
            for (edge, expected) in [(branch, 1), (indirect, 0)] {
                let argument = f.dfg.insts[edge]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                    .iter()
                    .find(|call| call.block(&f.dfg.value_lists) == join)
                    .unwrap()
                    .args(&f.dfg.value_lists)
                    .next()
                    .unwrap()
                    .as_value()
                    .unwrap();
                assert_eq!(literal(&f, argument), Some(expected));
            }
        }
    }
}

//! Forward a byte order from two already loaded, admitted little-endian words.
//! Other paths keep their original checked byte reads and default behavior.
use super::*;
use crate::native_artifact::program::codegen::wide_equal::AdmittedWordMismatch;
use cranelift_codegen::ir::Endianness;

fn current_window(
    f: &Function,
    cfg: &ControlFlowGraph,
    block: Block,
    guard: Inst,
) -> Option<(Value, Value)> {
    let (parent, incoming) = super::super::window_bounds::sole_predecessor(cfg, block)?;
    if incoming != guard || f.layout.last_inst(parent) != Some(guard) {
        return None;
    }
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[guard] else {
        return None;
    };
    let [yes, no] = blocks.map(|call| call.block(&f.dfg.value_lists) == block);
    if !yes || no {
        return None;
    }
    let Data::IntCompareImm {
        cond: IntCC::UnsignedGreaterThanOrEqual,
        arg,
        imm,
        ..
    } = f.dfg.insts[comparison(f, arg)?]
    else {
        return None;
    };
    if imm.bits() != 8 {
        return None;
    }
    let remaining = current(f, cfg, arg);
    let ValueDef::Result(subtract, 0) = f.dfg.value_def(remaining) else {
        return None;
    };
    let Data::Binary {
        opcode: Op::Isub,
        args: [limit, position],
    } = f.dfg.insts[subtract]
    else {
        return None;
    };
    Some((position, limit))
}

fn walk<'a>(
    context: &'a Context,
    witness: &AdmittedWordMismatch,
    views: &BTreeMap<usize, (Value, Value)>,
) -> Option<(Walk<'a>, BlockCall)> {
    let (f, cfg, dom) = (&context.func, &context.cfg, &context.domtree);
    let branch = witness.branch;
    let block = f.layout.inst_block(branch)?;
    if !dom.is_reachable(block) || f.layout.last_inst(block) != Some(branch) {
        return None;
    }
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
        return None;
    };
    let difference = current(f, cfg, arg);
    let ValueDef::Result(xor, 0) = f.dfg.value_def(difference) else {
        return None;
    };
    let Data::Binary {
        opcode: Op::Bxor,
        args,
    } = f.dfg.insts[xor]
    else {
        return None;
    };
    let words = args.map(|arg| current(f, cfg, arg));
    let (position, limit) = current_window(f, cfg, block, witness.guard)?;
    let &(pointer, length) = views.get(&witness.array)?;
    let pointer = current(f, cfg, pointer);
    let mut symbols = Symbols::default();
    let length = symbols.existing(length);
    let length = symbols.address(f, cfg, length, 0)?;
    let mut byte = |word| {
        let ValueDef::Result(load, 0) = f.dfg.value_def(word) else {
            return None;
        };
        let Data::Load {
            opcode: Op::Load,
            arg,
            offset,
            flags,
        } = f.dfg.insts[load]
        else {
            return None;
        };
        if !witness.loads.contains(&load)
            || f.dfg.value_type(word) != types::I64
            || f.layout.inst_block(load) != Some(block)
            || !dom.dominates(load, branch, &f.layout)
            || flags.endianness(Endianness::Little) != Endianness::Little
        {
            return None;
        }
        let arg = symbols.existing(arg);
        let address = symbols
            .address(f, cfg, arg, i32::from(offset))?
            .plus_byte_offset();
        let mut index = address.clone();
        if index.words.remove(&pointer) != Some(1) {
            return None;
        }
        Some(Byte {
            value: word,
            address,
            flags: MemFlags::new(),
            bounds: Some((index, length.clone())),
            must_match_bound: true,
        })
    };
    if words[0] == words[1] {
        return None;
    }
    let bytes = [byte(words[0])?, byte(words[1])?];
    let position = symbols.existing(position);
    let limit = symbols.existing(limit);
    // On this edge limit - position is at least eight, and the XOR is
    // nonzero. The first unequal byte is therefore strictly below limit.
    let inside = Fact {
        cond: IntCC::UnsignedLessThan,
        left: symbols.address(f, cfg, position, 0)?.plus_byte_offset(),
        right: symbols.address(f, cfg, limit, 0)?,
    };
    Some((
        Walk {
            f,
            cfg,
            dom,
            origin: branch,
            bytes,
            symbols,
            facts: vec![inside],
            word_difference: Some(difference),
            byte_offset: None,
        },
        blocks[0],
    ))
}

/// Reverse the whole words only when the two forwarded bytes have one joint
/// observation. The first unequal byte sets the unsigned order in either form.
fn sole_order(
    f: &Function,
    dom: &DominatorTree,
    plan: &Plan,
    repairs: &[(Value, Symbol)],
) -> Option<Inst> {
    let mut operands = Vec::new();
    for (&param, &symbol) in f
        .dfg
        .block_params(plan.join)
        .iter()
        .zip(&plan.args)
        .chain(repairs.iter().map(|(value, symbol)| (value, symbol)))
    {
        if let Some(word) = plan.symbols.byte_word(symbol) {
            operands.push((f.dfg.resolve_aliases(param), word));
        } else if plan.symbols.depends_on_word_byte(symbol) {
            return None;
        }
    }
    let &[(a, word_a), (b, word_b)] = operands.as_slice() else {
        return None;
    };
    if a == b || word_a == word_b {
        return None;
    }
    let mut observed = None;
    for block in f
        .layout
        .blocks()
        .filter(|&b| dom.block_dominates(plan.join, b))
    {
        for inst in f.layout.block_insts(block) {
            if !f.dfg.inst_values(inst).any(|value| {
                let value = f.dfg.resolve_aliases(value);
                value == a || value == b
            }) {
                continue;
            }
            let Data::IntCompare {
                cond: IntCC::UnsignedLessThan | IntCC::UnsignedGreaterThan,
                args,
                ..
            } = f.dfg.insts[inst]
            else {
                return None;
            };
            let args = args.map(|arg| f.dfg.resolve_aliases(arg));
            if observed.is_some() || (args != [a, b] && args != [b, a]) {
                return None;
            }
            observed = Some(inst);
        }
    }
    observed
}

// Compute the pure order on each incoming edge before the values join.
// Each argument is the exact value that the target comparison would observe
// on that edge. Only the Boolean result remains live across the join.
fn forward_order_result(context: &mut Context, join: Block, comparison: Inst) -> Option<()> {
    context.flowgraph();
    let f = &context.func;
    let Data::IntCompare { cond, args, .. } = f.dfg.insts[comparison] else {
        return None;
    };
    let [left, right] = args.map(|value| f.dfg.resolve_aliases(value));
    let [left_index, right_index] = [left, right].map(|value| match f.dfg.value_def(value) {
        ValueDef::Param(block, index) if block == join => Some(index),
        _ => None,
    });
    let (left_index, right_index) = (left_index?, right_index?);
    if left_index == right_index || !context.domtree.dominates(join, comparison, &f.layout) {
        return None;
    }
    let has_other_use = f.layout.blocks().any(|block| {
        f.layout.block_insts(block).any(|inst| {
            inst != comparison
                && f.dfg.inst_values(inst).any(|value| {
                    let value = f.dfg.resolve_aliases(value);
                    value == left || value == right
                })
        })
    });
    if has_other_use {
        return None;
    }
    let predecessors = context.cfg.pred_iter(join).collect::<Vec<_>>();
    if !(2..=8).contains(&predecessors.len()) {
        return None;
    }
    let inputs = predecessors
        .iter()
        .map(|pred| {
            if context.domtree.block_dominates(join, pred.block) {
                return None;
            }
            let Data::Jump { destination, .. } = f.dfg.insts[pred.inst] else {
                return None;
            };
            let values = &f.dfg.value_lists;
            if destination.block(values) != join {
                return None;
            }
            let a = destination.args(values).nth(left_index)?.as_value()?;
            let b = destination.args(values).nth(right_index)?.as_value()?;
            Some((pred.inst, a, b))
        })
        .collect::<Option<Vec<_>>>()?;

    let f = &mut context.func;
    let original = f.dfg.first_result(comparison);
    let output_type = f.dfg.value_type(original);
    let result = f.dfg.append_block_param(join, output_type);
    let mut removed = [left_index, right_index];
    removed.sort_unstable();
    for (jump, a, b) in inputs {
        let mut cursor = FuncCursor::new(f).at_inst(jump);
        cursor.use_srcloc(comparison);
        let edge_result = cursor.ins().icmp(cond, a, b);
        let dfg = &mut cursor.func.dfg;
        let Data::Jump { destination, .. } = &mut dfg.insts[jump] else {
            unreachable!("checked incoming jump");
        };
        for &index in removed.iter().rev() {
            destination.remove(index, &mut dfg.value_lists);
        }
        destination.append_argument(edge_result, &mut dfg.value_lists);
    }
    f.dfg.detach_inst_results(comparison);
    f.dfg.change_to_alias(original, result);
    f.layout.remove_inst(comparison);
    f.dfg.remove_block_param(left);
    f.dfg.remove_block_param(right);
    f.dfg.resolve_all_aliases();
    Some(())
}

pub(in crate::native_artifact) fn fold_word_order(
    context: &mut Context,
    witnesses: &[AdmittedWordMismatch],
    views: &BTreeMap<usize, (Value, Value)>,
    reverse_words: bool,
) -> (usize, usize) {
    let trace = std::env::var_os("MECH_INSPECT_WORD_MISMATCH_ORDER").is_some();
    let forward_order = std::env::var_os("MECH_COMPACT_FORWARD_WORD_ORDER_RESULT").is_some();
    let (mut forwarded, mut reversed) = (0, 0);
    for witness in witnesses {
        context.flowgraph();
        let Some((walk, destination)) = walk(context, witness, views) else {
            if trace {
                eprintln!(
                    "word mismatch {} lost its admitted operands",
                    witness.branch
                );
            }
            continue;
        };
        let Some(mut plan) = walk.trace(0, destination) else {
            if trace {
                eprintln!(
                    "word mismatch {} could not replay the byte path",
                    witness.branch
                );
            }
            continue;
        };
        let Some(repairs) = repairs(&context.func, &context.domtree, &plan, true) else {
            if trace {
                eprintln!(
                    "word mismatch {} needs unproved SSA repairs",
                    witness.branch
                );
            }
            continue;
        };
        let order = reverse_words
            .then(|| sole_order(&context.func, &context.domtree, &plan, &repairs))
            .flatten();
        plan.reverse_words = order.is_some();
        reversed += usize::from(plan.reverse_words);
        let join = plan.join;
        install(context, plan, repairs);
        if forward_order
            && let Some(order) = order
            && forward_order_result(context, join, order).is_some()
            && trace
        {
            eprintln!(
                "word mismatch {} forwarded the order result",
                witness.branch
            );
        }
        forwarded += 1;
    }
    if forwarded != 0 {
        context.flowgraph();
    }
    (forwarded, reversed)
}

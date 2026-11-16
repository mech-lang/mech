//! Reuse one decision for consecutive scalar selections on the same words.
use super::window_bounds::{comparison, sole_predecessor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, types,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_HOPS: usize = 32;

#[derive(Clone, Copy)]
struct Exit {
    jump: Inst,
    join: Block,
    value: Value,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Test {
    condition: IntCC,
    left: Value,
    right: Value,
}

fn branch(f: &Function, inst: Inst) -> Option<(Inst, Test, [BlockCall; 2])> {
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
        return None;
    };
    if blocks[0].block(&f.dfg.value_lists) == blocks[1].block(&f.dfg.value_lists) {
        return None;
    }
    let cmp = comparison(f, arg)?;
    let Data::IntCompare {
        cond,
        args: [left, right],
        ..
    } = f.dfg.insts[cmp]
    else {
        return None;
    };
    let [left, right] = [left, right].map(|value| f.dfg.resolve_aliases(value));
    (f.dfg.value_type(left) == types::I64).then_some((
        cmp,
        Test {
            condition: cond,
            left,
            right,
        },
        blocks,
    ))
}

fn unique(
    f: &Function,
    cfg: &ControlFlowGraph,
    block: Block,
    pred: Inst,
    visited: &mut BTreeSet<Block>,
) -> Option<()> {
    (Some(block) != f.layout.entry_block()
        && f.dfg.block_params(block).is_empty()
        && sole_predecessor(cfg, block)?.1 == pred
        && visited.insert(block))
    .then_some(())
}

fn one_arg(f: &Function, call: BlockCall) -> Option<Value> {
    let mut args = call.args(&f.dfg.value_lists);
    let value = args.next()?.as_value()?;
    args.next().is_none().then(|| f.dfg.resolve_aliases(value))
}

fn forwards(
    f: &Function,
    cfg: &ControlFlowGraph,
    mut pred: Inst,
    mut call: BlockCall,
    visited: &mut BTreeSet<Block>,
) -> Option<Exit> {
    for _ in 0..MAX_HOPS {
        if call.len(&f.dfg.value_lists) != 0 {
            return Some(Exit {
                jump: pred,
                join: call.block(&f.dfg.value_lists),
                value: one_arg(f, call)?,
            });
        }
        let block = call.block(&f.dfg.value_lists);
        unique(f, cfg, block, pred, visited)?;
        pred = f.layout.first_inst(block)?;
        if f.layout.next_inst(pred).is_some() {
            return None;
        }
        let Data::Jump { destination, .. } = f.dfg.insts[pred] else {
            return None;
        };
        call = destination;
    }
    None
}

fn joined(f: &Function, cfg: &ControlFlowGraph, a: Exit, b: Exit) -> Option<Value> {
    let [word] = *f.dfg.block_params(a.join) else {
        return None;
    };
    (a.jump != b.jump
        && a.join == b.join
        && Some(a.join) != f.layout.entry_block()
        && f.dfg.value_type(word) == types::I64
        && cfg
            .pred_iter(a.join)
            .map(|p| p.inst)
            .collect::<BTreeSet<_>>()
            == BTreeSet::from([a.jump, b.jump]))
    .then_some(word)
}

fn through_test(
    f: &Function,
    cfg: &ControlFlowGraph,
    mut block: Block,
    visited: &mut BTreeSet<Block>,
) -> Option<Inst> {
    for _ in 0..MAX_HOPS {
        let last = f.layout.last_inst(block)?;
        if !visited.insert(block)
            || f.layout
                .block_insts(block)
                .take_while(|&i| i != last)
                .any(|i| {
                    !matches!(
                        f.dfg.insts[i].opcode(),
                        Opcode::Icmp | Opcode::IcmpImm | Opcode::Uextend | Opcode::BandImm
                    )
                })
        {
            return None;
        }
        match f.dfg.insts[last] {
            Data::Brif { .. } => return Some(last),
            Data::Jump { destination, .. } if destination.len(&f.dfg.value_lists) == 0 => {
                block = destination.block(&f.dfg.value_lists);
                if !f.dfg.block_params(block).is_empty() || sole_predecessor(cfg, block)?.1 != last
                {
                    return None;
                }
            }
            _ => return None,
        }
    }
    None
}

struct Rewrite {
    selected: Exit,
    unselected: Exit,
    checked_arm: Block,
    minimum_test: Inst,
    minimum_operand: Exit,
    minimum_exit: Exit,
    unchanged: Value,
    first_join_end: Inst,
    affected: BTreeSet<Block>,
}

fn available(f: &Function, dom: &DominatorTree, value: Value, at: Inst) -> bool {
    use cranelift_codegen::ir::ValueDef;
    match f.dfg.value_def(value) {
        ValueDef::Param(block, _) => dom.dominates(block, at, &f.layout),
        ValueDef::Result(inst, _) => inst != at && dom.dominates(inst, at, &f.layout),
        ValueDef::Union(..) => false,
    }
}

fn plan(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    uses: &BTreeMap<Value, BTreeSet<Inst>>,
    first: Inst,
) -> Option<Rewrite> {
    let (_, predicate, [yes, no]) = branch(f, first)?;
    if predicate.condition != IntCC::UnsignedLessThan
        || yes.len(&f.dfg.value_lists) != 0
        || no.len(&f.dfg.value_lists) != 0
    {
        return None;
    }
    let mut arms = BTreeSet::new();
    let selected = forwards(f, cfg, first, yes, &mut arms)?;
    let unselected = forwards(f, cfg, first, no, &mut arms)?;
    let joined_value = joined(f, cfg, selected, unselected)?;
    let first_join_end = f.layout.last_inst(selected.join)?;
    if !f.layout.block_contains_exactly_one_inst(selected.join) || selected.value != predicate.left
    {
        return None;
    }
    let mut repeated_path = BTreeSet::new();
    let second = through_test(f, cfg, selected.join, &mut repeated_path)?;
    let (_, repeated, [check, keep]) = branch(f, second)?;
    if repeated != predicate
        || check.len(&f.dfg.value_lists) != 0
        || keep.len(&f.dfg.value_lists) != 0
    {
        return None;
    }
    // All definitions from the bypassed predicate must die with that path.
    for &block in &repeated_path {
        for value in f
            .layout
            .block_insts(block)
            .flat_map(|i| f.dfg.inst_results(i))
        {
            if uses.get(value).is_some_and(|users| {
                users.iter().any(|&i| {
                    f.layout
                        .inst_block(i)
                        .is_none_or(|b| !repeated_path.contains(&b))
                })
            }) {
                return None;
            }
        }
    }
    let checked_arm = check.block(&f.dfg.value_lists);
    unique(f, cfg, checked_arm, second, &mut arms)?;
    arms.remove(&checked_arm);
    let mut minimum_path = BTreeSet::new();
    let condition = through_test(f, cfg, checked_arm, &mut minimum_path)?;
    let (minimum_test, minimum, [low, high]) = branch(f, condition)?;
    if minimum.condition != IntCC::UnsignedLessThan
        || minimum.right != joined_value
        || low.len(&f.dfg.value_lists) != 0
        || high.len(&f.dfg.value_lists) != 0
        || !f
            .layout
            .inst_block(minimum_test)
            .is_some_and(|b| minimum_path.contains(&b))
    {
        return None;
    }
    let left = forwards(f, cfg, condition, low, &mut minimum_path)?;
    let right = forwards(f, cfg, condition, high, &mut minimum_path)?;
    let min = joined(f, cfg, left, right)?;
    if (left.value, right.value) != (minimum.left, joined_value) {
        return None;
    }
    // Moving the typed merge later only replaces these uses on the near arm.
    if uses.get(&joined_value)?.iter().any(|&i| {
        f.layout
            .inst_block(i)
            .is_some_and(|b| minimum_path.contains(&b))
            && i != minimum_test
            && i != right.jump
    }) {
        return None;
    }
    let after_min = f.layout.first_inst(left.join)?;
    if f.layout.next_inst(after_min).is_some() || !minimum_path.insert(left.join) {
        return None;
    }
    let Data::Jump { destination, .. } = f.dfg.insts[after_min] else {
        return None;
    };
    let minimum_exit = forwards(f, cfg, after_min, destination, &mut minimum_path)?;
    let retained = forwards(f, cfg, second, keep, &mut arms)?;
    joined(f, cfg, minimum_exit, retained)?;
    if minimum_exit.value != min
        || retained.value != minimum.left
        || !available(f, dom, retained.value, unselected.jump)
        || !arms.is_disjoint(&minimum_path)
        || !repeated_path.is_disjoint(&arms)
        || !repeated_path.is_disjoint(&minimum_path)
    {
        return None;
    }
    // A loop or alternate entry could refresh the values being threaded.
    let source = f.layout.inst_block(first)?;
    let mut affected = arms;
    affected.extend(repeated_path);
    affected.extend(minimum_path);
    if affected.contains(&source) || !affected.insert(minimum_exit.join) {
        return None;
    }
    affected.insert(source);
    Some(Rewrite {
        selected,
        unselected,
        checked_arm,
        minimum_test,
        minimum_operand: right,
        minimum_exit,
        unchanged: retained.value,
        first_join_end,
        affected,
    })
}

pub(in crate::native_artifact) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    let mut uses = BTreeMap::<Value, BTreeSet<Inst>>::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            for value in f.dfg.inst_values(inst) {
                uses.entry(f.dfg.resolve_aliases(value))
                    .or_default()
                    .insert(inst);
            }
        }
    }
    let mut affected = BTreeSet::new();
    let mut edits = Vec::new();
    for block in f.layout.blocks().filter(|&b| dom.is_reachable(b)) {
        let Some(first) = f.layout.last_inst(block) else {
            continue;
        };
        let Some(rewrite) = plan(f, cfg, dom, &uses, first) else {
            continue;
        };
        if rewrite.affected.is_disjoint(&affected) {
            affected.extend(rewrite.affected.iter().copied());
            edits.push(rewrite);
        }
    }
    for edit in &edits {
        let join = edit.selected.join;
        let carried = f.dfg.append_block_param(join, types::I64);
        f.dfg
            .replace(edit.first_join_end)
            .jump(edit.minimum_exit.join, &[carried.into()]);
        f.dfg
            .replace(edit.selected.jump)
            .jump(edit.checked_arm, &[]);
        f.dfg
            .replace(edit.unselected.jump)
            .jump(join, &[edit.unselected.value.into(), edit.unchanged.into()]);
        f.dfg.replace(edit.minimum_test).icmp(
            IntCC::UnsignedLessThan,
            edit.unchanged,
            edit.selected.value,
        );
        f.dfg
            .replace(edit.minimum_operand.jump)
            .jump(edit.minimum_operand.join, &[edit.selected.value.into()]);
        f.dfg.replace(edit.minimum_exit.jump).jump(
            join,
            &[edit.selected.value.into(), edit.minimum_exit.value.into()],
        );
    }
    edits.len()
}

#[cfg(all(test, feature = "jit", target_arch = "aarch64"))]
mod tests;

//! Reconstruct current canonical Boolean words at a merge from exact selected
//! incoming edges. Every source home store, effect, rejection and fee remains.
//! An older predicate snapshot is an SSA atom: only zero-extension aliases are
//! followed when rematerializing, never comparisons under later phi facts.
use super::word_ranges::{Facts, Range, condition, literal};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, BlockArg, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, Opcode,
    Value, ValueDef,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_BLOCKS: usize = 4096;
const MAX_FACTS: usize = 256;
const MAX_PARAMETERS: usize = 128;
const MAX_OPERANDS: usize = 4096;
const MAX_PROOF_STEPS: usize = 262_144;

struct Budget(usize);
impl Budget {
    fn spend(&mut self, steps: usize) -> Option<()> {
        self.0 = self.0.checked_sub(steps)?;
        Some(())
    }
}

#[derive(Default)]
pub(in crate::native_artifact) struct Report {
    pub constant_operands: usize,
    pub merge_parameters: usize,
}

type Known = BTreeMap<Value, Range>;

fn owner(f: &Function, value: Value) -> Option<Block> {
    match f.dfg.value_def(value) {
        ValueDef::Param(block, _) => Some(block),
        ValueDef::Result(inst, _) => f.layout.inst_block(inst),
        _ => None,
    }
}

fn canonical(f: &Function, value: Value, boolean_phis: &Known) -> bool {
    Facts::bound(f, boolean_phis).canonical_boolean(value)
}

fn assume(
    f: &Function,
    value: Value,
    truth: bool,
    known: &mut Known,
    base: &Known,
    budget: &mut Budget,
) -> bool {
    if !condition::assume_bounded(f, value, truth, known, &mut budget.0) {
        return false;
    }
    let value = f.dfg.resolve_aliases(value);
    if let ValueDef::Result(inst, 0) = f.dfg.value_def(value)
        && let Data::BinaryImm64 {
            opcode: Opcode::BandImm,
            arg,
            imm,
        } = f.dfg.insts[inst]
        && imm.bits() == 1
        // A low bit does not determine an unrestricted word's other bits.
        && canonical(f, arg, base)
    {
        return condition::assume_bounded(f, arg, truth, known, &mut budget.0);
    }
    true
}

fn incoming(f: &Function, cfg: &ControlFlowGraph, block: Block) -> Option<(Value, bool)> {
    if Some(block) == f.layout.entry_block() {
        return None;
    }
    let mut preds = cfg.pred_iter(block);
    let pred = preds.next()?;
    if preds.next().is_some() {
        return None;
    }
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[pred.inst] else {
        return None;
    };
    let yes = blocks[0].block(&f.dfg.value_lists);
    let no = blocks[1].block(&f.dfg.value_lists);
    (yes != no).then_some((arg, block == yes))
}

fn selected(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    base: &Known,
    budget: &mut Budget,
) -> Option<BTreeMap<Block, Known>> {
    let entry = f.layout.entry_block()?;
    let mut pending = vec![(entry, base.clone())];
    let mut facts = BTreeMap::new();
    while let Some((block, mut known)) = pending.pop() {
        budget.spend(known.len() + 1)?;
        if let Some((value, truth)) = incoming(f, cfg, block) {
            let mut narrowed = known.clone();
            if assume(f, value, truth, &mut narrowed, base, budget) {
                known = narrowed;
            }
            if budget.0 == 0 {
                return None;
            }
        }
        if known.len() > MAX_FACTS || facts.len() >= MAX_BLOCKS {
            return None;
        }
        for child in dom.children(block) {
            budget.spend(known.len() + 1)?;
            pending.push((child, known.clone()));
        }
        facts.insert(block, known);
    }
    Some(facts)
}

// Only an exact current SSA fact and its total zero-extension aliases qualify.
// In particular, do not re-evaluate an older comparison using a loop phi's
// newly selected value. Loads, arithmetic, bitcasts and phi inputs are opaque.
fn singleton(f: &Function, mut value: Value, known: &Known) -> Option<u64> {
    for _ in 0..16 {
        value = f.dfg.resolve_aliases(value);
        if let Some(&Range(n, m)) = known.get(&value)
            && n == m
            && n <= 1
        {
            return Some(n);
        }
        let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
            return None;
        };
        match f.dfg.insts[inst] {
            Data::Unary {
                opcode: Opcode::Uextend,
                arg,
            } => value = arg,
            _ => return None,
        }
    }
    None
}

struct Merge {
    block: Block,
    value: Value,
    edges: Vec<(Inst, usize, u64)>,
}

fn merges(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    base: &Known,
    facts: &BTreeMap<Block, Known>,
    uses: &BTreeMap<Value, BTreeSet<Block>>,
    budget: &mut Budget,
) -> Option<Vec<Merge>> {
    let mut plans = Vec::new();
    for block in f.layout.blocks() {
        budget.spend(1)?;
        if !dom.is_reachable(block) || Some(block) == f.layout.entry_block() {
            continue;
        }
        let preds = cfg
            .pred_iter(block)
            .map(|p| p.inst)
            .collect::<BTreeSet<_>>();
        if preds.len() < 2 {
            continue;
        }
        let mut edges = Vec::new();
        for inst in preds {
            budget.spend(1)?;
            let source = f.layout.inst_block(inst).expect("checked edge owner");
            let Some(source_facts) = facts.get(&source) else {
                edges.clear();
                break;
            };
            budget.spend(source_facts.len() + 1)?;
            let destinations =
                f.dfg.insts[inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables);
            let mut calls = destinations
                .iter()
                .enumerate()
                .filter(|(_, call)| call.block(&f.dfg.value_lists) == block);
            let Some((ordinal, _)) = calls.next() else {
                continue;
            };
            // Decline two different edge occurrences from one terminator.
            if calls.next().is_some() {
                edges.clear();
                break;
            }
            let mut known = source_facts.clone();
            if let Data::Brif { arg, .. } = f.dfg.insts[inst]
                && !assume(f, arg, ordinal == 0, &mut known, base, budget)
            {
                if budget.0 == 0 {
                    return None;
                }
                edges.clear();
                break;
            }
            edges.push((inst, ordinal, known));
        }
        let Some((_, _, first)) = edges.first() else {
            continue;
        };
        for (&value, &Range(n, m)) in first {
            budget.spend(1)?;
            if n != m || n > 1 || literal(f, value).is_some() || !canonical(f, value, base) {
                continue;
            }
            let Some(definition) = owner(f, value) else {
                continue;
            };
            if definition == block || !dom.block_dominates(definition, block) {
                continue;
            }
            budget.spend(uses.get(&value).map_or(0, BTreeSet::len))?;
            if !uses
                .get(&value)
                .is_some_and(|blocks| blocks.iter().any(|&user| dom.block_dominates(block, user)))
            {
                continue;
            }
            budget.spend(edges.len())?;
            let values = edges
                .iter()
                .map(|(_, _, known)| singleton(f, value, known))
                .collect::<Option<Vec<_>>>();
            let Some(values) = values else { continue };
            // Reconstruct a real selected merge, not a trivially fixed word.
            if !values.contains(&0) || !values.contains(&1) {
                continue;
            }
            plans.push(Merge {
                block,
                value,
                edges: edges
                    .iter()
                    .zip(values)
                    .map(|(&(inst, ordinal, _), n)| (inst, ordinal, n))
                    .collect(),
            });
            if plans.len() >= MAX_PARAMETERS {
                return Some(plans);
            }
        }
    }
    Some(plans)
}

fn arguments(f: &Function, inst: Inst, budget: &mut Budget) -> Option<Vec<Value>> {
    budget.spend(f.dfg.inst_args(inst).len() + 1)?;
    let mut values = f.dfg.inst_args(inst).to_vec();
    for call in f.dfg.insts[inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables) {
        for arg in call.args(&f.dfg.value_lists) {
            budget.spend(1)?;
            values.push(arg.as_value()?);
        }
    }
    values.sort_unstable();
    values.dedup();
    Some(values)
}

fn rewrite_arguments(f: &mut Function, inst: Inst, replacements: &BTreeMap<Value, Value>) {
    for value in f.dfg.inst_args_mut(inst) {
        if let Some(&new) = replacements.get(value) {
            *value = new;
        }
    }
    let dfg = &mut f.dfg;
    for call in
        dfg.insts[inst].branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
    {
        let args = call
            .args(&dfg.value_lists)
            .map(|arg| {
                let old = arg.as_value().expect("checked ordinary edge");
                BlockArg::from(replacements.get(&old).copied().unwrap_or(old))
            })
            .collect::<Vec<_>>();
        *call = BlockCall::new(call.block(&dfg.value_lists), args, &mut dfg.value_lists);
    }
}

pub(super) fn optimize(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> Report {
    let mut report = Report::default();
    let mut budget = Budget(MAX_PROOF_STEPS);
    for (n, block) in f.layout.blocks().enumerate() {
        if n >= MAX_BLOCKS {
            return report;
        }
        for inst in f.layout.block_insts(block) {
            if f.dfg.insts[inst].exception_table().is_some()
                || arguments(f, inst, &mut budget).is_none()
            {
                return report;
            }
        }
    }
    let base = super::boolean_words::derive(f, cfg);
    let Some(facts) = selected(f, cfg, dom, &base, &mut budget) else {
        return report;
    };
    let mut uses = BTreeMap::<Value, BTreeSet<Block>>::new();
    let mut constants = Vec::new();
    for block in f.layout.blocks() {
        let Some(known) = facts.get(&block) else {
            continue;
        };
        for inst in f.layout.block_insts(block) {
            let Some(args) = arguments(f, inst, &mut budget) else {
                return report;
            };
            for value in args {
                uses.entry(value).or_default().insert(block);
                if literal(f, value).is_none()
                    && canonical(f, value, &base)
                    && let Some(n) = singleton(f, value, known)
                {
                    constants.push((inst, value, n));
                    if constants.len() >= MAX_OPERANDS {
                        return report;
                    }
                }
            }
        }
    }
    let Some(plans) = merges(f, cfg, dom, &base, &facts, &uses, &mut budget) else {
        return report;
    };
    // All proof decisions above refer to the original immutable CFG/SSA graph.
    for (inst, value, n) in constants {
        let ty = f.dfg.value_type(value);
        let constant = FuncCursor::new(f).at_inst(inst).ins().iconst(ty, n as i64);
        rewrite_arguments(f, inst, &BTreeMap::from([(value, constant)]));
        report.constant_operands += 1;
    }
    let mut repairs = BTreeMap::<Block, BTreeMap<Value, Value>>::new();
    for plan in plans {
        let ty = f.dfg.value_type(plan.value);
        let param = f.dfg.append_block_param(plan.block, ty);
        repairs
            .entry(plan.block)
            .or_default()
            .insert(plan.value, param);
        for (inst, ordinal, n) in plan.edges {
            let constant = FuncCursor::new(f).at_inst(inst).ins().iconst(ty, n as i64);
            let dfg = &mut f.dfg;
            let call = &mut dfg.insts[inst]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)[ordinal];
            let mut args = call.args(&dfg.value_lists).collect::<Vec<_>>();
            args.push(constant.into());
            *call = BlockCall::new(plan.block, args, &mut dfg.value_lists);
        }
        report.merge_parameters += 1;
    }
    let Some(entry) = f.layout.entry_block() else {
        return report;
    };
    enum Step {
        Enter(Block),
        Leave(Vec<(Value, Option<Value>)>),
    }
    let mut pending = vec![Step::Enter(entry)];
    let mut active = BTreeMap::new();
    while let Some(step) = pending.pop() {
        match step {
            Step::Leave(undo) => {
                for (old, prior) in undo.into_iter().rev() {
                    if let Some(value) = prior {
                        active.insert(old, value);
                    } else {
                        active.remove(&old);
                    }
                }
            }
            Step::Enter(block) => {
                let mut undo = Vec::new();
                if let Some(map) = repairs.get(&block) {
                    for (&old, &new) in map {
                        undo.push((old, active.insert(old, new)));
                    }
                }
                let instructions = f.layout.block_insts(block).collect::<Vec<_>>();
                for inst in instructions {
                    rewrite_arguments(f, inst, &active);
                }
                pending.push(Step::Leave(undo));
                pending.extend(dom.children(block).map(Step::Enter));
            }
        }
    }
    report
}

#[cfg(test)]
mod tests;

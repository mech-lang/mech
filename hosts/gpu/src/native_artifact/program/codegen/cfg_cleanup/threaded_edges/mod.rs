//! Thread proved current loop edges through pure dispatch, without cloning.
//! Dominated live-outs gain explicit phis; shared merges conservatively refuse.
mod route;
mod values;
mod visits;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, BlockArg, BlockCall, Function, InstBuilder, InstructionData as Data, Value,
};
use std::collections::{BTreeMap, BTreeSet};
use values::Word;

#[derive(Default, serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub candidates: usize,
    pub threaded_edges: usize,
    pub selected_conditions: usize,
    pub repair_parameters: usize,
}

fn rewrite(
    f: &mut Function,
    dom: &DominatorTree,
    repairs: &BTreeMap<Block, BTreeMap<Value, Value>>,
) {
    enum Step {
        Enter(Block),
        Leave(Vec<(Value, Option<Value>)>),
    }
    let Some(entry) = f.layout.entry_block() else {
        return;
    };
    let mut steps = vec![Step::Enter(entry)];
    let mut active = BTreeMap::new();
    while let Some(step) = steps.pop() {
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
                    for value in f.dfg.inst_args_mut(inst) {
                        if let Some(&new) = active.get(value) {
                            *value = new;
                        }
                    }
                    let dfg = &mut f.dfg;
                    for call in dfg.insts[inst]
                        .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
                    {
                        let args = call
                            .args(&dfg.value_lists)
                            .map(|arg| {
                                let old = arg.as_value().expect("ordinary checked edge");
                                active.get(&old).copied().unwrap_or(old).into()
                            })
                            .collect::<Vec<_>>();
                        *call = BlockCall::new(
                            call.block(&dfg.value_lists),
                            args,
                            &mut dfg.value_lists,
                        );
                    }
                }
                steps.push(Step::Leave(undo));
                steps.extend(dom.children(block).map(Step::Enter));
            }
        }
    }
}

pub(super) fn optimize(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> Report {
    let mut report = Report::default();
    // Unknown/exceptional edge values keep the original entry/landing contract.
    if f.layout.blocks().any(|b| {
        f.layout.block_insts(b).any(|i| {
            f.dfg.insts[i].exception_table().is_some()
                || f.dfg.insts[i]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                    .iter()
                    .any(|call| {
                        call.args(&f.dfg.value_lists)
                            .any(|a| a.as_value().is_none())
                    })
        })
    }) {
        return report;
    }
    let known = super::boolean_words::derive(f, cfg);
    let mut edges = Vec::new();
    for block in f.layout.blocks() {
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        if !matches!(f.dfg.insts[inst], Data::Jump { .. } | Data::Brif { .. }) {
            continue;
        }
        for (ordinal, call) in f.dfg.insts[inst]
            .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            .iter()
            .enumerate()
        {
            if f.dfg
                .block_params(call.block(&f.dfg.value_lists))
                .is_empty()
            {
                continue;
            }
            edges.push(route::Edge {
                inst,
                ordinal,
                target: call.block(&f.dfg.value_lists),
                args: call
                    .args(&f.dfg.value_lists)
                    .map(|a| a.as_value().unwrap())
                    .collect(),
            });
        }
    }
    report.candidates = edges.len();
    // Every route observes this same immutable CFG. Reuse only visitation
    // storage; each live-out traversal still starts with an empty visited set.
    let mut visits = visits::Visits::new(f.dfg.num_blocks());
    let plans = edges
        .into_iter()
        .filter_map(|edge| route::find(f, dom, edge, known.clone(), &mut visits))
        .collect::<Vec<_>>();
    let targets = plans.iter().map(|p| p.target).collect::<BTreeSet<_>>();
    // Preserve the original dominance of every new transport phi. No other
    // shortcut may bypass its destination, including nested dispatch plans.
    let plans = plans
        .into_iter()
        .filter(|p| p.skipped.is_disjoint(&targets))
        .collect::<Vec<_>>();
    let mut groups = BTreeMap::<Block, BTreeSet<Value>>::new();
    for plan in &plans {
        groups
            .entry(plan.target)
            .or_default()
            .extend(plan.repairs.keys().copied());
    }
    let mut replacements = BTreeMap::new();
    for (&target, values) in &groups {
        let incoming = cfg
            .pred_iter(target)
            .map(|p| p.inst)
            .collect::<BTreeSet<_>>();
        let mut new = BTreeMap::new();
        for &value in values {
            let ty = f.dfg.value_type(value);
            let param = f.dfg.append_block_param(target, ty);
            new.insert(value, param);
            report.repair_parameters += 1;
        }
        for inst in incoming {
            let dfg = &mut f.dfg;
            for call in dfg.insts[inst]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
            {
                if call.block(&dfg.value_lists) == target {
                    let mut args = call.args(&dfg.value_lists).collect::<Vec<_>>();
                    args.extend(values.iter().copied().map(BlockArg::from));
                    *call = BlockCall::new(target, args, &mut dfg.value_lists);
                }
            }
        }
        replacements.insert(target, new);
    }
    for plan in plans {
        let location = f.srcloc(plan.edge.inst);
        let mut words = plan.args;
        words.extend(groups[&plan.target].iter().map(|&v| {
            plan.repairs
                .get(&v)
                .copied()
                .unwrap_or(Word::Value(v, None))
        }));
        let mut cursor = FuncCursor::new(f).at_inst(plan.edge.inst);
        cursor.set_srcloc(location);
        let args = words
            .into_iter()
            .map(|word| match word {
                Word::Value(v, _) => v,
                Word::Constant(ty, n) => cursor.ins().iconst(ty, n as i64),
            })
            .map(Into::into)
            .collect::<Vec<_>>();
        let dfg = &mut cursor.func.dfg;
        let call = dfg.insts[plan.edge.inst]
            .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
            .get_mut(plan.edge.ordinal)
            .expect("saved ordinary edge");
        assert_eq!(call.block(&dfg.value_lists), plan.edge.target);
        *call = BlockCall::new(plan.target, args, &mut dfg.value_lists);
        report.threaded_edges += 1;
        report.selected_conditions += plan.conditions;
    }
    rewrite(f, dom, &replacements);
    report
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

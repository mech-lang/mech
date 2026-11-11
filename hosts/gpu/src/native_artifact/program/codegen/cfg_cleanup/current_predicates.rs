//! Refine current immutable SSA predicates on a unique selected edge. This
//! changes control only: no new phis, permissions or memory facts are introduced.
use super::word_ranges::{Facts, Range, condition};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{Block, Function, InstBuilder, InstructionData as Data, Value};
use std::collections::BTreeMap;

pub(super) mod relations;
pub(super) mod scalar_spans;

#[derive(Default, serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub removed_branches: usize,
    pub refined_edges: usize,
    pub boolean_phi_values: usize,
}

fn rollback(known: &mut BTreeMap<Value, Range>, undo: Vec<(Value, Option<Range>)>) {
    for (value, prior) in undo.into_iter().rev() {
        if let Some(prior) = prior {
            known.insert(value, prior);
        } else {
            known.remove(&value);
        }
    }
}

fn incoming(f: &Function, cfg: &ControlFlowGraph, block: Block) -> Option<(Value, bool)> {
    if Some(block) == f.layout.entry_block() {
        return None;
    }
    let mut predecessors = cfg.pred_iter(block);
    let pred = predecessors.next()?;
    if predecessors.next().is_some() {
        return None;
    }
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[pred.inst] else {
        return None;
    };
    let yes = blocks[0].block(&f.dfg.value_lists);
    let no = blocks[1].block(&f.dfg.value_lists);
    (yes != no).then_some((arg, block == yes))
}

pub(super) fn fold(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> Report {
    let mut report = Report::default();
    if f.layout.blocks().any(|b| {
        f.layout
            .block_insts(b)
            .any(|i| f.dfg.insts[i].exception_table().is_some())
    }) {
        return report;
    }
    let Some(entry) = f.layout.entry_block() else {
        return report;
    };
    enum Step {
        Enter(Block),
        Leave(Vec<(Value, Option<Range>)>),
    }
    let mut known = super::boolean_words::derive(f, cfg);
    report.boolean_phi_values = known.len();
    let mut steps = vec![Step::Enter(entry)];
    let mut changes = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Leave(undo) => rollback(&mut known, undo),
            Step::Enter(block) => {
                let mut undo = Vec::new();
                if let Some((value, truth)) = incoming(f, cfg, block) {
                    if condition::assume_with_undo(f, value, truth, &mut known, &mut undo) {
                        report.refined_edges += 1;
                    } else {
                        // Contradictory or unsupported facts do not reject the
                        // program or contaminate another path.
                        rollback(&mut known, core::mem::take(&mut undo));
                    }
                }
                if let Some(inst) = f.layout.last_inst(block) {
                    if let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] {
                        if let Some(truth) = Facts::bound(f, &known).condition(arg) {
                            let target = blocks[usize::from(!truth)];
                            changes.push((
                                inst,
                                target.block(&f.dfg.value_lists),
                                target.args(&f.dfg.value_lists).collect::<Vec<_>>(),
                            ));
                        }
                    }
                }
                steps.push(Step::Leave(undo));
                steps.extend(dom.children(block).map(Step::Enter));
            }
        }
    }
    report.removed_branches = changes.len();
    for (inst, target, args) in changes {
        f.dfg.replace(inst).jump(target, &args);
    }
    report
}

#[cfg(test)]
mod tests;

//! Must-facts for exact Bool homes in the current source traversal.
//! No equivalence between distinct predicates is inferred. Every definition
//! kills the old fact, including projections and loop-carrier updates.
use super::flow::Flow;
use mech_engine::native_artifact::{
    NativeIterationTerminator as Term, NativeMatchTerminator as Branch,
    NativeProgramArtifactPlan as Plan,
};
use std::collections::{BTreeMap, VecDeque};

pub(super) struct Guards {
    known: Vec<BTreeMap<u32, bool>>,
    branches: Vec<Option<(u32, usize, usize)>>,
}
impl Guards {
    pub fn derive(plan: &Plan, flow: &Flow) -> Self {
        let facts = plan.facts();
        let branches = plan
            .blocks()
            .iter()
            .enumerate()
            .map(|(site, block)| {
                let operands = match block.terminator() {
                    Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
                    Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
                    _ => vec![],
                };
                operands
                    .into_iter()
                    .filter(|value| !plan.constants().iter().any(|c| c.value() == *value))
                    .find_map(|value| {
                        let (no, yes) = facts.boolean_branch_edges(site as u32, value)?;
                        Some((value.get(), no as usize, yes as usize))
                    })
            })
            .collect::<Vec<_>>();
        let mut states = vec![None::<BTreeMap<u32, bool>>; plan.blocks().len()];
        states[plan.entry() as usize] = Some(BTreeMap::new());
        let mut queued = vec![false; states.len()];
        let mut pending = VecDeque::from([plan.entry() as usize]);
        queued[plan.entry() as usize] = true;
        while let Some(site) = pending.pop_front() {
            queued[site] = false;
            let mut after = states[site].as_ref().unwrap().clone();
            for value in &flow.defs[site] {
                after.remove(value);
            }
            for &next in &flow.successors[site] {
                let mut edge = after.clone();
                if let Some((value, no, yes)) = branches[site] {
                    if next == no {
                        edge.insert(value, false);
                    }
                    if next == yes {
                        edge.insert(value, true);
                    }
                }
                let changed = if let Some(existing) = &mut states[next] {
                    let count = existing.len();
                    existing.retain(|value, truth| edge.get(value) == Some(truth));
                    count != existing.len()
                } else {
                    states[next] = Some(edge);
                    true
                };
                if changed && !queued[next] {
                    queued[next] = true;
                    pending.push_back(next);
                }
            }
        }
        Self {
            known: states.into_iter().map(Option::unwrap_or_default).collect(),
            branches,
        }
    }
    pub fn known(&self, site: usize) -> &BTreeMap<u32, bool> {
        &self.known[site]
    }
    pub fn successors(&self, flow: &Flow, site: usize, known: &BTreeMap<u32, bool>) -> Vec<usize> {
        if let Some((value, no, yes)) = self.branches[site] {
            if let Some(&truth) = known.get(&value) {
                return vec![if truth { yes } else { no }];
            }
        }
        flow.successors[site].clone()
    }
}

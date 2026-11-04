//! Ordinary source-home liveness at a native region's cold boundaries.
//! Private reconciliation words remain independently retained by its proof.
use super::super::prepare::PreparedNativeProgram;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeProgramReadRole as Read,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) struct Liveness {
    before: Vec<BTreeSet<usize>>,
}
impl Liveness {
    pub(super) fn derive(
        program: &PreparedNativeProgram,
        candidates: &BTreeSet<usize>,
    ) -> Result<Self, &'static str> {
        let plan = program.plan();
        let facts = plan.facts();
        let mut values = BTreeMap::new();
        for index in 0..plan.transport_input_count() {
            let value = plan.transport_input_value(index);
            values.insert(value.get(), value);
        }
        for constant in plan.constants() {
            values.insert(constant.value().get(), constant.value());
        }
        // An unknown producer keeps its old conservative homes. This inventory
        // only identifies existing constructor-private words; it grants no new
        // interpretation, current-value or execution authority.
        for block in plan.blocks() {
            let output = match block.action() {
                Action::Scalar(Scalar::Primitive { output, .. } | Scalar::Copy { output, .. })
                | Action::LibraryCall { output, .. }
                | Action::Structural { output, .. } => Some(*output),
                _ => None,
            };
            if let Some(value) = output {
                values.insert(value.get(), value);
            }
        }
        let count = plan.blocks().len();
        let mut action = vec![BTreeSet::new(); count];
        let mut terminal = action.clone();
        let mut definitions = action.clone();
        let mut known = BTreeSet::new();
        let mut pinned = BTreeSet::new();
        for (&index, &value) in &values {
            let mut homes = BTreeSet::new();
            if plan.values()[index as usize].scalar_kind().is_some() {
                homes.insert(index as usize);
            }
            if let Some((base, count)) = program.metadata().scalar_aggregates.slot(value) {
                homes.extend(base..base + count);
            }
            homes.retain(|home| candidates.contains(home));
            if homes.is_empty() {
                continue;
            }
            known.extend(&homes);
            for role in facts.read_roles(value)? {
                match *role {
                    Read::Action(site) => action[site as usize].extend(&homes),
                    Read::Terminator(site) => terminal[site as usize].extend(&homes),
                    // Loop recipes, effects and outputs retain the previous
                    // conservative contract at every boundary. Narrowing those
                    // lifetimes requires a separate reconciliation proof.
                    _ => pinned.extend(&homes),
                }
            }
            for definition in facts.definition_sites(index)? {
                if let Some(site) = definition.site() {
                    definitions[site as usize].extend(&homes);
                }
            }
        }
        pinned.extend(candidates.difference(&known).copied());
        let successors = (0..count)
            .map(|site| facts.successors(site as u32))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            before: solve(&action, &terminal, &definitions, &successors, &pinned),
        })
    }
    pub(super) fn at(&self, site: usize) -> &BTreeSet<usize> {
        &self.before[site]
    }
}

fn solve(
    action: &[BTreeSet<usize>],
    terminal: &[BTreeSet<usize>],
    definitions: &[BTreeSet<usize>],
    successors: &[[Option<u32>; 2]],
    pinned: &BTreeSet<usize>,
) -> Vec<BTreeSet<usize>> {
    let mut predecessors = vec![Vec::new(); successors.len()];
    for (site, next) in successors.iter().enumerate() {
        for &next in next.iter().flatten() {
            predecessors[next as usize].push(site);
        }
    }
    let mut before = vec![BTreeSet::new(); successors.len()];
    let mut queued = vec![true; successors.len()];
    let mut pending = (0..successors.len()).rev().collect::<VecDeque<_>>();
    while let Some(site) = pending.pop_front() {
        queued[site] = false;
        let mut after = terminal[site].clone();
        for &next in successors[site].iter().flatten() {
            after.extend(&before[next as usize]);
        }
        let live = action[site]
            .union(pinned)
            .copied()
            .chain(after.difference(&definitions[site]).copied())
            .collect();
        if live != before[site] {
            before[site] = live;
            for &prior in &predecessors[site] {
                if !core::mem::replace(&mut queued[prior], true) {
                    pending.push_back(prior);
                }
            }
        }
    }
    before
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set(values: &[usize]) -> BTreeSet<usize> {
        values.iter().copied().collect()
    }
    #[test]
    fn source_liveness_keeps_loop_carried_reads_and_old_values_before_overwrite() {
        // 0: seed x; 1: branch on x; 2: x = x + 1, then backedge;
        // 3: publish x. The unrelated earlier temporary y dies before 1.
        let action = [set(&[]), set(&[]), set(&[0]), set(&[0])];
        let terminal = [set(&[]), set(&[0]), set(&[]), set(&[])];
        let definitions = [set(&[0, 1]), set(&[]), set(&[0]), set(&[])];
        let successors = [
            [Some(1), None],
            [Some(2), Some(3)],
            [Some(1), None],
            [None, None],
        ];
        let result = solve(&action, &terminal, &definitions, &successors, &set(&[9]));
        assert_eq!(
            result,
            [set(&[9]), set(&[0, 9]), set(&[0, 9]), set(&[0, 9])]
        );
    }
    #[test]
    fn source_liveness_does_not_require_old_same_site_terminator_result() {
        let empty = set(&[]);
        let result = solve(
            &[empty.clone(), set(&[1]), set(&[2])],
            &[set(&[0]), empty.clone(), empty.clone()],
            &[set(&[0]), empty.clone(), empty],
            &[[Some(1), Some(2)], [None, None], [None, None]],
            &set(&[]),
        );
        assert_eq!(result, [set(&[1, 2]), set(&[1]), set(&[2])]);
    }
}

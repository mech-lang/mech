//! Repeated tests of one exact Bool cannot contradict the edge that reached a
//! Map write. This optional proof never carries truth through a redefinition,
//! owning latch or activation boundary. Unproved edges remain reachable.
use super::*;
use crate::native_artifact::{NativeProgramProducerRole as Role, NativeScalarKind};

fn reachable(
    plan: &Plan,
    start: usize,
    latch: u32,
    removed_node: Option<usize>,
    removed_edge: Option<(usize, usize)>,
) -> Vec<bool> {
    let mut seen = vec![false; plan.blocks().len()];
    let mut pending = vec![start];
    while let Some(site) = pending.pop() {
        if site == latch as usize
            || removed_node == Some(site)
            || core::mem::replace(&mut seen[site], true)
        {
            continue;
        }
        pending.extend(
            successors(plan, site)
                .into_iter()
                .filter(|&next| removed_edge != Some((site, next))),
        );
    }
    seen
}

pub(super) fn implied_edges(
    plan: &Plan,
    iteration: u32,
    body: u32,
    latch: u32,
    write: usize,
) -> BTreeSet<(usize, usize)> {
    let mut excluded = BTreeSet::new();
    let facts = plan.facts();
    let reached = reachable(plan, body as usize, latch, None, None);
    if !reached[write] {
        return excluded;
    }
    let mut branches = BTreeMap::<u32, Vec<(usize, usize, usize)>>::new();
    for (site, block) in plan.blocks().iter().enumerate() {
        if block.iteration() != Some(iteration) || !reached[site] {
            continue;
        }
        let operands = match block.terminator() {
            Terminator::Scalar(Term::Branch { condition, .. }) => vec![condition],
            Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => continue,
        };
        for predicate in operands {
            if let Some((no, yes)) = facts.boolean_branch_edges(site as u32, predicate) {
                branches.entry(predicate.get()).or_default().push((
                    site,
                    no as usize,
                    yes as usize,
                ));
            }
        }
    }
    // Optional proof effort is bounded independently of source size. Declining
    // it preserves every original old-version read and the canonical storage.
    if branches.len() > 64 {
        return excluded;
    }
    for (predicate, branches) in branches {
        let predicate = Value(predicate);
        if plan.values()[predicate.get() as usize].scalar_kind() != Some(NativeScalarKind::Bool) {
            continue;
        }
        let Ok([definition]) = facts.definition_sites(predicate.get()) else {
            continue;
        };
        if definition.role() != Role::Scalar || definition.iteration() != Some(iteration) {
            continue;
        }
        let Some(producer) = definition.site().map(|site| site as usize) else {
            continue;
        };
        // A unique producer still executes repeatedly on a backedge. Reject
        // any repeat before the owning latch, including a nested/outer loop
        // that exits and re-enters this traversal. No previous visit's truth
        // or one alternative definition of a phi can enter this proof.
        if successors(plan, producer).into_iter().any(|next| {
            let following = reachable(plan, next, latch, None, None);
            following[producer]
                // Activation can refresh live inputs independently of ordinary
                // definitions. A boundary before this visit is harmless; one
                // reached after its producer invalidates the optional proof.
                || plan.activation_boundary().is_some_and(|site| following[site as usize])
        }) {
            continue;
        }
        let without_producer = reachable(plan, body as usize, latch, Some(producer), None);
        let mut truth = None;
        let mut conflict = false;
        for &(site, no, yes) in &branches {
            if without_producer[site] {
                continue;
            }
            let requires_yes =
                !reachable(plan, body as usize, latch, None, Some((site, yes)))[write];
            let requires_no = !reachable(plan, body as usize, latch, None, Some((site, no)))[write];
            let selected = match (requires_yes, requires_no) {
                (true, false) => true,
                (false, true) => false,
                _ => continue,
            };
            if truth.is_some_and(|truth| truth != selected) {
                conflict = true;
                break;
            }
            truth = Some(selected);
        }
        if conflict {
            continue;
        }
        if let Some(truth) = truth {
            excluded.extend(
                branches
                    .into_iter()
                    .map(|(site, no, yes)| (site, if truth { no } else { yes })),
            );
        }
    }
    excluded
}

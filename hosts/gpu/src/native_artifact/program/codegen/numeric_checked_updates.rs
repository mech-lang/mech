//! Bounded local read/update scopes after a mutation or opaque continuation.
//! Reuse the shared traversal domain with fresh symbols; export only update
//! facts, never loop guards, completion, funding or speculative entry facts.
use super::*;
use mech_engine::native_artifact::{
    NativeIterationTerminator as Term, NativeMatchTerminator as Branch,
};

const MAX_LOCAL_SITES: usize = 32;
const MAX_LOCAL_WRITES: usize = 256;

/// These are semantic carrier values, not disjoint raw pointers. The exact
/// numeric provenance/current-version proof binds each distinct structural
/// leaf to its own owner home. The first-change boundary retains every live
/// owner and lets Core copy shared backing before replacing this one value;
/// subsequent private stores can affect only that exclusive owner. Therefore
/// even two incoming leaves sharing backing keep independent logical values.
/// Decline captures, overlapping homes and any noncanonical root binding.
pub(super) fn independent_carriers(numeric: &Numeric, written: usize) -> Vec<usize> {
    use crate::native_artifact::program::prepare::readonly_numeric::ArrayStorage;
    let target = &numeric.arrays[written];
    let ArrayStorage::Carrier { field, physical } = target.storage else {
        return Vec::new();
    };
    if target.root != field || !target.writable {
        return Vec::new();
    }
    numeric
        .arrays
        .iter()
        .enumerate()
        .filter_map(|(at, candidate)| {
            let ArrayStorage::Carrier {
                field: other,
                physical: home,
            } = candidate.storage
            else {
                return None;
            };
            (candidate.root == other && other != field && home != physical).then_some(at)
        })
        .collect()
}

pub(super) fn derive(code: CodePlan<'_>, numeric: &Numeric, shadow: usize) -> Proof {
    let CodePlan::Program(program) = code else {
        return Proof::default();
    };
    let owned = numeric.sites.iter().copied().collect::<BTreeSet<_>>();
    let mut predecessors = vec![(0u8, 0usize); code.blocks().len()];
    for (site, visit) in program.metadata().visits.iter().enumerate() {
        for next in visit.successors().into_iter().flatten() {
            let entry = &mut predecessors[next as usize];
            entry.0 = entry.0.saturating_add(1).min(2);
            entry.1 = site;
        }
    }
    let jump = |site: usize| match code.blocks()[site].terminator() {
        Term::Scalar(Branch::Jump(next)) => Some(next),
        _ => None,
    };
    let pure = |site: usize| match numeric.action(site) {
        Some(
            Operation::Read { .. }
            | Operation::Length { .. }
            | Operation::Frame
            | Operation::Aggregate,
        ) => true,
        Some(Operation::Write { array, .. }) => numeric.arrays[*array].width == 8,
        None => matches!(code.blocks()[site].action(), Action::Scalar(_)),
        // Even an otherwise transparent array-copy recipe is a conservative
        // boundary here. Growth, copies and callbacks are not crossed. A
        // supported replacement exports only the independent carrier proof.
        Some(_) => false,
    };
    let mut facts = Proof::default();
    for &(site, _) in numeric.actions.iter().filter(|(_, action)| {
        matches!(action, Operation::Write { array, .. } if numeric.arrays[*array].width == 8)
    }).take(MAX_LOCAL_WRITES) {
        let Some(exit) = jump(site) else { continue; };
        let mut sites = vec![site];
        let mut current = site;
        while sites.len() < MAX_LOCAL_SITES {
            let (count, previous) = predecessors[current];
            if count != 1 || !owned.contains(&previous) || sites.contains(&previous)
                || !pure(previous) || jump(previous) != Some(current as u32) { break; }
            sites.push(previous);
            current = previous;
        }
        sites.reverse();
        let scope = Scope { header: sites[0], latch: site, exit, sites, outside: BTreeSet::new() };
        if let Some(proof) = analyze(code, numeric, shadow, &scope, &[], &[], None,
                                    &traversal_facts::Queries::new()) {
            facts.extend_increment_writes(proof);
        }
    }
    facts
}

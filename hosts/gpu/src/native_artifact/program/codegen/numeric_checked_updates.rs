//! Bounded local read/update scopes after a mutation or opaque continuation.
//! Reuse the shared traversal domain with fresh symbols; export only update
//! facts, never loop guards, completion, funding or speculative entry facts.
use super::*;
use mech_engine::native_artifact::{
    NativeIterationTerminator as Term, NativeMatchTerminator as Branch,
};

const MAX_LOCAL_SITES: usize = 32;
const MAX_LOCAL_WRITES: usize = 256;

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
        None => matches!(code.blocks()[site].action(), Action::Scalar(_)),
        // Even an otherwise transparent array-copy recipe is a conservative
        // boundary here. No write, growth or callback is crossed.
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

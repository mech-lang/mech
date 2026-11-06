//! Exact current-traversal values needed by an ordinary cold continuation.
//! A repeated immutable predicate keeps its established truth until the root
//! latch. This prunes only impossible liveness edges, never generated branches.
use super::*;
use crate::native_artifact::program::prepare::numeric_plan::{Exit, ExitKind};

fn identity(
    facts: &NativeProgramFacts<'_>,
    mut value: mech_engine::native_artifact::NativeMatchValue,
) -> u32 {
    while let Some(source) = facts.unique_identity_copy(value) {
        value = source;
    }
    value.get()
}

fn reaches(
    plan: &Plan,
    forest: &super::super::numeric_phase::Forest,
    target: usize,
    avoid: Option<(usize, usize)>,
) -> bool {
    let mut pending = vec![forest.loops[0].header];
    let mut seen = BTreeSet::new();
    while let Some(site) = pending.pop() {
        if site == target {
            return true;
        }
        if !seen.insert(site) || site == forest.loops[0].latch {
            continue;
        }
        pending.extend(
            forest
                .successors(plan, site)
                .into_iter()
                .filter(|&next| forest.sites.contains(&next) && avoid != Some((site, next))),
        );
    }
    false
}

fn known_predicates(
    plan: &Plan,
    facts: &NativeProgramFacts<'_>,
    forest: &super::super::numeric_phase::Forest,
    from: usize,
) -> BTreeMap<u32, bool> {
    required_predicates(plan, facts, forest, from).0
}

fn required_predicates(
    plan: &Plan,
    facts: &NativeProgramFacts<'_>,
    forest: &super::super::numeric_phase::Forest,
    from: usize,
) -> (BTreeMap<u32, bool>, bool) {
    let mut known = BTreeMap::new();
    let mut consistent = true;
    for &site in &forest.sites {
        let operands = match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => continue,
        };
        for value in operands {
            let root = identity(facts, value);
            let Ok([definition]) = facts.definition_sites(root) else {
                continue;
            };
            if !definition
                .site()
                .is_some_and(|site| forest.sites.contains(&(site as usize)))
                || plan.value_scopes()[root as usize]
                    != (mech_engine::native_artifact::NativeIterationValueScope::Immutable {
                        iteration: Some(forest.loops[0].iteration),
                    })
            {
                continue;
            }
            let Some((no, yes)) = facts.boolean_branch_edges(site as u32, value) else {
                continue;
            };
            for (edge, truth) in [(no, false), (yes, true)] {
                if !reaches(plan, forest, from, Some((site, edge as usize))) {
                    if known
                        .insert(root, truth)
                        .is_some_and(|previous| previous != truth)
                    {
                        consistent = false;
                    }
                }
            }
        }
    }
    (known, consistent)
}

/// Profitability only: repeated immutable guards can make a CFG backedge
/// contradictory even though the graph has a syntactic path to it. Such a
/// prefix retains the existing parent pieces instead of adding a Tail phase.
pub(super) fn retains_native_backedge(
    plan: &Plan,
    tail: &NativeTailLoop,
    facts: &NativeProgramFacts<'_>,
    forest: &super::super::numeric_phase::Forest,
) -> bool {
    let latch = tail.latch() as usize;
    forest.sites.contains(&latch)
        && reaches(plan, forest, latch, None)
        && required_predicates(plan, facts, forest, latch).1
}

fn live_before(
    plan: &Plan,
    tail: &NativeTailLoop,
    facts: &NativeProgramFacts<'_>,
    start: usize,
    known: &BTreeMap<u32, bool>,
) -> Result<BTreeSet<u32>, &'static str> {
    live_query(plan, tail, facts, start, known, None)
}

fn live_query(
    plan: &Plan,
    tail: &NativeTailLoop,
    facts: &NativeProgramFacts<'_>,
    start: usize,
    known: &BTreeMap<u32, bool>,
    wanted: Option<&BTreeSet<u32>>,
) -> Result<BTreeSet<u32>, &'static str> {
    let mut next = BTreeMap::new();
    let mut reads = BTreeMap::<usize, BTreeSet<u32>>::new();
    let mut pending = vec![start];
    while let Some(site) = pending.pop() {
        if next.contains_key(&site) {
            continue;
        }
        let mut used = BTreeSet::new();
        match plan.blocks()[site].action() {
            Action::Initialize { iteration } | Action::Latch { iteration } => {
                if let NativeControlLoop::Tail(child) = &plan.iterations()[*iteration as usize] {
                    used.insert(
                        if matches!(plan.blocks()[site].action(), Action::Initialize { .. }) {
                            child.seed().get()
                        } else {
                            child.next().get()
                        },
                    );
                } else {
                    facts.parallel_action_reads(site as u32, &mut |value| {
                        used.insert(value.get());
                        Ok(())
                    })?;
                }
            }
            _ => facts.parallel_action_reads(site as u32, &mut |value| {
                used.insert(value.get());
                Ok(())
            })?,
        }
        if site == tail.finish() as usize {
            used.insert(tail.returned().get());
        }
        let mut successors = if site == tail.latch() as usize || site == tail.finish() as usize {
            Vec::new()
        } else {
            NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
                .into_iter()
                .flatten()
                .map(|s| s as usize)
                .collect::<Vec<_>>()
        };
        let conditions = match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => Vec::new(),
        };
        for value in conditions {
            used.insert(value.get());
            if let Some(&truth) = known.get(&identity(facts, value))
                && let Some((no, yes)) = facts.boolean_branch_edges(site as u32, value)
            {
                successors.retain(|&s| s == (if truth { yes } else { no }) as usize);
            }
        }
        pending.extend(&successors);
        next.insert(site, successors);
        if let Some(wanted) = wanted {
            used.retain(|value| wanted.contains(value));
        }
        reads.insert(site, used);
    }
    let mut definitions = BTreeMap::<usize, BTreeSet<u32>>::new();
    for value in 0..plan.values().len() {
        if wanted.is_some_and(|wanted| !wanted.contains(&(value as u32))) {
            continue;
        }
        for definition in facts.definition_sites(value as u32)? {
            if let Some(site) = definition.site() {
                definitions
                    .entry(site as usize)
                    .or_default()
                    .insert(value as u32);
            }
        }
    }
    let (mut live, _, _) = solve_liveness(&next, &reads, &definitions);
    Ok(live.remove(&start).unwrap())
}

fn solve_liveness(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
) -> (BTreeMap<usize, BTreeSet<u32>>, usize, usize) {
    // Seed each local read, then propagate only newly live facts through the
    // same graph. Definitions kill incoming successor facts, never local reads.
    let mut live = reads.clone();
    let mut differences = reads.clone();
    let mut predecessors = next
        .keys()
        .map(|&site| (site, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (&site, successors) in next {
        for successor in successors {
            predecessors.get_mut(successor).unwrap().push(site);
        }
    }
    let mut queued = next.keys().copied().collect::<BTreeSet<_>>();
    let mut pending = next
        .keys()
        .rev()
        .copied()
        .collect::<std::collections::VecDeque<_>>();
    let mut visits = 0usize;
    let mut propagated_facts = 0usize;
    while let Some(site) = pending.pop_front() {
        queued.remove(&site);
        visits = visits.saturating_add(1);
        let values = std::mem::take(differences.get_mut(&site).unwrap());
        propagated_facts = propagated_facts.saturating_add(values.len());
        for &predecessor in &predecessors[&site] {
            let kills = definitions.get(&predecessor);
            let destination = live.get_mut(&predecessor).unwrap();
            let mut changed = false;
            for &value in &values {
                if !kills.is_some_and(|kills| kills.contains(&value)) && destination.insert(value) {
                    differences.get_mut(&predecessor).unwrap().insert(value);
                    changed = true;
                }
            }
            if changed && queued.insert(predecessor) {
                pending.push_back(predecessor);
            }
        }
    }
    (live, visits, propagated_facts)
}

#[cfg(test)]
#[path = "numeric_liveness_tests.rs"]
mod liveness_tests;

pub(super) fn derive(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    forest: &super::super::numeric_phase::Forest,
    provenance: &mut Provenance<'_, '_>,
    arrays: &[Array],
) -> Result<Vec<Exit>, &'static str> {
    let mut exits = Vec::new();
    let mut boundaries = forest
        .cold
        .iter()
        .map(|child| child.initialize)
        .chain(forest.stops.iter().copied())
        .map(|to| (to, None, ExitKind::Unsupported))
        .collect::<Vec<_>>();
    if forest.checked_children {
        for child in forest.loops.iter().skip(1) {
            boundaries.push((
                child.initialize,
                None,
                ExitKind::ChildInitialize {
                    iteration: child.iteration,
                },
            ));
            boundaries.push((
                child.header,
                Some(child.latch),
                ExitKind::ChildHeader {
                    iteration: child.iteration,
                },
            ));
        }
    }
    for (to, from, kind) in boundaries {
        let from = if let Some(from) = from {
            from
        } else {
            let [from] = provenance.facts.predecessors(to as u32)? else {
                return Err("cold initializer lacks a unique exact predecessor");
            };
            *from as usize
        };
        if !forest.sites.contains(&from) || !reaches(plan, forest, from, None) {
            return Err("cold predecessor is outside the native traversal");
        }
        let known = known_predicates(plan, provenance.facts, forest, from);
        let live = live_before(plan, tail, provenance.facts, to, &known)?;
        let mut export = Exit {
            from,
            to,
            frames: Vec::new(),
            aliases: Vec::new(),
            values: live.iter().copied().collect(),
            retire_carrier: (!forest.prepaid_sites.is_empty() || forest.checked_children)
                && !live.contains(&tail.carrier().get()),
            kind,
        };
        for value in live {
            if plan.values()[value as usize].scalar_kind().is_some()
                || !provenance.facts.definition_sites(value)?.iter().any(|d| {
                    d.site()
                        .is_some_and(|s| forest.sites.contains(&(s as usize)))
                })
            {
                continue;
            }
            if frames.fields_at(value as usize).is_some() {
                export.frames.push(value);
            } else {
                let field = provenance
                    .array(value, None)
                    .ok_or("cold live value has no exact buffer binding")?;
                let array = arrays
                    .iter()
                    .position(|a| a.field == field)
                    .ok_or("cold live buffer is outside the phase")?;
                export.aliases.push((value, array));
            }
        }
        exits.push(export);
    }
    Ok(exits)
}

pub(super) fn boundary_values(
    plan: &Plan,
    tail: &NativeTailLoop,
    facts: &NativeProgramFacts<'_>,
    forest: &super::super::numeric_phase::Forest,
    frames: &[FrameWords],
    header: BTreeSet<u32>,
) -> Result<super::super::numeric_plan::BoundaryValues, &'static str> {
    let mut candidates = frames
        .iter()
        .map(|frame| frame.value)
        .collect::<BTreeSet<_>>();
    candidates.extend([tail.carrier().get(), tail.returned().get()]);
    for &site in &forest.sites {
        match plan.blocks()[site].action() {
            Action::Initialize { iteration } | Action::Latch { iteration }
                if matches!(
                    plan.iterations()[*iteration as usize],
                    NativeControlLoop::Tail(_)
                ) =>
            {
                let NativeControlLoop::Tail(child) = &plan.iterations()[*iteration as usize] else {
                    unreachable!()
                };
                candidates.insert(
                    if matches!(plan.blocks()[site].action(), Action::Initialize { .. }) {
                        child.seed().get()
                    } else {
                        child.next().get()
                    },
                );
            }
            _ => facts.parallel_action_reads(site as u32, &mut |value| {
                candidates.insert(value.get());
                Ok(())
            })?,
        }
        match plan.blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => {
                candidates.insert(condition.get());
            }
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                candidates.extend([left.get(), right.get()]);
            }
            _ => {}
        }
    }
    for value in 0..plan.values().len() {
        if facts
            .definition_sites(value as u32)?
            .iter()
            .any(|definition| {
                definition
                    .site()
                    .is_some_and(|site| forest.sites.contains(&(site as usize)))
            })
        {
            candidates.insert(value as u32);
        }
    }
    let known = BTreeMap::new();
    Ok(super::super::numeric_plan::BoundaryValues {
        candidates: candidates.iter().copied().collect(),
        header: header.into_iter().collect(),
        exit: live_query(
            plan,
            tail,
            facts,
            tail.exit() as usize,
            &known,
            Some(&candidates),
        )?
        .into_iter()
        .collect(),
    })
}

/// Only lexical local values dead at entry and wholly defined/read in this
/// selected traversal can lose their historical private SSA contents. This
/// changes neither canonical values nor source definitions. A live capture,
/// carrier, escaped output or partially selected definition is never freshened.
/// Return the same header query for boundary construction in this derivation;
/// its plan, root and empty predicate context have not changed between uses.
pub(super) fn fresh_values(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    forest: &super::super::numeric_phase::Forest,
    facts: &NativeProgramFacts<'_>,
) -> Result<(Vec<u32>, BTreeSet<u32>), &'static str> {
    use mech_engine::native_artifact::{
        NativeIterationValueScope as Scope, NativeProgramReadRole as Read,
    };
    let live = live_before(plan, tail, facts, tail.header() as usize, &BTreeMap::new())?;
    let scopes = forest
        .loops
        .iter()
        .map(|loop_| loop_.iteration)
        .collect::<BTreeSet<_>>();
    let mut outputs = BTreeMap::new();
    for block in plan.blocks() {
        let output = match block.action() {
            Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
            | Action::Structural { output, .. }
            | Action::LibraryCall { output, .. } => *output,
            _ => continue,
        };
        outputs.insert(output.get(), output);
    }
    let mut fresh = Vec::new();
    for (value, sealed) in outputs {
        if live.contains(&value)
            || !matches!(plan.value_scopes()[value as usize], Scope::Immutable { iteration: Some(owner) } if scopes.contains(&owner))
            || !(plan.values()[value as usize].scalar_kind().is_some()
                || frames.fields_at(value as usize).is_some()
                || aggregates.slot_at(value as usize).is_some())
        {
            continue;
        }
        let definitions = facts.definition_sites(value)?;
        if definitions.is_empty()
            || definitions.iter().any(|d| {
                !d.site()
                    .is_some_and(|site| forest.sites.contains(&(site as usize)))
            })
        {
            continue;
        }
        if facts.read_roles(sealed)?.iter().any(|role| !matches!(role, Read::Action(site) | Read::Terminator(site) if forest.sites.contains(&(*site as usize)))) {
            continue;
        }
        fresh.push(value);
    }
    Ok((fresh, live))
}

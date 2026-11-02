//! Exact current-traversal values needed by an ordinary cold continuation.
//! A repeated immutable predicate keeps its established truth until the root
//! latch. This prunes only impossible liveness edges, never generated branches.
use super::*;
use crate::native_artifact::program::prepare::numeric_plan::Exit;

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
    let mut known = BTreeMap::new();
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
                    known.insert(root, truth);
                }
            }
        }
    }
    known
}

fn live_before(
    plan: &Plan,
    tail: &NativeTailLoop,
    facts: &NativeProgramFacts<'_>,
    start: usize,
    known: &BTreeMap<u32, bool>,
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
        reads.insert(site, used);
    }
    let mut definitions = BTreeMap::<usize, BTreeSet<u32>>::new();
    for value in 0..plan.values().len() {
        for definition in facts.definition_sites(value as u32)? {
            if let Some(site) = definition.site() {
                definitions
                    .entry(site as usize)
                    .or_default()
                    .insert(value as u32);
            }
        }
    }
    let mut live = next
        .keys()
        .map(|&s| (s, BTreeSet::<u32>::new()))
        .collect::<BTreeMap<_, _>>();
    loop {
        let mut changed = false;
        for (&site, successors) in next.iter().rev() {
            let mut values = successors
                .iter()
                .flat_map(|s| live[s].iter().copied())
                .collect::<BTreeSet<_>>();
            if let Some(writes) = definitions.get(&site) {
                values.retain(|v| !writes.contains(v));
            }
            values.extend(&reads[&site]);
            changed |= live[&site] != values;
            live.insert(site, values);
        }
        if !changed {
            break;
        }
    }
    Ok(live.remove(&start).unwrap())
}

pub(super) fn derive(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    forest: &super::super::numeric_phase::Forest,
    provenance: &mut Provenance<'_, '_>,
    arrays: &[Array],
) -> Result<Vec<Exit>, &'static str> {
    let mut exits = Vec::new();
    for to in forest
        .cold
        .iter()
        .map(|child| child.initialize)
        .chain(forest.stops.iter().copied())
    {
        let [from] = provenance.facts.predecessors(to as u32)? else {
            return Err("cold initializer lacks a unique exact predecessor");
        };
        let from = *from as usize;
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

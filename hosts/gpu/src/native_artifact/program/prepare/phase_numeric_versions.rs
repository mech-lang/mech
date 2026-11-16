//! Current functional versions on one acyclic leaf traversal. The ordinary
//! must-analysis is retained; a missed proof may split on one uniquely defined
//! current Bool, checking BOTH truth values and merging every other branch.
//! This changes eligibility only, never execution or ownership authority.
use super::*;
use mech_engine::native_artifact::{NativeIterationValueScope as Scope, NativeMatchValue as Value};
type Key = (u32, Option<usize>);
type State = BTreeMap<Key, bool>;

struct Summary<'a> {
    tail: &'a NativeTailLoop,
    changed: BTreeSet<usize>,
    outer: BTreeSet<Key>,
}

fn summaries<'a>(
    plan: &'a Plan,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    forest: &super::super::numeric_phase::Forest,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    exits: &[super::super::numeric_plan::Exit],
) -> Vec<Summary<'a>> {
    forest
        .loops
        .iter()
        .map(|loop_| {
            let mut descendants = BTreeSet::from([loop_.iteration]);
            loop {
                let previous = descendants.len();
                for nested in &forest.loops {
                    if nested
                        .parent
                        .is_some_and(|parent| descendants.contains(&parent))
                    {
                        descendants.insert(nested.iteration);
                    }
                }
                if descendants.len() == previous {
                    break;
                }
            }
            let changed = actions
                .iter()
                .filter_map(|(site, operation)| {
                    let Operation::Write { array, .. } = operation else {
                        return None;
                    };
                    plan.blocks()[*site]
                        .iteration()
                        .is_some_and(|iteration| descendants.contains(&iteration))
                        .then_some(arrays[*array].field)
                })
                .collect();
            // A cold continuation inside a child can also need a caller's still-
            // live bindings. Treat these as explicit entry preconditions, proved
            // by the caller before summarizing the child. They are invalidated by
            // writes just like carried bindings; root identity never refreshes an
            // older functional version.
            let outer = exits
                .iter()
                .filter(|exit| {
                    plan.blocks()[exit.from]
                        .iteration()
                        .is_some_and(|iteration| descendants.contains(&iteration))
                })
                .flat_map(|exit| {
                    exit.frames
                        .iter()
                        .flat_map(|&value| managed_keys(frames, provenance, value))
                        .chain(exit.aliases.iter().map(|&(value, _)| (value, None)))
                        .collect::<Vec<_>>()
                })
                .filter(|&(value, _)| {
                    let owner = match plan.value_scopes()[value as usize] {
                        Scope::Carrier { iteration } => Some(iteration),
                        Scope::Immutable { iteration } => iteration,
                    };
                    owner.is_some_and(|iteration| !descendants.contains(&iteration))
                })
                .collect();
            let NativeControlLoop::Tail(tail) = &plan.iterations()[loop_.iteration as usize] else {
                unreachable!("numeric Tail tree");
            };
            Summary {
                tail,
                changed,
                outer,
            }
        })
        .collect()
}

struct Traversal {
    header: usize,
    order: Vec<usize>,
    next: BTreeMap<usize, Vec<usize>>,
    summaries: BTreeSet<usize>,
    local: bool,
}
impl Traversal {
    fn new(
        plan: &Plan,
        tail: &NativeTailLoop,
        sites: &BTreeSet<usize>,
        forest: &super::super::numeric_phase::Forest,
    ) -> Result<Self, &'static str> {
        if sites != &forest.sites || forest.loops[0].header != tail.header() as usize {
            return Err("numeric version forest differs from the selected phase");
        }
        let mut next = BTreeMap::new();
        for &site in sites {
            let successors = if forest.loops.iter().any(|l| l.latch == site) {
                Vec::new()
            } else {
                forest
                    .successors(plan, site)
                    .into_iter()
                    .filter(|n| sites.contains(n))
                    .collect::<Vec<_>>()
            };
            next.insert(site, successors);
        }
        let order = forest.order.clone();
        Ok(Self {
            header: tail.header() as usize,
            order,
            next,
            summaries: BTreeSet::new(),
            local: false,
        })
    }
    fn composed(
        plan: &Plan,
        tail: &NativeTailLoop,
        forest: &super::super::numeric_phase::Forest,
    ) -> Result<Self, &'static str> {
        let iteration = forest
            .loops
            .iter()
            .find(|loop_| loop_.header == tail.header() as usize)
            .ok_or("version traversal lacks its exact loop")?
            .iteration;
        let children = forest
            .loops
            .iter()
            .filter(|loop_| loop_.parent == Some(iteration))
            .map(|loop_| (loop_.initialize, loop_.exit))
            .collect::<BTreeMap<_, _>>();
        let mut pending = vec![tail.header() as usize];
        let mut next = BTreeMap::new();
        while let Some(site) = pending.pop() {
            if site == tail.exit() as usize || next.contains_key(&site) {
                continue;
            }
            let successors = if site == tail.latch() as usize || site == tail.finish() as usize {
                Vec::new()
            } else if let Some(&exit) = children.get(&site) {
                vec![exit]
            } else {
                forest
                    .successors(plan, site)
                    .into_iter()
                    .filter(|next| forest.sites.contains(next))
                    .collect()
            };
            pending.extend(&successors);
            next.insert(site, successors);
        }
        let order = forest
            .order
            .iter()
            .copied()
            .filter(|site| next.contains_key(site))
            .collect::<Vec<_>>();
        if order.len() != next.len() {
            return Err("composed version traversal lacks source order");
        }
        Ok(Self {
            header: tail.header() as usize,
            order,
            next,
            summaries: children.into_keys().collect(),
            local: true,
        })
    }
    // Current-traversal availability, excluding the latch backedge. Whole-plan
    // dominance alone cannot establish availability in THIS recursive visit.
    fn dominates(&self, definition: usize, reader: usize) -> bool {
        let mut pending = vec![self.header];
        let mut seen = BTreeSet::new();
        while let Some(site) = pending.pop() {
            if site == definition || !seen.insert(site) {
                continue;
            }
            if site == reader {
                return false;
            }
            pending.extend(&self.next[&site]);
        }
        true
    }
    fn current_identity(
        &self,
        facts: &NativeProgramFacts<'_>,
        mut value: Value,
        mut reader: usize,
    ) -> Option<Value> {
        let plan = facts.plan();
        let iteration = plan.blocks()[self.header].iteration();
        for _ in 0..plan.values().len() {
            let [definition] = facts.definition_sites(value.get()).ok()? else {
                return None;
            };
            let site = definition.site()? as usize;
            if !self.next.contains_key(&site)
                || definition.iteration() != iteration
                || plan.value_scopes()[value.get() as usize] != (Scope::Immutable { iteration })
                || !self.dominates(site, reader)
            {
                return None;
            }
            match facts.unique_identity_copy(value) {
                Some(source) => {
                    value = source;
                    reader = site;
                }
                None => return Some(value),
            }
        }
        None
    }
    fn predicates(&self, facts: &NativeProgramFacts<'_>) -> Vec<Vec<(usize, (usize, usize))>> {
        let mut predicates = BTreeMap::<u32, Vec<_>>::new();
        for &site in &self.order {
            let operands = match facts.plan().blocks()[site].terminator() {
                Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
                Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
                _ => continue,
            };
            for value in operands {
                if let Some((no, yes)) = facts.boolean_branch_edges(site as u32, value)
                    && let Some(root) = self.current_identity(facts, value, site)
                {
                    predicates
                        .entry(root.get())
                        .or_default()
                        .push((site, (no as usize, yes as usize)));
                }
            }
        }
        predicates
            .into_values()
            .filter(|edges| edges.len() > 1)
            .collect()
    }
}

pub(super) fn validate(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    sites: &BTreeSet<usize>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    result: &ResultRecipe,
    forest: &super::super::numeric_phase::Forest,
    exits: &[super::super::numeric_plan::Exit],
) -> Result<(), &'static str> {
    let summaries = summaries(plan, frames, provenance, forest, actions, arrays, exits);
    if actions.iter().any(|(site, operation)| {
        matches!(operation, Operation::Write { .. })
            && plan.blocks()[*site].iteration() != Some(forest.loops[0].iteration)
    }) {
        // Prove each call's carried/returned versions locally, then use that
        // exact summary at its caller. Independent loop guards do not require
        // a cross-product of truth assignments across nested invocations.
        for loop_ in forest.loops.iter().rev() {
            let NativeControlLoop::Tail(current) = &plan.iterations()[loop_.iteration as usize]
            else {
                unreachable!("numeric Tail tree");
            };
            let traversal = Traversal::composed(plan, current, forest)?;
            validate_traversal(
                plan, current, frames, provenance, &traversal, actions, arrays, result, exits,
                &summaries,
            )?;
        }
        return Ok(());
    }
    // Check the complete graph before any conditional edge can be excluded.
    let traversal = Traversal::new(plan, tail, sites, forest)?;
    validate_traversal(
        plan, tail, frames, provenance, &traversal, actions, arrays, result, exits, &summaries,
    )
}

fn validate_traversal(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    traversal: &Traversal,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    result: &ResultRecipe,
    exits: &[super::super::numeric_plan::Exit],
    summaries: &[Summary<'_>],
) -> Result<(), &'static str> {
    let original = pass(
        plan,
        tail,
        frames,
        provenance,
        traversal,
        actions,
        arrays,
        result,
        &[],
        exits,
        summaries,
    );
    if !matches!(
        original,
        Err("content use reads an older array version"
            | "recursive edge discards or forwards an older array version"
            | "leaf returns an older array version"
            | "child argument reads an older array version"
            | "child returns an older array version")
    ) {
        return original;
    }
    for edges in traversal.predicates(provenance.facts) {
        // One predicate at a time: polynomial, with no valuation cross-product
        // or truth-state cache. Each other branch keeps the conservative meet.
        let no = edges.iter().map(|&(s, (n, _))| (s, n)).collect::<Vec<_>>();
        let yes = edges.iter().map(|&(s, (_, y))| (s, y)).collect::<Vec<_>>();
        if pass(
            plan, tail, frames, provenance, traversal, actions, arrays, result, &no, exits,
            summaries,
        )
        .is_ok()
            && pass(
                plan, tail, frames, provenance, traversal, actions, arrays, result, &yes, exits,
                summaries,
            )
            .is_ok()
        {
            return Ok(());
        }
    }
    original
}

fn managed_keys(
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    value: u32,
) -> Vec<Key> {
    if let Some(fields) = frames.fields_at(value as usize) {
        fields
            .iter()
            .enumerate()
            .filter_map(|(ordinal, field)| {
                matches!(field.lane(), Lane::Managed { .. }).then_some((value, Some(ordinal)))
            })
            .collect()
    } else if provenance.array(value, None).is_some() {
        vec![(value, None)]
    } else {
        Vec::new()
    }
}

fn pass(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    traversal: &Traversal,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    result: &ResultRecipe,
    selected: &[(usize, usize)],
    exits: &[super::super::numeric_plan::Exit],
    summaries: &[Summary<'_>],
) -> Result<(), &'static str> {
    // A child's zero-trip return alone cannot describe the effects of its
    // arbitrary recursive visits. Latches establish the current carried
    // versions inductively; a return invalidates older caller bindings for
    // every root that any visit may change. Only current returned bindings
    // survive that summary. Observable older versions therefore decline.
    let outer = summaries
        .iter()
        .find(|summary| summary.tail.header() == tail.header())
        .ok_or("version traversal lacks its entry preconditions")?;
    let mut states = BTreeMap::<usize, State>::new();
    states.insert(
        tail.header() as usize,
        managed_keys(frames, provenance, tail.carrier().get())
            .into_iter()
            .chain(outer.outer.iter().copied().filter(|_| traversal.local))
            .map(|key| (key, true))
            .collect(),
    );
    for &site in &traversal.order {
        let Some(mut state) = states.remove(&site) else {
            if selected.is_empty() {
                return Err("unreachable numeric body root");
            }
            continue;
        };
        let fresh = |state: &State, key: Key| state.get(&key).copied().unwrap_or(false);
        if let Some(exit) = exits.iter().find(|exit| exit.to == site) {
            for &value in &exit.frames {
                for (ordinal, field) in frames
                    .fields_at(value as usize)
                    .ok_or("cold frame lost its private lanes")?
                    .iter()
                    .enumerate()
                {
                    if matches!(field.lane(), Lane::Managed { .. })
                        && !fresh(&state, (value, Some(ordinal)))
                    {
                        return Err("cold continuation requires an older array version");
                    }
                }
            }
            for &(value, _) in &exit.aliases {
                if !fresh(&state, (value, None)) {
                    return Err("cold continuation requires an older array version");
                }
            }
        }
        match actions.iter().find(|(s, _)| *s == site).map(|(_, op)| op) {
            Some(Operation::Frame) => match frames.action(site).unwrap().operation() {
                Frame::EnumPack { .. } | Frame::Observe { .. } => {
                    return Err("tagged frame outside flat phase version proof");
                }
                Frame::Pack { output } => {
                    let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                        return Err("version pack lost operands");
                    };
                    for (ordinal, operand) in operands.iter().enumerate() {
                        if provenance.array(operand.get(), None).is_some() {
                            let current = fresh(&state, (operand.get(), None));
                            state.insert((output.get(), Some(ordinal)), current);
                        }
                    }
                }
                Frame::Copy { source, output } => {
                    let copied = frames
                        .fields(*source)
                        .ok_or("version copy lost fields")?
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| matches!(f.lane(), Lane::Managed { .. }))
                        .map(|(ordinal, _)| {
                            (
                                (output.get(), Some(ordinal)),
                                fresh(&state, (source.get(), Some(ordinal))),
                            )
                        })
                        .collect::<Vec<_>>();
                    state.extend(copied);
                }
                Frame::Project {
                    source,
                    ordinal,
                    output,
                } => {
                    if provenance.array(output.get(), None).is_some() {
                        let current = fresh(&state, (source.get(), Some(*ordinal)));
                        state.insert((output.get(), None), current);
                    }
                }
            },
            Some(Operation::ArrayCopy) => {
                let Action::Scalar(Scalar::Copy { source, output }) = plan.blocks()[site].action()
                else {
                    return Err("version copy lost action");
                };
                let current = fresh(&state, (source.get(), None));
                state.insert((output.get(), None), current);
            }
            Some(
                operation @ (Operation::Read { array, .. }
                | Operation::Write { array, .. }
                | Operation::Append { array, .. }
                | Operation::AppendLeBytes { array, .. }
                | Operation::Length { array, .. }),
            ) => {
                let Action::LibraryCall {
                    operands, output, ..
                } = plan.blocks()[site].action()
                else {
                    return Err("version access lost operands");
                };
                if (!matches!(operation, Operation::Length { .. }) || arrays[*array].appends)
                    && !fresh(&state, (operands[0].get(), None))
                {
                    return Err("content use reads an older array version");
                }
                if matches!(
                    operation,
                    Operation::Write { .. }
                        | Operation::Append { .. }
                        | Operation::AppendLeBytes { .. }
                ) {
                    let root = arrays[*array].field;
                    for (&(value, field), current) in &mut state {
                        if provenance.array(value, field) == Some(root) {
                            *current = false;
                        }
                    }
                    state.insert((output.get(), None), true);
                }
            }
            _ => {}
        }
        if let Some(summary) = summaries.iter().skip(1).find(|summary| {
            let child = summary.tail;
            if traversal.local {
                traversal.summaries.contains(&site) && child.initialize() as usize == site
            } else {
                child.finish() as usize == site
            }
        }) {
            let child = summary.tail;
            let value = child.returned().get();
            let returned = managed_keys(frames, provenance, value);
            let required = if traversal.local {
                managed_keys(frames, provenance, child.seed().get())
                    .into_iter()
                    .chain(summary.outer.iter().copied())
                    .collect()
            } else {
                returned.clone()
            };
            if required.iter().any(|&key| !fresh(&state, key)) {
                if traversal.local {
                    return Err("child argument reads an older array version");
                }
                return Err("child returns an older array version");
            }
            for (&(value, field), current) in &mut state {
                if provenance
                    .array(value, field)
                    .is_some_and(|root| summary.changed.contains(&root))
                {
                    *current = false;
                }
            }
            state.extend(returned.into_iter().map(|key| (key, true)));
        }
        if let Some(child) = plan
            .iterations()
            .iter()
            .filter_map(|l| match l {
                NativeControlLoop::Tail(t)
                    if t.header() != tail.header() && t.latch() as usize == site =>
                {
                    Some(t)
                }
                _ => None,
            })
            .next()
            && let Some(fields) = frames.fields(child.carrier())
        {
            for (ordinal, field) in fields.iter().enumerate() {
                if matches!(field.lane(), Lane::Managed { .. })
                    && !fresh(&state, (child.next().get(), Some(ordinal)))
                {
                    return Err("child recursive edge forwards an older array version");
                }
            }
        }
        if site == tail.latch() as usize {
            for key in managed_keys(frames, provenance, tail.next().get()) {
                if !fresh(&state, key) {
                    return Err("recursive edge discards or forwards an older array version");
                }
            }
        }
        if site == tail.finish() as usize {
            let current = if traversal.local {
                managed_keys(frames, provenance, tail.returned().get())
                    .iter()
                    .all(|&key| fresh(&state, key))
            } else {
                match result {
                    ResultRecipe::Unreachable => false,
                    ResultRecipe::Scalar | ResultRecipe::Aggregate { .. } => true,
                    ResultRecipe::Array { .. } => fresh(&state, (tail.returned().get(), None)),
                    ResultRecipe::Frame { value, arrays } => arrays
                        .iter()
                        .all(|(ordinal, _)| fresh(&state, (*value, Some(*ordinal)))),
                }
            };
            if !current {
                return Err("leaf returns an older array version");
            }
        }
        for &next in &traversal.next[&site] {
            if selected
                .iter()
                .any(|&(branch, allowed)| branch == site && allowed != next)
            {
                continue;
            }
            if let Some(merged) = states.get_mut(&next) {
                // Definitions remain current only on every reachable incoming
                // path of this pass. A root match does not refresh old aliases.
                merged.retain(|key, current| {
                    *current &= state.get(key).copied().unwrap_or(false);
                    true
                });
            } else {
                states.insert(next, state.clone());
            }
        }
    }
    Ok(())
}

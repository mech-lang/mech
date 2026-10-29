//! Current functional versions on one acyclic leaf traversal. The ordinary
//! must-analysis is retained; a missed proof may split on one uniquely defined
//! current Bool, checking BOTH truth values and merging every other branch.
//! This changes eligibility only, never execution or ownership authority.
use super::*;
use mech_engine::native_artifact::{NativeIterationValueScope as Scope, NativeMatchValue as Value};
type Key = (u32, Option<usize>);
type State = BTreeMap<Key, bool>;

struct Traversal {
    header: usize,
    order: Vec<usize>,
    next: BTreeMap<usize, Vec<usize>>,
}
impl Traversal {
    fn new(
        plan: &Plan,
        tail: &NativeTailLoop,
        sites: &BTreeSet<usize>,
    ) -> Result<Self, &'static str> {
        let mut next = BTreeMap::new();
        let mut incoming = sites
            .iter()
            .map(|&s| (s, 0usize))
            .collect::<BTreeMap<_, _>>();
        for &site in sites {
            let successors = if site == tail.latch() as usize {
                Vec::new()
            } else {
                NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
                    .into_iter()
                    .flatten()
                    .map(|n| n as usize)
                    .filter(|n| sites.contains(n))
                    .collect::<Vec<_>>()
            };
            for n in &successors {
                *incoming.get_mut(n).unwrap() += 1;
            }
            next.insert(site, successors);
        }
        let mut ready = incoming
            .iter()
            .filter_map(|(&s, &n)| (n == 0).then_some(s))
            .collect::<Vec<_>>();
        let mut order = Vec::new();
        while let Some(site) = ready.pop() {
            order.push(site);
            for n in &next[&site] {
                let count = incoming.get_mut(n).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.push(*n);
                }
            }
        }
        if order.len() != sites.len() {
            return Err("numeric traversal has an undeclared cycle");
        }
        Ok(Self {
            header: tail.header() as usize,
            order,
            next,
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
) -> Result<(), &'static str> {
    // Check the complete graph before any conditional edge can be excluded.
    let traversal = Traversal::new(plan, tail, sites)?;
    let original = pass(
        plan,
        tail,
        frames,
        provenance,
        &traversal,
        actions,
        arrays,
        result,
        &[],
    );
    if !matches!(
        original,
        Err("content use reads an older array version"
            | "recursive edge discards or forwards an older array version"
            | "leaf returns an older array version")
    ) {
        return original;
    }
    for edges in traversal.predicates(provenance.facts) {
        // One predicate at a time: polynomial, with no valuation cross-product
        // or truth-state cache. Each other branch keeps the conservative meet.
        let no = edges.iter().map(|&(s, (n, _))| (s, n)).collect::<Vec<_>>();
        let yes = edges.iter().map(|&(s, (_, y))| (s, y)).collect::<Vec<_>>();
        if pass(
            plan, tail, frames, provenance, &traversal, actions, arrays, result, &no,
        )
        .is_ok()
            && pass(
                plan, tail, frames, provenance, &traversal, actions, arrays, result, &yes,
            )
            .is_ok()
        {
            return Ok(());
        }
    }
    original
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
) -> Result<(), &'static str> {
    let mut states = BTreeMap::<usize, State>::new();
    states.insert(
        tail.header() as usize,
        arrays
            .iter()
            .map(|a| ((tail.carrier().get(), Some(a.field)), true))
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
        match actions.iter().find(|(s, _)| *s == site).map(|(_, op)| op) {
            Some(Operation::Frame) => match frames.action(site).unwrap().operation() {
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
            Some(Operation::Read { .. } | Operation::Write { .. }) => {
                let Action::LibraryCall {
                    operands, output, ..
                } = plan.blocks()[site].action()
                else {
                    return Err("version access lost operands");
                };
                if !fresh(&state, (operands[0].get(), None)) {
                    return Err("content use reads an older array version");
                }
                if let Some(Operation::Write { array, .. }) =
                    actions.iter().find(|(s, _)| *s == site).map(|(_, op)| op)
                {
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
        if site == tail.latch() as usize {
            for array in arrays {
                if !fresh(&state, (tail.next().get(), Some(array.field))) {
                    return Err("recursive edge discards or forwards an older array version");
                }
            }
        }
        if site == tail.finish() as usize {
            let key = match result {
                ResultRecipe::Scalar => None,
                ResultRecipe::Array { .. } => Some((tail.returned().get(), None)),
                ResultRecipe::Frame { value, ordinal, .. } => Some((*value, Some(*ordinal))),
            };
            if key.is_some_and(|key| !fresh(&state, key)) {
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

//! Current functional versions on one acyclic leaf traversal. The ordinary
//! must-analysis is retained; a missed proof may split on one uniquely defined
//! current Bool, checking BOTH truth values and merging every other branch.
//! This changes eligibility only, never execution or ownership authority.
use super::*;
use mech_engine::native_artifact::{NativeIterationValueScope as Scope, NativeMatchValue as Value};
type Key = (u32, Option<usize>);
type State = BTreeMap<Key, bool>;
#[path = "numeric_induction.rs"]
mod induction;
#[derive(Default)]
pub(super) struct ColdHomes {
    pub aliases: Vec<(usize, Vec<ColdAlias>)>,
    pub scalars: Vec<(usize, Vec<u32>)>,
}

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
            next.insert(site, successors);
        }
        Self::from_next(tail.header() as usize, next)
    }
    fn from_next(header: usize, next: BTreeMap<usize, Vec<usize>>) -> Result<Self, &'static str> {
        let mut incoming = next
            .keys()
            .map(|&s| (s, 0usize))
            .collect::<BTreeMap<_, _>>();
        for successors in next.values() {
            for n in successors {
                *incoming
                    .get_mut(n)
                    .ok_or("numeric traversal edge leaves its source inventory")? += 1;
            }
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
        if order.len() != next.len() {
            return Err("numeric traversal has an undeclared cycle");
        }
        Ok(Self {
            header,
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

/// Must-current analysis over a structured subtree. The root backedge is the
/// induction boundary; child backedges remain in the graph. Starting at top
/// and meeting every incoming edge proves arbitrary child traversal counts,
/// including zero writes and alternation between branch paths. A correlated
/// predicate is immutable only within its own proved lexical traversal; the
/// analysis forgets it before the next traversal or a completed activation.
pub(super) fn validate_tree(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    sites: &BTreeSet<usize>,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    result: &ResultRecipe,
    tree: &super::super::numeric_tree::Proof,
) -> Result<ColdHomes, &'static str> {
    let mut next = BTreeMap::new();
    for &site in sites {
        next.insert(
            site,
            if site == tail.latch() as usize {
                Vec::new()
            } else {
                NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
                    .into_iter()
                    .flatten()
                    .map(|s| s as usize)
                    .filter(|s| sites.contains(s))
                    .collect()
            },
        );
    }
    let traversal = Traversal {
        header: tail.header() as usize,
        order: sites.iter().copied().collect(),
        next,
    };
    let mut analysis = analyze_tree(
        plan, tail, frames, provenance, &traversal, actions, arrays, result, tree, None,
    )?;
    let original = verify_tree(plan, tail, frames, &analysis, actions, arrays, result, tree);
    if original.is_ok() {
        return cold_aliases(plan, tail, frames, provenance, arrays, tree, &analysis);
    }
    for node in &tree.nodes {
        let mut local_next = traversal.next.clone();
        local_next.insert(node.tail.latch() as usize, Vec::new());
        local_next.insert(node.tail.exit() as usize, Vec::new());
        let mut local_sites = BTreeSet::new();
        let mut pending = vec![node.tail.header() as usize];
        while let Some(site) = pending.pop() {
            if local_sites.insert(site) {
                pending.extend(&local_next[&site]);
            }
        }
        let local = Traversal {
            header: node.tail.header() as usize,
            order: local_sites.into_iter().collect(),
            next: local_next,
        };
        for edges in local.predicates(provenance.facts) {
            let partition = Partition {
                iteration: node.iteration,
                header: node.tail.header() as usize,
                exit: node.tail.exit() as usize,
                edges,
            };
            let refined = analyze_tree(
                plan,
                tail,
                frames,
                provenance,
                &traversal,
                actions,
                arrays,
                result,
                tree,
                Some(&partition),
            )?;
            // Every pass is independently a sound must-analysis. Unioning its
            // proven facts combines predicates without a valuation product;
            // no pass assumes a fact supplied by another pass.
            analysis.strengthen(refined);
            if verify_tree(plan, tail, frames, &analysis, actions, arrays, result, tree).is_ok() {
                return cold_aliases(plan, tail, frames, provenance, arrays, tree, &analysis);
            }
        }
    }
    // Child currentness is an inductive theorem, not a guessed initial fact.
    // A compositional proof can establish it even when one unified fixed point
    // cannot bootstrap both the parent's and the child's carrier invariants.
    let analysis = induction::validate(
        plan, tail, frames, provenance, actions, arrays, result, tree,
    )?;
    cold_aliases(plan, tail, frames, provenance, arrays, tree, &analysis)
}

struct Partition {
    iteration: u32,
    header: usize,
    exit: usize,
    edges: Vec<(usize, (usize, usize))>,
}
struct Analysis {
    inputs: BTreeMap<usize, State>,
    outputs: BTreeMap<usize, State>,
    known: Vec<KnownPredicate>,
}
struct KnownPredicate {
    iteration: u32,
    truth: BTreeMap<usize, bool>,
    edges: Vec<(usize, (usize, usize))>,
}
impl Analysis {
    fn strengthen(&mut self, other: Self) {
        self.known.extend(other.known);
        for (current, other) in [
            (&mut self.inputs, other.inputs),
            (&mut self.outputs, other.outputs),
        ] {
            current.retain(|site, state| {
                let Some(proven) = other.get(site) else {
                    return false;
                };
                for (key, fresh) in state {
                    *fresh |= proven.get(key).copied().unwrap_or(false);
                }
                true
            });
        }
    }
}

fn analyze_tree(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    traversal: &Traversal,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    _result: &ResultRecipe,
    _tree: &super::super::numeric_tree::Proof,
    partition: Option<&Partition>,
) -> Result<Analysis, &'static str> {
    // A key is immutable SSA identity, never just the root number. A store
    // invalidates every earlier identity even when backing storage is shared.
    let mut top = State::new();
    for value in 0..plan.values().len() as u32 {
        if provenance.array(value, None).is_some() {
            top.insert((value, None), true);
        }
        if let Some(fields) = numeric_fields::at(frames, value) {
            for ordinal in 0..fields.len() {
                if provenance.array(value, Some(ordinal)).is_some() {
                    top.insert((value, Some(ordinal)), true);
                }
            }
        }
    }
    let entry = top
        .keys()
        .map(|key| {
            (
                *key,
                arrays.iter().any(|array| array.entry_key(tail) == *key),
            )
        })
        .collect::<State>();
    let mut states = BTreeMap::new();
    for &site in &traversal.order {
        for truth in 0..if partition.is_some() { 3 } else { 1 } {
            states.insert((site, truth), top.clone());
        }
    }
    states.insert((traversal.header, 0), entry.clone());
    let mut pending = std::collections::VecDeque::from([(traversal.header, 0)]);
    let mut queued = BTreeSet::from([(traversal.header, 0)]);
    let mut reachable = BTreeSet::new();
    let mut outputs = BTreeMap::new();
    while let Some((site, truth)) = pending.pop_front() {
        queued.remove(&(site, truth));
        let mut state = states[&(site, truth)].clone();
        reachable.insert((site, truth));
        transfer(plan, frames, provenance, site, actions, arrays, &mut state)?;
        outputs.insert((site, truth), state.clone());
        for &next in &traversal.next[&site] {
            let mut next_truth = truth;
            if let Some(partition) = partition {
                if let Some((_, (no, yes))) =
                    partition.edges.iter().find(|(branch, _)| *branch == site)
                {
                    if no != yes {
                        let edge = if next == *no {
                            1
                        } else if next == *yes {
                            2
                        } else {
                            return Err("numeric predicate edge leaves its source branch");
                        };
                        if truth != 0 && truth != edge {
                            continue;
                        }
                        next_truth = edge;
                    }
                }
                // No predicate observation survives a new lexical traversal or
                // a completed activation. The same SSA number may then contain
                // the opposite Bool, which must be checked independently.
                if next == partition.header || next == partition.exit {
                    next_truth = 0;
                }
            }
            let merged = states
                .get_mut(&(next, next_truth))
                .ok_or("numeric tree freshness edge leaves graph")?;
            let mut changed = false;
            for (key, current) in merged.iter_mut() {
                let value = *current && state.get(key).copied().unwrap_or(false);
                changed |= value != *current;
                *current = value;
            }
            // Each pending visit reads the latest meet from `states`. Several
            // predecessors can update that meet before it runs; queue it once
            // rather than cloning/transferring the same state for every path.
            // Removing it on pop still permits later fixed-point iterations.
            if (changed || !reachable.contains(&(next, next_truth)))
                && queued.insert((next, next_truth))
            {
                pending.push_back((next, next_truth));
            }
        }
    }
    let collapse = |bank: BTreeMap<(usize, u8), State>| {
        let mut result = BTreeMap::<usize, State>::new();
        for ((site, truth), state) in bank {
            if !reachable.contains(&(site, truth)) {
                continue;
            }
            if let Some(current) = result.get_mut(&site) {
                for (key, fresh) in current {
                    *fresh &= state.get(key).copied().unwrap_or(false);
                }
            } else {
                result.insert(site, state);
            }
        }
        result
    };
    let known = partition
        .map(|partition| {
            let mut truths = BTreeMap::<usize, BTreeSet<u8>>::new();
            for &(site, truth) in &reachable {
                truths.entry(site).or_default().insert(truth);
            }
            KnownPredicate {
                iteration: partition.iteration,
                edges: partition.edges.clone(),
                truth: truths
                    .into_iter()
                    .filter_map(|(site, truths)| {
                        (truths.len() == 1 && !truths.contains(&0))
                            .then(|| (site, truths.contains(&2)))
                    })
                    .collect(),
            }
        })
        .into_iter()
        .collect();
    Ok(Analysis {
        inputs: collapse(states),
        outputs: collapse(outputs),
        known,
    })
}

fn verify_tree(
    plan: &Plan,
    tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    analysis: &Analysis,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    result: &ResultRecipe,
    tree: &super::super::numeric_tree::Proof,
) -> Result<(), &'static str> {
    for (&site, state) in &analysis.inputs {
        if let Some(
            operation @ (Operation::Read { array, .. }
            | Operation::Write { array, .. }
            | Operation::Append { array, .. }
            | Operation::Length { array, .. }),
        ) = actions.iter().find(|(s, _)| *s == site).map(|(_, a)| a)
        {
            let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                return Err("tree array action lost operands");
            };
            if (!matches!(operation, Operation::Length { .. }) || arrays[*array].appends)
                && !state
                    .get(&(operands[0].get(), None))
                    .copied()
                    .unwrap_or(false)
            {
                return Err("numeric tree content use reads an older array version");
            }
        }
        let state = &analysis.outputs[&site];
        for node in &tree.nodes {
            if site == node.tail.latch() as usize {
                if let Some(fields) = numeric_fields::at(frames, node.tail.carrier().get()) {
                    for (ordinal, field) in fields.iter().enumerate() {
                        if matches!(field.lane(), Lane::Managed { .. })
                            && !state
                                .get(&(node.tail.next().get(), Some(ordinal)))
                                .copied()
                                .unwrap_or(false)
                        {
                            return Err(
                                "numeric tree recursive edge forwards an older array version",
                            );
                        }
                    }
                }
            }
            if site == node.tail.finish() as usize {
                let returned = node.tail.returned().get();
                let keys = state.keys().filter(|(value, _)| *value == returned);
                if keys
                    .into_iter()
                    .any(|key| !state.get(key).copied().unwrap_or(false))
                {
                    return Err("numeric tree child returns an older array version");
                }
            }
        }
    }
    // Result recipe is checked independently against the root's terminal edge.
    if let Some(state) = analysis.outputs.get(&(tail.finish() as usize)) {
        let fresh = |key| state.get(&key).copied().unwrap_or(false);
        let current = match result {
            ResultRecipe::Scalar | ResultRecipe::Aggregate { .. } => true,
            ResultRecipe::Array { .. } => fresh((tail.returned().get(), None)),
            ResultRecipe::Frame { value, arrays } => arrays
                .iter()
                .all(|(field, _)| fresh((*value, Some(*field)))),
        };
        if !current {
            return Err("numeric tree returns an older array version");
        }
    }
    Ok(())
}

/// A header bailout does not replay already executed ancestor projections.
/// Restore exactly the current array aliases and scalar Frame homes its
/// ordinary continuation needs. Older mutable aliases are never promoted to
/// the current root version, and managed Frame intermediates still decline.
fn cold_aliases(
    plan: &Plan,
    _tail: &NativeTailLoop,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    arrays: &[Array],
    tree: &super::super::numeric_tree::Proof,
    analysis: &Analysis,
) -> Result<ColdHomes, &'static str> {
    use mech_engine::native_artifact::NativeProgramReadRole as Read;
    let ancestor = |mut child: usize, parent: usize| {
        for _ in 0..tree.nodes.len() {
            if child == parent {
                return true;
            }
            let Some(next) = tree.nodes[child].parent else {
                return false;
            };
            child = next;
        }
        false
    };
    let mut recipes = ColdHomes::default();
    let mut count = 0usize;
    let bound = plan
        .values()
        .len()
        .checked_mul(tree.nodes.len())
        .ok_or("cold alias inventory size overflows")?;
    for (current, node) in tree.nodes.iter().enumerate() {
        let header = node.tail.header() as usize;
        let Some(state) = analysis.inputs.get(&header) else {
            continue;
        };
        let mut aliases = Vec::new();
        let mut scalars = Vec::new();
        for value in 0..plan.values().len() as u32 {
            let Scope::Immutable {
                iteration: Some(scope),
            } = plan.value_scopes()[value as usize]
            else {
                continue;
            };
            let Some(owner) = tree.nodes.iter().position(|node| node.iteration == scope) else {
                continue;
            };
            if owner == current || !ancestor(current, owner) {
                continue;
            }
            let root = provenance.array(value, None);
            let virtual_frame = numeric_fields::at(frames, value).is_some();
            if root.is_none() && !virtual_frame {
                continue;
            }
            let definitions = provenance.facts.definition_sites(value)?;
            if definitions.is_empty()
                || definitions.iter().any(|definition| {
                    definition.iteration() != Some(scope)
                        || definition
                            .site()
                            .is_none_or(|site| tree.node(site as usize).is_none())
                })
            {
                // An ordinary enclosing definition already has its own Arena
                // publication. Only definitions elided by this tree need one.
                continue;
            }
            let definitions = definitions
                .iter()
                .filter_map(|definition| definition.site())
                .map(|site| site as usize)
                .collect::<BTreeSet<_>>();
            let mut readers = BTreeSet::new();
            let native_value = match plan.blocks()
                [*definitions.first().ok_or("cold alias has no producer")?]
            .action()
            {
                Action::Scalar(Scalar::Copy { output, .. })
                | Action::LibraryCall { output, .. }
                | Action::Structural { output, .. } => *output,
                _ => return Err("cold alias producer has no immutable result"),
            };
            if native_value.get() != value {
                return Err("cold alias producer differs from its current value");
            }
            for role in provenance.facts.read_roles(native_value)? {
                readers.insert(match *role {
                    Read::Action(site) | Read::Terminator(site) => site as usize,
                    Read::LoopSeed(iteration) => {
                        plan.iterations()[iteration as usize].as_ref().initialize() as usize
                    }
                    Read::LoopNext(iteration) => {
                        plan.iterations()[iteration as usize].as_ref().latch() as usize
                    }
                    Read::LoopReturned(iteration) => match &plan.iterations()[iteration as usize] {
                        NativeControlLoop::Tail(tail) => tail.finish() as usize,
                        _ => return Err("numeric cold alias return belongs to a non-Tail loop"),
                    },
                    Read::CollectionSource(_) | Read::Output(_) | Read::Effect(_) => {
                        return Err("numeric tree cold alias has an unsupported escaping use");
                    }
                });
            }
            // A current ancestor Bool remains invariant until this alias's
            // scope ends. Use only independently proved entry truth, never a
            // branch valuation borrowed from another traversal/activation.
            let mut allowed = BTreeMap::new();
            for predicate in &analysis.known {
                let Some(predicate_owner) = tree
                    .nodes
                    .iter()
                    .position(|node| node.iteration == predicate.iteration)
                else {
                    continue;
                };
                if !ancestor(owner, predicate_owner) {
                    continue;
                }
                if let Some(truth) = predicate.truth.get(&header) {
                    for &(site, (no, yes)) in &predicate.edges {
                        let next = if *truth { yes } else { no };
                        if allowed.insert(site, next).is_some_and(|old| old != next) {
                            return Err("cold alias predicate facts conflict");
                        }
                    }
                }
            }
            let mut pending = vec![header];
            let mut seen = BTreeSet::new();
            let mut needed = false;
            while let Some(site) = pending.pop() {
                if !seen.insert(site) || definitions.contains(&site) {
                    continue;
                }
                if readers.contains(&site) {
                    needed = true;
                    break;
                }
                if site == tree.nodes[owner].tail.header() as usize
                    || site == tree.nodes[owner].tail.latch() as usize
                    || site == tree.nodes[owner].tail.finish() as usize
                    || site == tree.nodes[owner].tail.exit() as usize
                {
                    continue;
                }
                for next in NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
                    .into_iter()
                    .flatten()
                {
                    if allowed
                        .get(&site)
                        .is_none_or(|selected| *selected == next as usize)
                    {
                        pending.push(next as usize);
                    }
                }
            }
            if !needed {
                continue;
            }
            count = count
                .checked_add(1)
                .ok_or("cold alias inventory count overflows")?;
            if count > bound {
                return Err("cold alias inventory exceeds bounded SSA storage");
            }
            // Active carriers have their own recipe. Other wholly scalar
            // ancestor frames retain exact immutable shadow words and can be
            // restored into existing physical lanes without owners or Values.
            // A managed frame still needs its own current-version recipe.
            if virtual_frame {
                let fields = numeric_fields::at(frames, value).unwrap();
                for (ordinal, field) in fields.iter().enumerate() {
                    if matches!(field.lane(), Lane::Managed { .. }) {
                        if provenance.array(value, Some(ordinal)).is_none()
                            || !state.get(&(value, Some(ordinal))).copied().unwrap_or(false)
                        {
                            return Err(
                                "numeric cold frame requires an older or unavailable managed leaf",
                            );
                        }
                    }
                }
                // Whole-plan dominance is insufficient for a repeated SSA
                // visit. Every path in THIS ancestor traversal must cross a
                // current producer before reaching this child header.
                let owner_tail = &tree.nodes[owner].tail;
                let mut pending = vec![owner_tail.header() as usize];
                let mut seen = BTreeSet::new();
                while let Some(site) = pending.pop() {
                    if !seen.insert(site) || definitions.contains(&site) {
                        continue;
                    }
                    if site == header {
                        return Err(
                            "numeric cold scalar frame lacks a current dominating producer",
                        );
                    }
                    if site == owner_tail.latch() as usize
                        || site == owner_tail.finish() as usize
                        || site == owner_tail.exit() as usize
                    {
                        continue;
                    }
                    pending.extend(
                        NativeProgramFacts::control_successors(plan.blocks()[site].terminator())
                            .into_iter()
                            .flatten()
                            .map(|next| next as usize),
                    );
                }
                scalars.push(value);
                continue;
            }
            let root = root.ok_or("cold alias loses its logical array root")?;
            let array = arrays
                .iter()
                .position(|array| array.root == root)
                .ok_or("cold alias loses its logical array root")?;
            if !state.get(&(value, None)).copied().unwrap_or(false) {
                return Err(
                    "numeric tree cold continuation requires an older or unavailable array alias",
                );
            }
            if plan.values()[value as usize].schema() != arrays[array].schema {
                return Err("numeric tree cold alias requires unsupported schema rebinding");
            }
            aliases.push(ColdAlias { value, array });
        }
        if !aliases.is_empty() {
            recipes.aliases.push((header, aliases));
        }
        if !scalars.is_empty() {
            recipes.scalars.push((header, scalars));
        }
    }
    Ok(recipes)
}

fn transfer(
    plan: &Plan,
    frames: &NativeBufferFrames,
    provenance: &mut Provenance<'_, '_>,
    site: usize,
    actions: &[(usize, Operation)],
    arrays: &[Array],
    state: &mut State,
) -> Result<(), &'static str> {
    let fresh = |state: &State, key: Key| state.get(&key).copied().unwrap_or(false);
    match actions.iter().find(|(s, _)| *s == site).map(|(_, a)| a) {
        Some(Operation::Frame) => {
            let copied = numeric_fields::transfers(plan, frames, site)?
                .into_iter()
                .filter(|(_, source)| provenance.array(source.0, source.1).is_some())
                .map(|(target, source)| (target, fresh(state, source)))
                .collect::<Vec<_>>();
            state.extend(copied);
        }
        Some(Operation::ArrayCopy) => {
            let Action::Scalar(Scalar::Copy { source, output }) = plan.blocks()[site].action()
            else {
                return Err("tree array copy lost operands");
            };
            let current = fresh(state, (source.get(), None));
            state.insert((output.get(), None), current);
        }
        Some(Operation::Write { array, .. } | Operation::Append { array, .. }) => {
            let Action::LibraryCall { output, .. } = plan.blocks()[site].action() else {
                return Err("tree write lost output");
            };
            let root = arrays[*array].root;
            for (&(value, field), current) in state.iter_mut() {
                if provenance.array(value, field) == Some(root) {
                    *current = false;
                }
            }
            state.insert((output.get(), None), true);
        }
        _ => {}
    }
    Ok(())
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
        arrays.iter().map(|a| (a.entry_key(tail), true)).collect(),
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
            Some(Operation::Frame) => {
                let copied = numeric_fields::transfers(plan, frames, site)?
                    .into_iter()
                    .filter(|(_, source)| provenance.array(source.0, source.1).is_some())
                    .map(|(target, source)| (target, fresh(&state, source)))
                    .collect::<Vec<_>>();
                state.extend(copied);
            }
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
                    Operation::Write { .. } | Operation::Append { .. }
                ) {
                    let root = arrays[*array].root;
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
                let key = match array.storage {
                    super::ArrayStorage::Carrier { field, .. } => (tail.next().get(), Some(field)),
                    super::ArrayStorage::Capture { value } => (value, None),
                    super::ArrayStorage::FrameCapture { value, field, .. } => (value, Some(field)),
                };
                if !fresh(&state, key) {
                    return Err("recursive edge discards or forwards an older array version");
                }
            }
        }
        if site == tail.finish() as usize {
            let current = match result {
                ResultRecipe::Scalar | ResultRecipe::Aggregate { .. } => true,
                ResultRecipe::Array { .. } => fresh(&state, (tail.returned().get(), None)),
                ResultRecipe::Frame { value, arrays } => arrays
                    .iter()
                    .all(|(ordinal, _)| fresh(&state, (*value, Some(*ordinal)))),
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

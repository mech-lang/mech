//! Facts about the current selected key, never about benchmark queries. A new
//! selection or header invalidates predicates; a successful insertion changes
//! absence to presence. Joins keep only facts valid on every incoming path.
use super::*;
use super::packed_maps::LookupFact as Fact;
use tag_threading::{Edge, Graph, Target};
use mech_engine::native_artifact::NativeScalarOp as Op;
use std::collections::VecDeque;
use crate::native_artifact::iteration_compiler::LookupThreading;

const MAX_NODES: usize = 1024;
const MAX_PREDICATES: usize = 128;
// Four existing call/bounds/adjacency modes must still fit their unchanged
// 1024-node admission bound. Unsupported expansion keeps the original graph.
const MAX_THREADED_NODES: usize = 256;

#[cfg(test)]
thread_local! {
    static THREADING_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static THREAD_EMISSIONS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_lookup_threading<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore { fn drop(&mut self) { THREADING_DISABLED.with(|v| v.set(self.0)); } }
    let _restore = Restore(THREADING_DISABLED.with(|v| v.replace(true))); f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_lookup_thread_emissions() -> (usize, usize) {
    THREAD_EMISSIONS.with(|v| v.replace((0, 0)))
}
pub(super) fn note_threading(nodes: usize) {
    #[cfg(test)]
    THREAD_EMISSIONS.with(|v| { let (regions, old) = v.get(); v.set((regions + 1, old + nodes)); });
    #[cfg(not(test))]
    let _ = nodes;
}

#[derive(Clone, Debug)]
enum Expr { Unknown, Copy(usize), Not(usize), Equal(usize, u32) }
impl Expr {
    fn predicate(&self, aliases: &BTreeMap<usize, bool>) -> Option<bool> {
        match *self {
            Self::Unknown => None,
            Self::Copy(word) => aliases.get(&word).copied(),
            Self::Not(word) => aliases.get(&word).map(|v| !v),
            Self::Equal(word, ordinal @ (0 | 1)) => aliases.get(&word).map(|v| *v == (ordinal == 1)),
            Self::Equal(_, _) => None,
        }
    }
}
#[derive(Clone, Debug)]
enum Effect {
    Reset, Select, Contains(usize), Get(usize), Insert,
    Words(Vec<(usize, Expr)>),
}
#[derive(Clone, Debug)]
struct Step { effect: Effect, edge: Edge, condition: Expr }
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct State {
    lookup: Fact,
    // true: this boolean equals current-key presence; false: its inverse.
    aliases: BTreeMap<usize, bool>,
}
impl Default for State {
    fn default() -> Self { Self { lookup: Fact::Unknown, aliases: BTreeMap::new() } }
}
impl State {
    fn apply(&mut self, effect: &Effect) -> Option<()> {
        match effect {
            Effect::Reset | Effect::Select => {
                self.lookup = if matches!(effect, Effect::Reset) { Fact::Unknown } else { Fact::Fresh };
                self.aliases.clear();
            }
            Effect::Contains(output) => {
                if matches!(self.lookup, Fact::Unknown | Fact::Fresh) { self.lookup = Fact::Cached; }
                self.aliases.insert(*output, true);
            }
            Effect::Get(output) => {
                // Only the successful original get can reach its successor.
                self.lookup = Fact::Present;
                self.aliases.remove(output);
            }
            Effect::Insert => {
                // The private helper refreshes the pointer even after growth.
                // Old predicates describe the pre-insert map, so discard them.
                self.lookup = Fact::Present;
                self.aliases.clear();
            }
            Effect::Words(writes) => {
                // Aggregate copies read all old words before writing any new one.
                let values = writes.iter().map(|(word, expr)| (*word, expr.predicate(&self.aliases))).collect::<Vec<_>>();
                for (word, predicate) in values {
                    self.aliases.remove(&word);
                    if let Some(predicate) = predicate { self.aliases.insert(word, predicate); }
                }
            }
        }
        (self.aliases.len() <= MAX_PREDICATES).then_some(())
    }
    fn refine(&mut self, present: bool) -> bool {
        match (self.lookup, present) {
            (Fact::Present, false) | (Fact::Absent, true) => false,
            (Fact::Cached | Fact::Present | Fact::Absent, _) => {
                self.lookup = if present { Fact::Present } else { Fact::Absent }; true
            }
            // No predicate may manufacture a cached entry after invalidation.
            _ => true,
        }
    }
    fn intersect(&mut self, other: &Self) {
        self.lookup = match (self.lookup, other.lookup) {
            (a, b) if a == b => a,
            (Fact::Cached | Fact::Present | Fact::Absent, Fact::Cached | Fact::Present | Fact::Absent) => Fact::Cached,
            _ => Fact::Unknown,
        };
        self.aliases.retain(|word, polarity| other.aliases.get(word) == Some(polarity));
        if matches!(self.lookup, Fact::Unknown | Fact::Fresh) { self.aliases.clear(); }
    }
}

fn solve(steps: &[Step]) -> Option<Vec<Fact>> {
    if steps.is_empty() || steps.len() > MAX_NODES { return None; }
    let targets = |edge: &Edge| -> Vec<Target> { match edge {
        Edge::Jump(next) => vec![*next],
        Edge::Branch { yes, no } => vec![*yes, *no],
        Edge::Dispatch { cases, other } => cases.iter().map(|(_, target)| *target).chain([*other]).collect(),
        Edge::Reject => Vec::new(),
    }};
    for step in steps {
        if targets(&step.edge).iter().any(|t| matches!(t, Target::Node(n) if *n >= steps.len())) { return None; }
    }
    let mut incoming = vec![None; steps.len()];
    incoming[0] = Some(State::default());
    let mut pending = VecDeque::from([0]);
    let mut queued = vec![false; steps.len()]; queued[0] = true;
    let mut visits = 0usize;
    while let Some(node) = pending.pop_front() {
        queued[node] = false;
        visits += 1;
        if visits > MAX_NODES * (MAX_PREDICATES + 4) { return None; }
        let step = &steps[node];
        let mut after = incoming[node].clone()?;
        after.apply(&step.effect)?;
        let predicate = step.condition.predicate(&after.aliases);
        for (edge_index, next) in targets(&step.edge).into_iter().enumerate() {
            let Target::Node(next) = next else { continue; };
            let mut state = after.clone();
            if matches!(step.edge, Edge::Branch { .. }) {
                if let Some(polarity) = predicate {
                    if !state.refine(polarity == (edge_index == 0)) { continue; }
                }
            }
            let changed = if let Some(old) = &mut incoming[next] {
                let before = old.clone(); old.intersect(&state); *old != before
            } else { incoming[next] = Some(state); true };
            if changed && !queued[next] { pending.push_back(next); queued[next] = true; }
        }
    }
    Some(incoming.into_iter().map(|state| state.map_or(Fact::Unknown, |s| s.lookup)).collect())
}

fn aggregate_effect(layout: &NativeScalarAggregates, action: &Aggregate) -> Option<Effect> {
    let copy = |value, index| field(layout, value, index).map(Expr::Copy);
    let (output, values) = match action {
        Aggregate::Pack { output, words } => (*output, words.iter().map(|word| Some(match word {
            Word::Scalar(value) => Expr::Copy(value.get() as usize),
            Word::Field(value, index) => copy(*value, *index)?,
            Word::Constant(_) => Expr::Unknown,
        })).collect::<Option<Vec<_>>>()?),
        Aggregate::Copy { source, output } => (*output,
            (0..layout.slot(*output)?.1).map(|i| copy(*source, i)).collect::<Option<Vec<_>>>()?),
        Aggregate::Project { source, offset, output, .. } => (*output,
            (0..layout.slot(*output)?.1).map(|i| copy(*source, offset.checked_add(i)?)).collect::<Option<Vec<_>>>()?),
        Aggregate::Observe { source, offset, ordinal, output, .. } => {
            let word = field(layout, *source, *offset)?;
            let expr = ordinal.map_or(Expr::Copy(word), |ordinal| Expr::Equal(word, ordinal));
            return Some(Effect::Words(vec![(output.get() as usize, expr)]));
        }
    };
    let (base, count) = layout.slot(output)?;
    if values.len() != count { return None; }
    Some(Effect::Words(values.into_iter().enumerate().map(|(i, expr)| (base + i, expr)).collect()))
}

fn steps(code: CodePlan<'_>, closed: &Layout, id: usize,
    owner: &NativeMapOwnerWords, aggregates: &NativeScalarAggregates, graph: &Graph) -> Option<Vec<Step>>
{
    let region = &closed.regions[id];
    region.packed.as_ref()?;
    let owner_index = region.map?;
    let mut constants = BTreeMap::new();
    for i in 0..code.constant_count() {
        let (value, word) = code.constant(i).ok()?;
        if let Some(word) = word { constants.insert(value.get() as usize, word); }
    }
    let mut steps = Vec::new();
    for node in &graph.nodes {
        let site = node.site;
        if closed.sites.get(site) != Some(&Some(id)) { return None; }
        let block = &code.blocks()[site];
        let effect = if site == region.tail.header() as usize {
            if !matches!(block.action(), Action::Scalar(Scalar::None)) { return None; }
            // Refill can clear the cache, including on a terminal traversal.
            Effect::Reset
        } else if let Some((layout, action)) = aggregate_at(owner, aggregates, owner_index, site) {
            aggregate_effect(layout, action)?
        } else if let Some(span) = owner.span_site(site).map(|i| &owner.spans()[i]) {
            if span.region() != owner_index { return None; }
            if span.range_site() as usize == site { Effect::Select } else { Effect::Words(Vec::new()) }
        } else if owner.call_region(site) == Some(owner_index) {
            let Action::LibraryCall { template, output, .. } = block.action() else { return None; };
            match template.primitive() {
                P::MapContainsKey => Effect::Contains(output.get() as usize),
                P::MapKeyAccess => Effect::Get(output.get() as usize),
                P::MapInsert => Effect::Insert,
                _ => return None,
            }
        } else { match block.action() {
            Action::Scalar(Scalar::None) => Effect::Words(Vec::new()),
            Action::Scalar(Scalar::Copy { source, output }) => Effect::Words(vec![(output.get() as usize, Expr::Copy(source.get() as usize))]),
            Action::Scalar(Scalar::Primitive { op, operands, output, .. }) => {
                let expr = if *op == Op::BoolNot && operands.len() == 1 { Expr::Not(operands[0].get() as usize) } else { Expr::Unknown };
                Effect::Words(vec![(output.get() as usize, expr)])
            }
            _ => return None,
        }};
        let condition = match block.terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => Expr::Copy(condition.get() as usize),
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) if code.kind(left)? != NativeScalarKind::F64 => {
                if let Some(word @ (0 | 1)) = constants.get(&(right.get() as usize)) {
                    Expr::Equal(left.get() as usize, *word as u32)
                } else if let Some(word @ (0 | 1)) = constants.get(&(left.get() as usize)) {
                    Expr::Equal(right.get() as usize, *word as u32)
                } else { Expr::Unknown }
            }
            _ => Expr::Unknown,
        };
        steps.push(Step { effect, condition, edge: node.edge.clone() });
    }
    Some(steps)
}

pub(super) fn derive(code: CodePlan<'_>, closed: &Layout, id: usize,
    owner: &NativeMapOwnerWords, aggregates: &NativeScalarAggregates, graph: &Graph) -> Option<Vec<Fact>>
{
    solve(&steps(code, closed, id, owner, aggregates, graph)?)
}

struct ThreadBuilder {
    keys: Vec<(usize, State)>,
    interned: BTreeMap<(usize, State), usize>,
}
impl ThreadBuilder {
    fn target(&mut self, target: Target, mut state: State, steps: &[Step]) -> Option<Target> {
        let Target::Node(original) = target else { return Some(Target::Exit); };
        // No current-key predicate survives a selection or a header/refill.
        // Canonicalize BEFORE interning so old cache state cannot duplicate
        // headers or leak across a new source range.
        if matches!(steps.get(original)?.effect, Effect::Reset | Effect::Select) { state = State::default(); }
        let key = (original, state);
        if let Some(&node) = self.interned.get(&key) { return Some(Target::Node(node)); }
        if self.keys.len() == MAX_THREADED_NODES { return None; }
        let node = self.keys.len();
        self.keys.push(key.clone());
        self.interned.insert(key, node);
        Some(Target::Node(node))
    }
}

/// A bounded product of the actual source CFG and current-key facts. Keeping
/// the two incoming states separate at a join does not move or fuse a source
/// action: each version emits that exact action and its original successor.
/// Insertion refreshes the private entry after growth; selection/header clears
/// all old aliases. Unknown shapes keep the existing intersecting analysis.
fn thread_steps(graph: &Graph, steps: &[Step]) -> Option<(Graph, LookupThreading)> {
    if graph.nodes.is_empty() || graph.nodes.len() > MAX_THREADED_NODES || graph.nodes.len() != steps.len() { return None; }
    let old_facts = solve(steps)?;
    let mut builder = ThreadBuilder { keys: Vec::new(), interned: BTreeMap::new() };
    builder.target(Target::Node(0), State::default(), steps)?;
    let mut nodes = Vec::new();
    let mut states = Vec::new();
    let mut folded = Vec::new();
    let mut improved = false;
    while nodes.len() < builder.keys.len() {
        let (original, mut state) = builder.keys[nodes.len()].clone();
        let source = &graph.nodes[original];
        let step = &steps[original];
        improved |= old_facts[original] == Fact::Cached && matches!(state.lookup, Fact::Present | Fact::Absent);
        states.push((nodes.len(), original, source.site, state.lookup.name()));
        state.apply(&step.effect)?;
        let edge = match &step.edge {
            Edge::Jump(next) => Edge::Jump(builder.target(*next, state, steps)?),
            Edge::Reject => Edge::Reject,
            Edge::Dispatch { cases, other } => {
                let cases = cases.iter().map(|&(tag, next)| Some((tag, builder.target(next, state.clone(), steps)?)))
                    .collect::<Option<Vec<_>>>()?;
                Edge::Dispatch { cases, other: builder.target(*other, state, steps)? }
            }
            Edge::Branch { yes, no } => {
                let predicate = step.condition.predicate(&state.aliases);
                let mut branch = |next, truth| {
                    let mut selected = state.clone();
                    let possible = predicate.is_none_or(|polarity| selected.refine(polarity == truth));
                    if possible { Some(Some(builder.target(next, selected, steps)?)) } else { Some(None) }
                };
                let yes = branch(*yes, true)?;
                let no = branch(*no, false)?;
                match (yes, no) {
                    (Some(yes), Some(no)) => Edge::Branch { yes, no },
                    (Some(next), None) | (None, Some(next)) => {
                        folded.push((nodes.len(), source.site)); Edge::Jump(next)
                    }
                    (None, None) => return None,
                }
            }
        };
        nodes.push(tag_threading::Node { site: source.site, edge, header_tag: source.header_tag,
            folded_branch: source.folded_branch, known_latch: source.known_latch });
    }
    if !improved { return None; }
    let receipt = LookupThreading {
        original_tag_nodes: graph.nodes.len(), states, folded_predicate_branches: folded,
    };
    Some((Graph { carrier_word: graph.carrier_word, variants: graph.variants.clone(), nodes }, receipt))
}

pub(super) fn thread(code: CodePlan<'_>, closed: &Layout, id: usize,
    owner: &NativeMapOwnerWords, aggregates: &NativeScalarAggregates, graph: &Graph)
    -> Option<(Graph, LookupThreading)>
{
    #[cfg(test)]
    if THREADING_DISABLED.with(std::cell::Cell::get) { return None; }
    thread_steps(graph, &steps(code, closed, id, owner, aggregates, graph)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn step(effect: Effect, next: usize) -> Step {
        Step { effect, edge: Edge::Jump(Target::Node(next)), condition: Expr::Unknown }
    }
    fn graph(steps: &[Step]) -> Graph {
        Graph { carrier_word: 0, variants: vec![0, 1], nodes: steps.iter().enumerate()
            .map(|(site, step)| tag_threading::Node { site, edge: step.edge.clone(), header_tag: None,
                folded_branch: false, known_latch: false }).collect() }
    }
    #[test]
    fn lookup_threading_keeps_join_states_separate_but_discards_stale_predicates() {
        let mut source = vec![step(Effect::Reset, 1), step(Effect::Select, 2), step(Effect::Contains(10), 3),
            step(Effect::Words(Vec::new()), 4), step(Effect::Get(12), 6), step(Effect::Words(Vec::new()), 6),
            step(Effect::Insert, 7), step(Effect::Words(Vec::new()), 8), step(Effect::Select, 9),
            step(Effect::Words(Vec::new()), 10), step(Effect::Words(Vec::new()), 12),
            step(Effect::Words(Vec::new()), 12), step(Effect::Reset, 0)];
        source[3].edge = Edge::Branch { yes: Target::Node(4), no: Target::Node(5) };
        source[3].condition = Expr::Copy(10);
        // This branch uses the saved OLD-key predicate after insertion and a
        // new selection. It cannot establish presence for the new key.
        source[9].edge = Edge::Branch { yes: Target::Node(10), no: Target::Node(11) };
        source[9].condition = Expr::Copy(10);
        assert_eq!(solve(&source).unwrap()[6], Fact::Cached);
        let (threaded, receipt) = thread_steps(&graph(&source), &source).unwrap();
        let facts_at = |site| receipt.states.iter().filter(|state| state.2 == site)
            .map(|state| state.3).collect::<BTreeSet<_>>();
        assert_eq!(facts_at(6), BTreeSet::from(["Present", "Absent"]));
        assert_eq!(facts_at(7), BTreeSet::from(["Present"]));
        assert_eq!(facts_at(10), BTreeSet::from(["Fresh"]));
        assert_eq!(facts_at(11), BTreeSet::from(["Fresh"]));
        for &site in &[0, 12] { assert_eq!(facts_at(site), BTreeSet::from(["Unknown"])); }
        for node in threaded.nodes.iter().filter(|node| node.site == 9) {
            assert!(matches!(node.edge, Edge::Branch { .. }), "stale predicate was folded");
        }
        // Every emitted node still performs precisely its original action;
        // solve the expanded graph again independently by intersection.
        let expanded = receipt.states.iter().zip(&threaded.nodes).map(|(state, node)| {
            let mut step = source[state.1].clone(); step.edge = node.edge.clone(); step
        }).collect::<Vec<_>>();
        let checked = solve(&expanded).unwrap();
        for &(node, _, site, fact) in &receipt.states {
            if site == 6 { assert_eq!(checked[node].name(), fact); }
        }
    }
    #[test]
    fn lookup_threading_declines_expansion_beyond_the_existing_four_mode_bound() {
        let mut source = (0..MAX_THREADED_NODES).map(|n| step(Effect::Words(Vec::new()), n + 1)).collect::<Vec<_>>();
        source[0].effect = Effect::Select;
        source[1].effect = Effect::Contains(10);
        source[2].condition = Expr::Copy(10);
        source[2].edge = Edge::Branch { yes: Target::Node(3), no: Target::Node(4) };
        source[3].edge = Edge::Jump(Target::Node(4));
        source.last_mut().unwrap().edge = Edge::Jump(Target::Exit);
        assert!(thread_steps(&graph(&source), &source).is_none());
    }
    #[test]
    fn lookup_facts_follow_membership_aliases_and_invalidate_on_insert_selection_and_headers() {
        let mut nodes = vec![step(Effect::Reset, 1), step(Effect::Select, 2), step(Effect::Contains(10), 3),
            step(Effect::Words(vec![(11, Expr::Not(10))]), 4), step(Effect::Words(Vec::new()), 5),
            step(Effect::Get(12), 7), step(Effect::Insert, 7), step(Effect::Select, 8),
            step(Effect::Contains(13), 9), step(Effect::Reset, 10), step(Effect::Words(Vec::new()), 0)];
        nodes[4].edge = Edge::Branch { yes: Target::Node(6), no: Target::Node(5) };
        nodes[4].condition = Expr::Copy(11);
        let facts = solve(&nodes).unwrap();
        assert_eq!((facts[2], facts[5], facts[6], facts[7], facts[8], facts[10]),
            (Fact::Fresh, Fact::Present, Fact::Absent, Fact::Present, Fact::Fresh, Fact::Unknown));
        // A saved predicate cannot describe a newly selected key or a map
        // after insertion. Neither branch may manufacture presence there.
        for invalidation in [Effect::Select, Effect::Insert, Effect::Reset] {
            let mut altered = nodes.clone(); altered[3].effect = invalidation;
            let facts = solve(&altered).unwrap();
            assert_eq!(facts[5], facts[6]);
        }
    }
    #[test]
    fn lookup_facts_join_hits_and_misses_without_assuming_a_bypassed_lookup() {
        let mut nodes = vec![step(Effect::Select, 1), step(Effect::Contains(10), 2),
            step(Effect::Words(Vec::new()), 3), step(Effect::Words(Vec::new()), 5),
            step(Effect::Words(Vec::new()), 5), step(Effect::Words(Vec::new()), 0)];
        nodes[2].edge = Edge::Branch { yes: Target::Node(3), no: Target::Node(4) };
        nodes[2].condition = Expr::Equal(10, 1);
        assert_eq!(solve(&nodes).unwrap()[5], Fact::Cached);
        nodes[0].edge = Edge::Branch { yes: Target::Node(1), no: Target::Node(5) };
        assert_eq!(solve(&nodes).unwrap()[5], Fact::Unknown);
        // A copy cycle is simultaneous; swapping a predicate into another
        // slot must not make both slots aliases of it.
        let mut state = State { lookup: Fact::Cached, aliases: BTreeMap::from([(1, true)]) };
        state.apply(&Effect::Words(vec![(1, Expr::Copy(2)), (2, Expr::Copy(1))])).unwrap();
        assert_eq!(state.aliases, BTreeMap::from([(2, true)]));
        nodes[0].edge = Edge::Jump(Target::Node(MAX_NODES + 1));
        assert!(solve(&nodes).is_none());
    }
}

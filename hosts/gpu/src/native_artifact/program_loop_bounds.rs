//! A bounded, source-derived certificate for a guarded unsigned window loop.
//! This does not replace the body or change its accounting. The original tag
//! graph remains the fallback; only a runtime-guarded copy may use this proof.
use super::*;
use tag_threading::{Edge, Graph, Target};
use mech_engine::native_artifact::NativeScalarOp as Op;

const MAX_NODES: usize = 512; // Two versions must fit the existing 1024-node bound.
const MAX_TERMS: usize = 4096;
const MAX_PATHS: usize = 64;
type Id = usize;
type State = BTreeMap<usize, Id>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Expr { Input(usize), Constant(u64), Add(Id, Id), Sub(Id, Id), Eq(Id, Id), Ge(Id, Id), Gt(Id, Id), Opaque(usize) }
#[derive(Default)]
struct Terms { values: Vec<Expr>, ids: BTreeMap<Expr, Id> }
impl Terms {
    fn intern(&mut self, value: Expr) -> Option<Id> {
        if let Some(&id) = self.ids.get(&value) { return Some(id); }
        if self.values.len() == MAX_TERMS { return None; }
        let id = self.values.len(); self.values.push(value.clone()); self.ids.insert(value, id); Some(id)
    }
    fn constant(&mut self, value: u64) -> Option<Id> { self.intern(Expr::Constant(value)) }
    fn value(&self, id: Id) -> Option<u64> { match self.values.get(id)? { Expr::Constant(n) => Some(*n), _ => None } }
    fn add(&mut self, a: Id, b: Id) -> Option<Id> { self.intern(Expr::Add(a.min(b), a.max(b))) }
}

pub(super) struct Proof {
    pub index_word: usize,
    pub width_word: usize,
    pub limit_word: usize,
    pub active_tag: u32,
    pub first: usize,
    pub later: [usize; 2],
    pub ranges: BTreeSet<usize>,
    pub guard_sites: [usize; 3],
    pub range_site: usize,
    pub recurrence_sites: Vec<usize>,
    pub recurrence_nodes: BTreeSet<usize>,
    // Every productive recurrence inserts exactly once, no productive prefix
    // inserts twice, and other state variants cannot enter the active cycle.
    // Only these actual product-graph nodes may use the mutation relation.
    pub single_insert_nodes: Option<BTreeSet<usize>>,
}

#[derive(Default)]
struct Effects { uses: BTreeSet<usize>, writes: BTreeSet<usize> }
fn effects(code: CodePlan<'_>, closed: &Layout, id: usize, owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates, site: usize) -> Option<Effects>
{
    let mut result = Effects::default();
    if let Some((layout, action)) = aggregate_at(owner, aggregates, closed.regions[id].map?, site) {
        let field_at = |value, index| field(layout, value, index);
        let (output, count) = match action {
            Aggregate::Pack { output, words } => {
                for word in words { match word {
                    Word::Scalar(v) => { result.uses.insert(v.get() as usize); }
                    Word::Field(v, i) => { result.uses.insert(field_at(*v, *i)?); }
                    Word::Constant(_) => {}
                }}
                (*output, words.len())
            }
            Aggregate::Copy { source, output } => {
                let count = layout.slot(*output)?.1;
                for i in 0..count { result.uses.insert(field_at(*source, i)?); }
                (*output, count)
            }
            Aggregate::Project { source, offset, guards, output } => {
                for &(i, _) in guards { result.uses.insert(field_at(*source, i)?); }
                let count = layout.slot(*output)?.1;
                for i in 0..count { result.uses.insert(field_at(*source, offset.checked_add(i)?)?); }
                (*output, count)
            }
            Aggregate::Observe { source, offset, guards, output, .. } => {
                for &(i, _) in guards { result.uses.insert(field_at(*source, i)?); }
                result.uses.insert(field_at(*source, *offset)?);
                result.writes.insert(output.get() as usize);
                return Some(result);
            }
        };
        for i in 0..count { result.writes.insert(field_at(output, i)?); }
    } else if let Some(span) = owner.span_site(site).map(|i| &owner.spans()[i]) {
        if span.region() != closed.regions[id].map? { return None; }
        if span.range_site() as usize == site {
            result.uses.extend(span.operands().iter().map(|v| v.get() as usize));
        }
    } else { match code.blocks()[site].action() {
        Action::Scalar(Scalar::None) => {}
        Action::Scalar(Scalar::Copy { source, output }) => {
            result.uses.insert(source.get() as usize); result.writes.insert(output.get() as usize);
        }
        Action::Scalar(Scalar::Primitive { operands, output, .. }) => {
            result.uses.extend(operands.iter().map(|v| v.get() as usize)); result.writes.insert(output.get() as usize);
        }
        Action::LibraryCall { template, operands, output } if owner.call_region(site) == closed.regions[id].map
            && matches!(template.primitive(), P::MapContainsKey | P::MapKeyAccess | P::MapInsert) => {
            result.uses.insert(operands[0].get() as usize);
            if template.primitive() == P::MapInsert { result.uses.insert(operands[2].get() as usize); }
            result.writes.insert(output.get() as usize);
        }
        _ => return None,
    }}
    Some(result)
}

// Liveness is over the ACTUAL tag product graph, including its unknown-tag
// fallback. It prevents bypassing a definition that a later visit can observe.
fn live_in(code: CodePlan<'_>, graph: &Graph, effects: &[Effects], returned: usize) -> Option<Vec<BTreeSet<usize>>> {
    let mut live = vec![BTreeSet::new(); graph.nodes.len()];
    for _ in 0..64 {
        let mut changed = false;
        for (id, node) in graph.nodes.iter().enumerate().rev() {
            let mut next = BTreeSet::new();
            let mut edge = |target| match target { Target::Exit => { next.insert(returned); },
                Target::Node(id) => next.extend(live[id].iter().copied()) };
            match &node.edge {
                Edge::Jump(target) => edge(*target),
                Edge::Branch { yes, no } => {
                    edge(*yes); edge(*no);
                    match code.blocks()[node.site].terminator() {
                        Term::Scalar(Branch::Branch { condition, .. }) => { next.insert(condition.get() as usize); }
                        Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                            next.insert(left.get() as usize); next.insert(right.get() as usize);
                        }
                        _ => return None,
                    }
                }
                Edge::Dispatch { cases, other } => {
                    for &(_, target) in cases { edge(target); } edge(*other);
                    next.insert(graph.carrier_word);
                }
                Edge::Reject => {}
            }
            // Branch operands are read AFTER the action's writes, while its
            // own operands use the old values (parallel aggregate copies).
            for word in &effects[id].writes { next.remove(word); }
            next.extend(effects[id].uses.iter().copied());
            if live[id] != next { live[id] = next; changed = true; }
        }
        if !changed { return Some(live); }
    }
    None
}

fn apply(code: CodePlan<'_>, closed: &Layout, id: usize, owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates, site: usize, state: &mut State, terms: &mut Terms,
    pure: bool) -> Option<()>
{
    let read = |state: &State, word| state.get(&word).copied();
    if let Some((layout, action)) = aggregate_at(owner, aggregates, closed.regions[id].map?, site) {
        let at = |value, index| field(layout, value, index);
        let (output, values) = match action {
            Aggregate::Pack { output, words } => {
                let values = words.iter().map(|word| match word {
                    Word::Scalar(v) => read(state, v.get() as usize),
                    Word::Field(v, i) => read(state, at(*v, *i)?),
                    Word::Constant(n) => terms.constant(*n),
                }).collect::<Option<Vec<_>>>()?;
                (*output, values)
            }
            Aggregate::Copy { source, output } => (*output, (0..layout.slot(*output)?.1)
                .map(|i| read(state, at(*source, i)?)).collect::<Option<Vec<_>>>()?),
            Aggregate::Project { source, offset, guards, output } => {
                for &(i, ordinal) in guards {
                    if terms.value(read(state, at(*source, i)?)?) != Some(u64::from(ordinal)) { return None; }
                }
                (*output, (0..layout.slot(*output)?.1).map(|i|
                    read(state, at(*source, offset.checked_add(i)?)?)).collect::<Option<Vec<_>>>()?)
            }
            Aggregate::Observe { source, offset, ordinal, guards, output } => {
                for &(i, ordinal) in guards {
                    if terms.value(read(state, at(*source, i)?)?) != Some(u64::from(ordinal)) { return None; }
                }
                let input = read(state, at(*source, *offset)?)?;
                let value = if let Some(ordinal) = ordinal {
                    let value = terms.value(input)?; terms.constant(u64::from(value == u64::from(*ordinal)))?
                } else { input };
                state.insert(output.get() as usize, value); return Some(());
            }
        };
        for (i, value) in values.into_iter().enumerate() { state.insert(at(output, i)?, value); }
        return Some(());
    }
    if owner.span_site(site).is_some() { return (!pure).then_some(()); }
    match code.blocks()[site].action() {
        Action::Scalar(Scalar::None) => {}
        Action::Scalar(Scalar::Copy { source, output }) => {
            state.insert(output.get() as usize, read(state, source.get() as usize)?);
        }
        Action::Scalar(Scalar::Primitive { op, operands, output }) => {
            let args = operands.iter().map(|v| read(state, v.get() as usize)).collect::<Option<Vec<_>>>()?;
            let value = match (*op, args.as_slice()) {
                (Op::U64Eq, &[a, b]) => terms.intern(Expr::Eq(a, b))?,
                (Op::U64Ge, &[a, b]) => terms.intern(Expr::Ge(a, b))?,
                (Op::U64Gt, &[a, b]) => terms.intern(Expr::Gt(a, b))?,
                (Op::U64Sub, &[a, b]) => terms.intern(Expr::Sub(a, b))?,
                (Op::U64Add, &[a, b]) if !pure => terms.add(a, b)?,
                _ if !pure => terms.intern(Expr::Opaque(site))?,
                _ => return None,
            };
            state.insert(output.get() as usize, value);
        }
        Action::LibraryCall { output, .. } if !pure && owner.call_region(site) == closed.regions[id].map => {
            state.insert(output.get() as usize, terms.intern(Expr::Opaque(site))?);
        }
        _ => return None,
    }
    Some(())
}

struct Walk<'a> {
    code: CodePlan<'a>, closed: &'a Layout, id: usize, owner: &'a NativeMapOwnerWords,
    aggregates: &'a NativeScalarAggregates, graph: &'a Graph,
}
impl Walk<'_> {
    fn step(&self, node: usize, state: &mut State, terms: &mut Terms, pure: bool) -> Option<()> {
        apply(self.code, self.closed, self.id, self.owner, self.aggregates, self.graph.nodes[node].site, state, terms, pure)
    }
    // Stop before the next non-tag branch; source actions, including any
    // checked subtraction, are retained and separately certified below.
    fn branch(&self, mut node: usize, state: &mut State, terms: &mut Terms,
        seen: &mut BTreeSet<usize>, subtract: bool) -> Option<usize>
    {
        for _ in 0..self.graph.nodes.len() {
            if !seen.insert(node) { return None; }
            if !subtract && matches!(self.code.blocks()[self.graph.nodes[node].site].action(),
                Action::Scalar(Scalar::Primitive { op: Op::U64Sub, .. })) { return None; }
            self.step(node, state, terms, true)?;
            match self.graph.nodes[node].edge {
                Edge::Jump(Target::Node(next)) => node = next,
                Edge::Branch { .. } => return Some(node),
                _ => return None,
            }
        }
        None
    }
    fn stop(&self, mut node: usize, mut state: State, terms: &mut Terms) -> Option<(usize, State)> {
        let latch = self.closed.regions[self.id].tail.latch() as usize;
        for _ in 0..self.graph.nodes.len() {
            // Stop packs/copies are total. No primitive, source call, source
            // failure or unrelated observable action may be jumped over.
            let site = self.graph.nodes[node].site;
            if aggregate_at(self.owner, self.aggregates, self.closed.regions[self.id].map?, site).is_none()
                && !matches!(self.code.blocks()[site].action(), Action::Scalar(Scalar::None | Scalar::Copy { .. })) { return None; }
            self.step(node, &mut state, terms, true)?;
            let Edge::Jump(Target::Node(next)) = self.graph.nodes[node].edge else { return None; };
            if site == latch { return Some((next, state)); }
            node = next;
        }
        None
    }
}
fn condition(code: CodePlan<'_>, graph: &Graph, node: usize, state: &State) -> Option<Id> {
    let Term::Scalar(Branch::Branch { condition, .. }) = code.blocks()[graph.nodes[node].site].terminator() else { return None; };
    state.get(&(condition.get() as usize)).copied()
}
fn arms(graph: &Graph, node: usize) -> Option<(usize, usize)> {
    let Edge::Branch { yes: Target::Node(yes), no: Target::Node(no) } = graph.nodes[node].edge else { return None; };
    Some((yes, no))
}
fn same_live(a: &State, b: &State, live: &BTreeSet<usize>) -> bool {
    live.iter().all(|word| a.get(word).is_some() && a.get(word) == b.get(word))
}

fn targets(edge: &Edge) -> Vec<Target> {
    match edge {
        Edge::Jump(target) => vec![*target],
        Edge::Branch { yes, no } => vec![*yes, *no],
        Edge::Dispatch { cases, other } => cases.iter().map(|(_, target)| *target)
            .chain(core::iter::once(*other)).collect(),
        Edge::Reject => Vec::new(),
    }
}

fn closed_active_recurrence(graph: &Graph, header: usize, active_tag: u32,
    recurrences: &BTreeSet<usize>) -> bool
{
    graph.nodes.iter().enumerate().all(|(node, source)| targets(&source.edge).into_iter().all(|target| {
        let Target::Node(next) = target else { return true; };
        let Some(next) = graph.nodes.get(next) else { return false; };
        next.site != header || matches!(next.header_tag, Some(tag) if tag != active_tag)
            || recurrences.contains(&node)
    }))
}

// Start a fresh reachability search at EVERY header version, including the
// actual-tag dispatcher and Other partition. A previous traversal's guard is
// therefore never evidence for the current traversal. Removing `cut` must
// disconnect every certified node, regardless of unrelated incoming joins.
fn dominates_this_visit(graph: &Graph, header_site: usize, cut: usize,
    certified: &BTreeSet<usize>) -> bool
{
    if graph.nodes.len() > MAX_NODES || cut >= graph.nodes.len()
        || certified.iter().any(|&node| node >= graph.nodes.len()) { return false; }
    let mut pending = graph.nodes.iter().enumerate().filter_map(|(id, node)|
        (node.site == header_site).then_some(id)).collect::<Vec<_>>();
    if pending.is_empty() { return false; }
    let mut visited = vec![false; graph.nodes.len()];
    while let Some(id) = pending.pop() {
        if id == cut || visited[id] { continue; }
        if certified.contains(&id) { return false; }
        visited[id] = true;
        for target in targets(&graph.nodes[id].edge) {
            if let Target::Node(next) = target {
                if next >= graph.nodes.len() { return false; }
                if !visited[next] { pending.push(next); }
            }
        }
    }
    true
}

fn guarded_entry_dominates(graph: &Graph, header_site: usize, active_tag: u32,
    body: usize, first: usize, certified: &BTreeSet<usize>) -> bool
{
    if body >= graph.nodes.len() || graph.nodes[body].site == header_site { return false; }
    // Symbolic prefix analysis starts with an exact active tag. Only the
    // corresponding known header or actual dispatcher arm may establish it;
    // an ordinary edge into this prefix is not an equivalent entry witness.
    for node in &graph.nodes {
        match &node.edge {
            Edge::Jump(Target::Node(next)) if *next == body => {
                if node.site != header_site || node.header_tag != Some(active_tag) { return false; }
            }
            Edge::Dispatch { cases, other } => {
                if *other == Target::Node(body) { return false; }
                for &(tag, target) in cases {
                    if target == Target::Node(body) && (node.site != header_site || tag != active_tag) { return false; }
                }
            }
            edge if targets(edge).contains(&Target::Node(body)) => return false,
            _ => {}
        }
    }
    dominates_this_visit(graph, header_site, body, &BTreeSet::from([first]))
        && dominates_this_visit(graph, header_site, first, certified)
}

#[cfg(feature = "canonical-native-program")]
pub(super) fn derive(code: CodePlan<'_>, closed: &Layout, id: usize, owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates, graph: &Graph, slots: &BTreeSet<usize>) -> Option<Proof>
{
    let CodePlan::Program(plan) = code else { return None; };
    if graph.nodes.len() > MAX_NODES || slots.len() > 512 { return None; }
    let region = &closed.regions[id]; let packed = region.packed.as_ref()?;
    if !packed.inclusive() { return None; }
    let width_word = packed.end_offset()? as usize;
    let start_word = packed.start_offset()? as usize;
    let visits = super::super::super::iteration_abi::control_visits(plan.blocks(), plan.iterations().iter().map(Into::into), 0).ok()?;
    let effects = graph.nodes.iter().map(|node| effects(code, closed, id, owner, aggregates, node.site))
        .collect::<Option<Vec<_>>>()?;
    let live = live_in(code, graph, &effects, region.tail.returned().get() as usize)?;
    let walk = Walk { code, closed, id, owner, aggregates, graph };
    let mut terms = Terms::default(); let mut initial = State::new();
    for &slot in slots { initial.insert(slot, terms.intern(Expr::Input(slot))?); }
    for i in 0..code.constant_count() { let (v, word) = code.constant(i).ok()?;
        if let Some(word) = word && slots.contains(&(v.get() as usize)) { initial.insert(v.get() as usize, terms.constant(word)?); }
    }
    let one = terms.constant(1)?; let zero = terms.constant(0)?;
    if initial.get(&start_word) != Some(&one) { return None; }
    let width = *initial.get(&width_word)?;
    for (header, entry) in graph.nodes.iter().enumerate() {
        let Some(active_tag) = entry.header_tag else { continue; };
        let mut state = initial.clone(); state.insert(graph.carrier_word, terms.constant(u64::from(active_tag))?);
        let mut chain = BTreeSet::new();
        let Some(first) = walk.branch(header, &mut state, &mut terms, &mut chain, false) else { continue; };
        let Some(test) = condition(code, graph, first, &state) else { continue; };
        if !matches!(terms.values[test], Expr::Eq(a,b) if (a==width && b==zero)||(b==width && a==zero)) { continue; }
        let Action::Scalar(Scalar::Primitive { op: Op::U64Eq, operands, .. }) = code.blocks()[graph.nodes[first].site].action() else { return None; };
        let width_value = *operands.iter().find(|v| v.get() as usize == width_word)?;
        if !super::super::super::program_map_packed_layout::invariant(plan, &visits, width_value, region.iteration) { return None; }
        let (stop0, next) = arms(graph, first)?;
        let (end0, state0) = walk.stop(stop0, state.clone(), &mut terms)?;
        let second = walk.branch(next, &mut state, &mut terms, &mut chain, false)?;
        let Expr::Ge(index, limit) = terms.values[condition(code, graph, second, &state)?] else { return None; };
        let Expr::Input(index_word) = terms.values[index] else { return None; };
        let (carrier, count) = owner.aggregates().slot(region.tail.carrier())?;
        if index_word <= carrier || index_word >= carrier.checked_add(count)? { return None; }
        let Action::Scalar(Scalar::Primitive { op: Op::U64Ge, operands, .. }) = code.blocks()[graph.nodes[second].site].action() else { return None; };
        let limit_value = *operands.get(1)?;
        if initial.get(&(limit_value.get() as usize)) != Some(&limit)
            || !super::super::super::program_map_packed_layout::invariant(plan, &visits, limit_value, region.iteration) { return None; }
        let (stop1, next) = arms(graph, second)?;
        let (end1, state1) = walk.stop(stop1, state.clone(), &mut terms)?;
        let third = walk.branch(next, &mut state, &mut terms, &mut chain, true)?;
        let difference = terms.intern(Expr::Sub(limit, index))?;
        if terms.values[condition(code, graph, third, &state)?] != Expr::Gt(width, difference) { return None; }
        // Every potentially failing scalar skipped by the normalized stop is
        // exactly n-i after the original i<n guard. Other arithmetic declines.
        for &node in &chain {
            if let Action::Scalar(Scalar::Primitive { op: Op::U64Sub, output, .. }) = code.blocks()[graph.nodes[node].site].action() {
                if state.get(&(output.get() as usize)) != Some(&difference) { return None; }
            }
        }
        let (stop2, productive) = arms(graph, third)?;
        let (end2, state2) = walk.stop(stop2, state.clone(), &mut terms)?;
        if end0 != end1 || end0 != end2 || graph.nodes[end0].header_tag == Some(active_tag) { return None; }
        // Compare every live word at the next header, not only the map token.
        if !same_live(&state0, &state1, &live[end0]) || !same_live(&state0, &state2, &live[end0]) { return None; }
        let next_index = terms.add(index, one)?;
        let range_end = terms.add(index, width)?;
        let mut pending = vec![(productive, state, false, 0usize)]; let mut paths = 0usize;
        let mut steps = 0usize; let mut ranges = BTreeSet::new(); let mut recurrence_sites = BTreeSet::new();
        let mut recurrence_nodes = BTreeSet::new(); let mut insert_nodes = BTreeSet::new();
        let mut single_insert = true;
        while let Some((mut node, mut state, mut selected, mut insertions)) = pending.pop() {
            loop {
                steps = steps.checked_add(1)?; if steps > MAX_PATHS * MAX_NODES { return None; }
                let site = graph.nodes[node].site;
                if site == region.tail.header() as usize { return None; }
                if site == packed.range_site() as usize {
                    if selected { return None; }
                    let span = &owner.spans()[owner.span_site(site)?];
                    if state.get(&(span.operands()[0].get() as usize)) != Some(&next_index)
                        || state.get(&(span.operands()[1].get() as usize)) != Some(&range_end) { return None; }
                    selected = true; ranges.insert(node);
                }
                if owner.call_region(site) == region.map
                    && let Action::LibraryCall { template, .. } = code.blocks()[site].action()
                    && template.primitive() == P::MapInsert
                {
                    insertions = insertions.checked_add(1)?;
                    insert_nodes.insert(node);
                    if !selected || insertions > 1 { single_insert = false; }
                }
                walk.step(node, &mut state, &mut terms, false)?;
                if site == region.tail.latch() as usize {
                    if !selected || state.get(&index_word) != Some(&next_index)
                        || terms.value(*state.get(&graph.carrier_word)?) != Some(u64::from(active_tag)) { return None; }
                    if insertions != 1 { single_insert = false; }
                    recurrence_sites.insert(site); recurrence_nodes.insert(node); paths += 1; break;
                }
                match graph.nodes[node].edge {
                    Edge::Jump(Target::Node(next)) => node = next,
                    Edge::Branch { yes: Target::Node(yes), no: Target::Node(no) } => {
                        if pending.len().checked_add(paths)? >= MAX_PATHS { return None; }
                        pending.push((no, state.clone(), selected, insertions)); node = yes;
                    }
                    Edge::Reject => { paths += 1; break; } // Original source rejection remains emitted.
                    _ => return None,
                }
            }
        }
        if recurrence_sites.is_empty() || ranges.is_empty() { return None; }
        let Edge::Jump(Target::Node(body)) = entry.edge else { return None; };
        let mut certified = ranges.clone(); certified.extend([second, third]);
        if !guarded_entry_dominates(graph, region.tail.header() as usize, active_tag,
            body, first, &certified) { return None; }
        let mut mutation_sites = certified.clone(); mutation_sites.extend(insert_nodes.iter().copied());
        let single_insert_nodes = (single_insert && !insert_nodes.is_empty()
            && closed_active_recurrence(graph, region.tail.header() as usize, active_tag, &recurrence_nodes)
            && guarded_entry_dominates(graph, region.tail.header() as usize, active_tag, body, first, &mutation_sites))
            .then_some(insert_nodes);
        return Some(Proof { index_word, width_word, limit_word: limit_value.get() as usize,
            active_tag, first, later: [second, third], ranges,
            guard_sites: [first, second, third].map(|node| graph.nodes[node].site),
            range_site: packed.range_site() as usize, recurrence_sites: recurrence_sites.into_iter().collect(),
            recurrence_nodes, single_insert_nodes });
    }
    None
}

#[cfg(not(feature = "canonical-native-program"))]
pub(super) fn derive(_: CodePlan<'_>, _: &Layout, _: usize, _: &NativeMapOwnerWords,
    _: &NativeScalarAggregates, _: &Graph, _: &BTreeSet<usize>) -> Option<Proof> { None }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guarded_loop_rejects_alternate_state_join_and_previous_visit_guard() {
        use tag_threading::Node;
        let node = |site, edge, header_tag| Node { site, edge, header_tag,
            folded_branch: false, known_latch: false };
        let mut graph = Graph { carrier_word: 0, variants: vec![0, 1], nodes: vec![
            node(10, Edge::Dispatch { cases: vec![(0, Target::Node(2)), (1, Target::Node(6))], other: Target::Node(6) }, None),
            node(10, Edge::Jump(Target::Node(2)), Some(0)),
            node(11, Edge::Jump(Target::Node(3)), None), // Active body prefix.
            node(12, Edge::Branch { yes: Target::Node(6), no: Target::Node(4) }, None), // First guard.
            node(13, Edge::Jump(Target::Node(5)), None), // Certified range.
            node(14, Edge::Jump(Target::Node(1)), None), // Original latch.
            node(15, Edge::Reject, None),
        ] };
        let certified = BTreeSet::from([4]);
        assert!(guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified));
        graph.nodes[6].edge = Edge::Jump(Target::Node(4));
        assert!(!guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified), "Other must not join after the guard");
        graph.nodes[6].edge = Edge::Jump(Target::Node(2));
        assert!(!guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified), "a copied tag is not a certified prefix entry");
        graph.nodes[6].edge = Edge::Reject;
        graph.nodes.push(node(10, Edge::Jump(Target::Node(4)), Some(1)));
        assert!(!guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified), "every later header starts a fresh dominance root");
    }
    #[test]
    fn guarded_loop_stop_equivalence_includes_noncarrier_live_words() {
        let a = State::from([(1, 10), (2, 20), (3, 30), (4, 40)]);
        // Same tag and map payload are insufficient if a later source action
        // also observes a word defined by the bypassed guard prefix.
        let b = State::from([(1, 10), (2, 20), (3, 31), (4, 41)]);
        assert!(same_live(&a, &b, &BTreeSet::from([1, 2])));
        assert!(!same_live(&a, &b, &BTreeSet::from([1, 2, 3])));
        assert!(!same_live(&a, &b, &BTreeSet::from([1, 2, 5])));
    }
    #[test]
    fn guarded_loop_terms_are_finite_and_preserve_exact_value_identity() {
        let mut terms = Terms::default();
        let index = terms.intern(Expr::Input(1)).unwrap();
        let one = terms.constant(1).unwrap(); let two = terms.constant(2).unwrap();
        assert_eq!(terms.add(index, one), terms.add(one, index));
        assert_ne!(terms.add(index, one), terms.add(index, two));
        for word in 0..MAX_TERMS { if terms.intern(Expr::Input(word)).is_none() { break; } }
        assert_eq!(terms.values.len(), MAX_TERMS);
        assert!(terms.intern(Expr::Input(MAX_TERMS + 1)).is_none());
        assert_eq!(terms.intern(Expr::Input(1)), Some(index), "known identities survive proof-budget exhaustion");
    }
}

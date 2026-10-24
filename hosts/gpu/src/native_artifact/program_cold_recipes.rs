//! Exact scalar/frame recipes at original source continuations. Analysis is
//! confined to one successful traversal; completed edges rebase index+1 to
//! the next cursor. Unknown values are never replaced by guessed recipes.
use super::*;

const MAX_PATHS: usize = 64;
const MAX_STEPS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScalarRecipe {
    Constant(u64),
    Index(u64),
}
impl ScalarRecipe {
    fn add(self, rhs: Self) -> Option<Self> {
        use ScalarRecipe::*;
        match (self, rhs) {
            (Constant(a), Constant(b)) => Some(Constant(a.wrapping_add(b))),
            (Index(a), Constant(b)) | (Constant(b), Index(a)) => Some(Index(a.wrapping_add(b))),
            _ => None,
        }
    }
    fn sub(self, rhs: Self) -> Option<Self> {
        use ScalarRecipe::*;
        match (self, rhs) {
            (Constant(a), Constant(b)) => Some(Constant(a.wrapping_sub(b))),
            (Index(a), Constant(b)) => Some(Index(a.wrapping_sub(b))),
            (Index(a), Index(b)) => Some(Constant(a.wrapping_sub(b))),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    words: BTreeMap<usize, ScalarRecipe>,
    updates: Option<u64>,
    previous: Option<u64>,
}
impl State {
    fn assign(&mut self, word: usize, value: Option<ScalarRecipe>) {
        if let Some(value) = value {
            self.words.insert(word, value);
        } else {
            self.words.remove(&word);
        }
    }
    fn intersect(&mut self, rhs: &Self) {
        self.words
            .retain(|word, value| rhs.words.get(word) == Some(value));
        if self.updates != rhs.updates {
            self.updates = None;
        }
        if self.previous != rhs.previous {
            self.previous = None;
        }
    }
    fn rebase_completed(&mut self) {
        for value in self.words.values_mut() {
            if let ScalarRecipe::Index(offset) = value {
                *offset = offset.wrapping_sub(1);
            }
        }
        self.updates = self.updates.map(|offset| offset.wrapping_sub(1));
        self.previous = self.previous.map(|offset| offset.wrapping_sub(1));
    }
}

#[derive(Clone, Default)]
pub(super) struct Snapshot {
    pub words: BTreeMap<usize, u64>,
    pub updates: Option<u64>,
    pub previous: Option<u64>,
}
impl Snapshot {
    fn from_state(state: &State, live: &BTreeSet<usize>) -> Self {
        Self {
            words: state
                .words
                .iter()
                .filter_map(|(&word, value)| match value {
                    ScalarRecipe::Index(offset) if live.contains(&word) => Some((word, *offset)),
                    _ => None,
                })
                .collect(),
            updates: state.updates,
            previous: state.previous,
        }
    }
    pub fn records(&self) -> usize {
        self.words.len() + 5
    }
}

pub(super) struct Proof {
    pub before: BTreeMap<usize, Snapshot>,
    pub completed: BTreeMap<usize, Snapshot>,
}
impl Proof {
    pub fn records(&self) -> usize {
        self.before
            .values()
            .chain(self.completed.values())
            .map(Snapshot::records)
            .sum()
    }
}

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(super) fn take_emissions() -> usize {
    EMISSIONS.with(|value| value.replace(0))
}
#[cfg(test)]
pub(super) fn note_emissions(count: usize) {
    EMISSIONS.with(|value| value.set(value.get() + count));
}
#[cfg(test)]
pub(super) fn without<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            DISABLED.with(|value| value.set(self.0));
        }
    }
    let _restore = Restore(DISABLED.with(|value| value.replace(true)));
    f()
}

fn merge(states: &mut BTreeMap<usize, State>, node: usize, state: &State) {
    if let Some(previous) = states.get_mut(&node) {
        previous.intersect(state);
    } else {
        states.insert(node, state.clone());
    }
}

fn apply(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    site: usize,
    state: &mut State,
) -> Option<()> {
    if let Some((layout, action)) = aggregate_at(owner, aggregates, closed.regions[id].map?, site) {
        let at = |value, index| field(layout, value, index);
        let read = |word| state.words.get(&word).copied();
        // This is the successful action edge. Original guard failures keep
        // the BEFORE state; parallel copies read all sources before writing.
        let (output, values) = match action {
            Aggregate::Pack { output, words } => (
                *output,
                words
                    .iter()
                    .map(|word| {
                        Some(match word {
                            Word::Scalar(value) => read(value.get() as usize),
                            Word::Field(value, index) => read(at(*value, *index)?),
                            Word::Constant(value) => Some(ScalarRecipe::Constant(*value)),
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            ),
            Aggregate::Copy { source, output } => (
                *output,
                (0..layout.slot(*output)?.1)
                    .map(|index| Some(read(at(*source, index)?)))
                    .collect::<Option<Vec<_>>>()?,
            ),
            Aggregate::Project {
                source,
                offset,
                output,
                ..
            } => (
                *output,
                (0..layout.slot(*output)?.1)
                    .map(|index| Some(read(at(*source, offset.checked_add(index)?)?)))
                    .collect::<Option<Vec<_>>>()?,
            ),
            Aggregate::Observe {
                source,
                offset,
                ordinal,
                output,
                ..
            } => {
                let input = read(at(*source, *offset)?);
                let value = match (ordinal, input) {
                    (None, value) => value,
                    (Some(ordinal), Some(ScalarRecipe::Constant(value))) => Some(
                        ScalarRecipe::Constant(u64::from(value == u64::from(*ordinal))),
                    ),
                    _ => None,
                };
                state.assign(output.get() as usize, value);
                return Some(());
            }
        };
        for (index, value) in values.into_iter().enumerate() {
            state.assign(at(output, index)?, value);
        }
    } else if let Some(span) = owner.span_site(site).map(|index| &owner.spans()[index]) {
        if span.range_site() as usize == site {
            state.previous = Some(0);
        }
        if let Action::LibraryCall { output, .. } = code.blocks()[site].action() {
            state.assign(output.get() as usize, None);
        }
    } else {
        match code.blocks()[site].action() {
            Action::Scalar(Scalar::None) | Action::Latch { .. } => {}
            Action::Scalar(Scalar::Copy { source, output }) => {
                state.assign(
                    output.get() as usize,
                    state.words.get(&(source.get() as usize)).copied(),
                );
            }
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
                ..
            }) => {
                let args = operands
                    .iter()
                    .map(|v| state.words.get(&(v.get() as usize)).copied())
                    .collect::<Vec<_>>();
                let value = match (*op, args.as_slice()) {
                    (Op::U64Add, &[Some(a), Some(b)]) => a.add(b),
                    (Op::U64Sub, &[Some(a), Some(b)]) => a.sub(b),
                    _ => None,
                };
                state.assign(output.get() as usize, value);
            }
            Action::LibraryCall {
                template, output, ..
            } if owner.call_region(site) == closed.regions[id].map => {
                if template.primitive() == P::MapInsert {
                    state.updates = state.updates.and_then(|value| value.checked_add(1));
                }
                state.assign(output.get() as usize, None);
            }
            _ => return None,
        }
    }
    Some(())
}

pub(super) fn derive(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    source: &super::Proof,
    slots: &BTreeSet<usize>,
) -> Option<Proof> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) {
        return None;
    }
    let mut initial = State {
        words: BTreeMap::new(),
        updates: Some(0),
        previous: None,
    };
    for index in 0..code.constant_count() {
        let (word, value) = code.constant(index).ok()?;
        if let Some(value) = value
            && slots.contains(&(word.get() as usize))
        {
            initial
                .words
                .insert(word.get() as usize, ScalarRecipe::Constant(value));
        }
    }
    initial
        .words
        .insert(source.bounds.index_word, ScalarRecipe::Index(0));
    initial.words.insert(
        source.graph.carrier_word,
        ScalarRecipe::Constant(u64::from(source.bounds.active_tag)),
    );
    let mut before = BTreeMap::new();
    let mut completed = BTreeMap::new();
    let mut pending = vec![(source.active, initial, BTreeSet::new())];
    let mut steps = 0;
    let mut paths = 0;
    while let Some((mut node, mut state, mut seen)) = pending.pop() {
        loop {
            steps += 1;
            if steps > MAX_STEPS || !seen.insert(node) {
                return None;
            }
            merge(&mut before, node, &state);
            let current = &source.graph.nodes[node];
            if (current.site == closed.regions[id].tail.header() as usize && node != source.active)
                || matches!(
                    current.edge,
                    tag_threading::Edge::Reject | tag_threading::Edge::Dispatch { .. }
                )
            {
                break;
            }
            apply(
                code,
                closed,
                id,
                owner,
                aggregates,
                current.site,
                &mut state,
            )?;
            let next = if node == source.bounds.first {
                let tag_threading::Edge::Branch { no, .. } = current.edge else {
                    return None;
                };
                vec![no]
            } else {
                super::targets(&source.graph, &source.bounds, node)
            };
            let mut continuing = Vec::new();
            for target in next {
                match target {
                    tag_threading::Target::Node(next) if next == source.active => {
                        // The existing source proof certifies one selection,
                        // one insertion and exactly counter+1 on this edge.
                        if !source.bounds.recurrence_nodes.contains(&node)
                            || state.words.get(&source.bounds.index_word)
                                != Some(&ScalarRecipe::Index(1))
                            || state.updates != Some(1)
                            || state.previous != Some(0)
                        {
                            return None;
                        }
                        let mut next = state.clone();
                        next.rebase_completed();
                        merge(&mut completed, node, &next);
                    }
                    tag_threading::Target::Node(next) if source.nodes.contains(&next) => {
                        continuing.push(next)
                    }
                    tag_threading::Target::Node(next) => merge(&mut before, next, &state),
                    tag_threading::Target::Exit => {
                        merge(&mut before, source.graph.nodes.len(), &state)
                    }
                }
            }
            if continuing.is_empty() {
                break;
            }
            node = continuing[0];
            for &next in &continuing[1..] {
                paths += 1;
                if paths > MAX_PATHS {
                    return None;
                }
                pending.push((next, state.clone(), seen.clone()));
            }
        }
    }
    let returned = closed.regions[id].tail.returned().get() as usize;
    Some(Proof {
        before: before
            .into_iter()
            .map(|(node, state)| {
                (
                    node,
                    Snapshot::from_state(&state, &source.live_at(node, returned)),
                )
            })
            .collect(),
        completed: completed
            .into_iter()
            .map(|(edge, state)| {
                (
                    edge,
                    Snapshot::from_state(&state, &source.live_at(source.active, returned)),
                )
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_merges_preserve_only_equal_values_and_exact_frame_progress() {
        let mut left = State {
            words: BTreeMap::from([
                (1, ScalarRecipe::Index(0)),
                (2, ScalarRecipe::Index(1)),
                (3, ScalarRecipe::Constant(7)),
            ]),
            updates: Some(0),
            previous: Some(0),
        };
        let right = State {
            words: BTreeMap::from([(1, ScalarRecipe::Index(0)), (2, ScalarRecipe::Index(2))]),
            updates: Some(1),
            previous: Some(0),
        };
        left.intersect(&right);
        assert_eq!(left.words, BTreeMap::from([(1, ScalarRecipe::Index(0))]));
        assert_eq!((left.updates, left.previous), (None, Some(0)));
        left.rebase_completed();
        assert_eq!(left.words[&1], ScalarRecipe::Index(u64::MAX));
        assert_eq!(left.previous, Some(u64::MAX));
    }
}

//! Backward live-ordinal analysis for a private scan followed by a range join.
//! This proves storage lower bounds only. It neither implements acquisition
//! nor evaluates the FSM: emitted Mech control remains the sole computation.
use super::super::{
    NativeAggregateWord as PackWord, NativeCollectionPrimitive as Primitive,
    NativeScalarAggregateAction as Aggregate, NativeScalarAggregates, NativeSelectedTextJoin,
    NativeTailLoop,
};
use super::{
    Action, NativeControlLoop, Op, Plan, Scalar, SemanticActions, Value,
    availability::{NativeTextLineReadFrontier, identity},
    definitions, predicate_branches, reads, semantic_actions, successors,
};
use mech_core::{SchemaBody, ValueData};

/// Word offsets in the SAME checked scalar-aggregate carrier layout. Scanning
/// states retain from min(keep_fields). A returning state retains from first
/// only when first < end; an empty range must not revive a discarded prefix.
#[derive(Clone, Debug)]
pub struct NativeTextLineRetentionState {
    ordinal: u32,
    index: Option<usize>,
    keep: Box<[usize]>,
    returned: Option<(usize, usize)>,
}
impl NativeTextLineRetentionState {
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }
    pub const fn index_field(&self) -> Option<usize> {
        self.index
    }
    pub fn keep_fields(&self) -> &[usize] {
        &self.keep
    }
    pub const fn returned_range(&self) -> Option<(usize, usize)> {
        self.returned
    }
}

/// Constructor-private retention rules bound to actual current-visit reads,
/// carried words, original range selection, and its nonempty guard. No source
/// names, state names, string literals, benchmark sizes or host URIs are used.
/// Full capture/error ordering, budgets, Unicode and owning-session delivery
/// still require separate implementation and qualification before streaming.
#[derive(Clone, Debug)]
pub struct NativeTextLineRetentionFrontier {
    read: NativeTextLineReadFrontier,
    iteration: u32,
    join: NativeSelectedTextJoin,
    result_fields: (usize, usize),
    states: Box<[NativeTextLineRetentionState]>,
    scans_per_ordinal: Option<u64>,
}
impl NativeTextLineRetentionFrontier {
    pub fn read(&self) -> &NativeTextLineReadFrontier {
        &self.read
    }
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub fn join(&self) -> &NativeSelectedTextJoin {
        &self.join
    }
    pub const fn result_fields(&self) -> (usize, usize) {
        self.result_fields
    }
    pub fn states(&self) -> &[NativeTextLineRetentionState] {
        &self.states
    }
    /// A bound on scan traversals at one physical input ordinal. The same
    /// symbol analysis proves nondecreasing indices and an acyclic graph of
    /// zero-advance transitions. A zero-advance cycle retains the ordinary
    /// snapshot strategy; retention alone does not prove bounded execution.
    pub const fn scans_per_ordinal(&self) -> Option<u64> {
        self.scans_per_ordinal
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Atom {
    State(u32, usize),
    Result(usize),
    Count,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Word {
    atom: Option<Atom>,
    add: u64,
}
impl Word {
    fn constant(value: u64) -> Self {
        Self {
            atom: None,
            add: value,
        }
    }
    fn state(self) -> Option<(u32, usize)> {
        if let Some(Atom::State(ordinal, field)) = self.atom {
            Some((ordinal, field))
        } else {
            None
        }
    }
    fn result(self) -> Option<usize> {
        if self.add == 0
            && let Some(Atom::Result(field)) = self.atom
        {
            Some(field)
        } else {
            None
        }
    }
    fn plus(self, other: Self) -> Option<Self> {
        if self.atom.is_some() && other.atom.is_some() {
            return None;
        }
        Some(Self {
            atom: self.atom.or(other.atom),
            add: self.add.checked_add(other.add)?,
        })
    }
}
#[derive(Clone, Debug)]
enum Def {
    Known(Box<[Word]>),
    Copy(Value),
    Pack(Box<[PackWord]>),
    Field(Value, usize),
    Add(Value, Value),
    Other,
}
struct Symbols {
    defs: Vec<Vec<Def>>,
    words: Vec<Option<Box<[Word]>>>,
}

fn scans_per_ordinal(next: &[Box<[Word]>], indices: &[Option<usize>]) -> Option<u64> {
    let mut outgoing = vec![Vec::new(); indices.len()];
    let mut incoming = vec![0usize; indices.len()];
    for words in next {
        let target = ordinal(words)?;
        let Some(index) = *indices.get(target)? else {
            continue;
        };
        let word = *words.get(index)?;
        let (source, field) = word.state()?;
        let source = source as usize;
        if indices.get(source).copied().flatten() != Some(field) {
            return None;
        }
        if word.add == 0 {
            outgoing[source].push(target);
            incoming[target] = incoming[target].checked_add(1)?;
        }
    }
    let mut ready = indices
        .iter()
        .enumerate()
        .filter_map(|(state, index)| (index.is_some() && incoming[state] == 0).then_some(state))
        .collect::<Vec<_>>();
    let count = indices.iter().filter(|index| index.is_some()).count();
    let mut seen = 0usize;
    while let Some(state) = ready.pop() {
        seen += 1;
        for &target in &outgoing[state] {
            incoming[target] -= 1;
            if incoming[target] == 0 {
                ready.push(target);
            }
        }
    }
    // Every positive transition increases the checked u64 index by >=1;
    // between increases, a DAG path visits at most every scanning state once.
    (seen == count && count != 0).then_some(count as u64)
}
impl Symbols {
    fn scalar(&self, value: Value) -> Option<Word> {
        let [word] = self.words[value.get() as usize].as_deref()? else {
            return None;
        };
        Some(*word)
    }
    fn evaluate(&self, def: &Def) -> Option<Box<[Word]>> {
        match def {
            Def::Known(words) => Some(words.clone()),
            Def::Copy(source) => self.words[source.get() as usize].clone(),
            Def::Field(source, field) => {
                Some(vec![*self.words[source.get() as usize].as_deref()?.get(*field)?].into())
            }
            Def::Add(a, b) => Some(vec![self.scalar(*a)?.plus(self.scalar(*b)?)?].into()),
            Def::Pack(words) => words
                .iter()
                .map(|word| match word {
                    PackWord::Constant(n) => Some(Word::constant(*n)),
                    PackWord::Scalar(value) => self.scalar(*value),
                    PackWord::Field(value, field) => self.words[value.get() as usize]
                        .as_deref()?
                        .get(*field)
                        .copied(),
                })
                .collect(),
            Def::Other => None,
        }
    }
    fn new(
        plan: &Plan,
        actions: &SemanticActions<'_>,
        copy_defs: &[Vec<Option<Value>>],
        aggregates: &NativeScalarAggregates,
        tail: &NativeTailLoop,
        counts: &[Value],
    ) -> Self {
        let mut result = Self {
            defs: vec![Vec::new(); plan.values().len()],
            words: vec![None; plan.values().len()],
        };
        for constant in plan.constants() {
            if let ValueData::U64(n) = constant.constant().data() {
                result.defs[constant.value().get() as usize]
                    .push(Def::Known(vec![Word::constant(*n)].into()));
            }
        }
        for (value, words) in aggregates.constants() {
            result.defs[value.get() as usize].push(Def::Known(
                words.iter().map(|n| Word::constant(*n)).collect(),
            ));
        }
        for (site, action) in actions.iter().enumerate() {
            let selected = match aggregates.action(site) {
                Some(Aggregate::Pack { output, words }) => {
                    Some((*output, Def::Pack(words.clone())))
                }
                Some(Aggregate::Copy { source, output }) => Some((*output, Def::Copy(*source))),
                Some(Aggregate::Observe {
                    source,
                    offset,
                    ordinal: None,
                    guards,
                    output,
                }) => {
                    let source = identity(copy_defs, *source);
                    let word = if source == Some(tail.carrier()) {
                        match guards.as_ref() {
                            [(0, ordinal)] if *offset > 0 => Some(Word {
                                atom: Some(Atom::State(*ordinal, *offset)),
                                add: 0,
                            }),
                            _ => None,
                        }
                    } else if source == Some(tail.result()) && guards.is_empty() {
                        Some(Word {
                            atom: Some(Atom::Result(*offset)),
                            add: 0,
                        })
                    } else {
                        None
                    };
                    Some((
                        *output,
                        word.map(|word| Def::Known(vec![word].into()))
                            .unwrap_or_else(|| {
                                source
                                    .map(|source| Def::Field(source, *offset))
                                    .unwrap_or(Def::Other)
                            }),
                    ))
                }
                _ => None,
            };
            if let Some((output, def)) = selected {
                result.defs[output.get() as usize].push(def);
                continue;
            }
            let selected = match action {
                Action::Scalar(Scalar::Copy { source, output }) => {
                    Some((*output, Def::Copy(*source)))
                }
                Action::Scalar(Scalar::Primitive {
                    op: Op::U64Add,
                    operands,
                    output,
                    ..
                }) if operands.len() == 2 => Some((*output, Def::Add(operands[0], operands[1]))),
                Action::Scalar(Scalar::Primitive { output, .. })
                | Action::Structural { output, .. }
                | Action::LibraryCall { output, .. } => Some((*output, Def::Other)),
                _ => None,
            };
            if let Some((output, def)) = selected {
                result.defs[output.get() as usize].push(def);
            }
        }
        // These roots are dynamic identities, not their initializer words.
        result.defs[tail.carrier().get() as usize] = vec![Def::Other];
        result.defs[tail.result().get() as usize] = vec![Def::Other];
        for value in counts {
            result.defs[value.get() as usize] = vec![Def::Known(
                vec![Word {
                    atom: Some(Atom::Count),
                    add: 0,
                }]
                .into(),
            )];
        }
        // Only equal ALL-definition results become symbols. No first-arm phi
        // shortcut, speculative constant state, or recursive evaluator is used.
        loop {
            let mut changed = false;
            for value in 0..result.defs.len() {
                if result.words[value].is_some() || result.defs[value].is_empty() {
                    continue;
                }
                let alternatives = result.defs[value]
                    .iter()
                    .map(|def| result.evaluate(def))
                    .collect::<Option<Vec<_>>>();
                if let Some(alternatives) = alternatives
                    && alternatives.iter().all(|words| words == &alternatives[0])
                {
                    result.words[value] = Some(alternatives[0].clone());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        result
    }
    /// Enumerate every assignment feeding a phi endpoint without choosing a
    /// branch or interpreting the language. Unsupported leaves reject proof.
    fn alternatives(&self, value: Value) -> Option<Vec<Box<[Word]>>> {
        let mut pending = vec![value];
        let mut seen = vec![false; self.defs.len()];
        let mut result = Vec::new();
        while let Some(value) = pending.pop() {
            if std::mem::replace(&mut seen[value.get() as usize], true) {
                continue;
            }
            let defs = &self.defs[value.get() as usize];
            if defs.is_empty() {
                return None;
            }
            for def in defs {
                if let Def::Copy(source) = def {
                    pending.push(*source);
                } else {
                    result.push(self.evaluate(def)?);
                }
            }
        }
        (!result.is_empty()).then_some(result)
    }
}

fn reachable(plan: &Plan, cut: &[(u32, u32)]) -> Vec<bool> {
    let mut seen = vec![false; plan.blocks().len()];
    let mut pending = vec![plan.entry()];
    while let Some(site) = pending.pop() {
        if std::mem::replace(&mut seen[site as usize], true) {
            continue;
        }
        for next in successors(plan.blocks()[site as usize].terminator())
            .into_iter()
            .flatten()
        {
            if !cut.contains(&(site, next)) {
                pending.push(next);
            }
        }
    }
    seen
}
fn nonempty(
    plan: &Plan,
    symbols: &Symbols,
    first: Word,
    end: Word,
    selection: u32,
    uses: &[Vec<super::Read>],
    defs: &[Vec<Option<Value>>],
) -> bool {
    let mut cut = Vec::new();
    for (site, block) in plan.blocks().iter().enumerate() {
        let Action::Scalar(Scalar::Primitive {
            op,
            operands,
            output,
            ..
        }) = block.action()
        else {
            continue;
        };
        let [left, right] = operands.as_ref() else {
            continue;
        };
        if block.iteration().is_some() {
            continue;
        }
        let selected = match op {
            Op::U64Lt => {
                symbols.scalar(*left) == Some(first) && symbols.scalar(*right) == Some(end)
            }
            Op::U64Gt => {
                symbols.scalar(*left) == Some(end) && symbols.scalar(*right) == Some(first)
            }
            _ => false,
        };
        if selected
            && let Some(branches) = predicate_branches(plan, site as u32, *output, uses, defs)
        {
            cut.extend(branches.into_iter().map(|(site, _, yes)| (site, yes)));
        }
    }
    !cut.is_empty() && !reachable(plan, &cut)[selection as usize]
}
fn ordinal(words: &[Word]) -> Option<usize> {
    let first = *words.first()?;
    if first.atom.is_some() {
        return None;
    }
    usize::try_from(first.add).ok()
}
fn empty(first: Word, end: Word) -> bool {
    first == end || (first.atom.is_none() && end.atom.is_none() && first.add >= end.add)
}

fn frontier(
    plan: &Plan,
    read: NativeTextLineReadFrontier,
    actions: &SemanticActions<'_>,
    defs: &[Vec<Option<Value>>],
    uses: &[Vec<super::Read>],
    aggregates: &NativeScalarAggregates,
) -> Option<NativeTextLineRetentionFrontier> {
    let iteration = read.reads().first()?.iteration();
    if read
        .reads()
        .iter()
        .any(|read| read.iteration() != iteration)
        || read
            .count()
            .guards()
            .iter()
            .any(|guard| plan.blocks()[guard.comparison() as usize].iteration() != Some(iteration))
    {
        return None;
    }
    let NativeControlLoop::Tail(tail) = plan.iterations().get(iteration as usize)? else {
        return None;
    };
    let SchemaBody::Enum { variants, .. } = plan.values()[tail.carrier().get() as usize]
        .definition()
        .body()
    else {
        return None;
    };
    let (_, width) = aggregates.slot(tail.carrier())?;
    let symbols = Symbols::new(plan, actions, defs, aggregates, tail, read.count().counts());
    let [selection] = read.count().lines().selections() else {
        return None;
    };
    let join = plan
        .selected_text_joins()
        .into_iter()
        .find(|join| join.selection() == *selection)?;
    let (first, end) = plan
        .blocks()
        .iter()
        .find_map(|block| match block.action() {
            Action::LibraryCall {
                template,
                operands,
                output,
            } if *output == join.selector()
                && template.primitive() == Primitive::RangeExclusive
                && operands.len() == 2 =>
            {
                Some((symbols.scalar(operands[0])?, symbols.scalar(operands[1])?))
            }
            _ => None,
        })?;
    let result_fields = (first.result()?, end.result()?);
    if result_fields.0 == result_fields.1
        || !nonempty(plan, &symbols, first, end, *selection, uses, defs)
    {
        return None;
    }
    let mut indices = vec![None; variants.len()];
    for access in read.reads() {
        let word = symbols.scalar(access.index())?;
        let (state, field) = word.state()?;
        if word.add != 0 || field == 0 || field >= width {
            return None;
        }
        let index = indices.get_mut(state as usize)?;
        if index.is_some_and(|prior| prior != field) {
            return None;
        }
        *index = Some(field);
    }
    // The generated header waits for the current read ordinal, not arbitrary
    // count-comparison operands. Every comparison that can be deferred must
    // name that SAME state field with no lookahead. Otherwise an additional
    // `i + 1 > count` could reach its original comparison before row i + 1
    // exists and turn an ordinary pause into a permanent native ABI failure.
    // Reuse the all-definitions symbol proof; unproved relations retain the
    // complete-snapshot strategy before any source acquisition occurs.
    for guard in read.count().guards() {
        let word = symbols.scalar(guard.index())?;
        let (state, field) = word.state()?;
        if word.add != 0 || indices.get(state as usize).copied().flatten() != Some(field) {
            return None;
        }
    }
    let mut ranges = vec![None; variants.len()];
    for returned in symbols.alternatives(tail.returned())? {
        let a = *returned.get(result_fields.0)?;
        let b = *returned.get(result_fields.1)?;
        let (state, first) = a.state()?;
        let (other, end) = b.state()?;
        if state != other
            || a.add != 0
            || b.add != 0
            || indices.get(state as usize)?.is_some()
            || first == 0
            || end == 0
            || first >= width
            || end >= width
        {
            return None;
        }
        let range = ranges.get_mut(state as usize)?;
        if range.is_some_and(|prior| prior != (first, end)) {
            return None;
        }
        *range = Some((first, end));
    }
    if indices
        .iter()
        .zip(&ranges)
        .any(|(index, range)| index.is_none() == range.is_none())
    {
        return None;
    }
    let next = symbols.alternatives(tail.next())?;
    if next.iter().any(|words| words.len() != width) {
        return None;
    }
    let mut keep = vec![vec![false; width]; variants.len()];
    for (state, index) in indices.iter().enumerate() {
        if let Some(field) = index {
            keep[state][*field] = true;
        }
    }
    // A nonempty terminal range seeds backward liveness. Equal or reversed
    // constant bounds need no data, as proved by the ORIGINAL nonempty guard.
    for words in &next {
        let state = ordinal(words)?;
        if let Some((first, end)) = *ranges.get(state)? {
            let (first, end) = (words[first], words[end]);
            if empty(first, end) {
                continue;
            }
            let (source, field) = first.state()?;
            if indices.get(source as usize)?.is_none() || field == 0 || field >= width {
                return None;
            }
            keep[source as usize][field] = true;
        }
    }
    // Every live target field must advance from an included field of its
    // source state by checked nonnegative addition. This proves the minimum
    // retention ordinal cannot move backwards at a successful latch.
    loop {
        let mut changed = false;
        for words in &next {
            let target = ordinal(words)?;
            let Some(index) = *indices.get(target)? else {
                continue;
            };
            let (source, field) = words[index].state()?;
            if indices.get(source as usize).copied().flatten() != Some(field) {
                return None;
            }
            for target_field in 1..width {
                if !keep[target][target_field] {
                    continue;
                }
                let (owner, source_field) = words[target_field].state()?;
                if owner != source || source_field == 0 || source_field >= width {
                    return None;
                }
                if !keep[source as usize][source_field] {
                    keep[source as usize][source_field] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    for seed in symbols.alternatives(tail.seed())? {
        if seed.len() != width {
            return None;
        }
        let state = ordinal(&seed)?;
        if let Some(index) = *indices.get(state)? {
            if seed[index].atom.is_some() || seed[index].add == 0 {
                return None;
            }
            // Every live retention ordinal is one-based, not only the read
            // index. The liveness induction above admits only checked
            // nonnegative additions from included current-state fields, so
            // positive seeds keep all later open/nonempty bounds positive.
            for field in 1..width {
                if keep[state][field] && (seed[field].atom.is_some() || seed[field].add == 0) {
                    return None;
                }
            }
        } else {
            let (first, end) = ranges[state]?;
            if !empty(seed[first], seed[end]) {
                return None;
            }
        }
    }
    let scans_per_ordinal = scans_per_ordinal(&next, &indices);
    let states = indices
        .into_iter()
        .zip(ranges)
        .enumerate()
        .map(|(state, (index, returned))| NativeTextLineRetentionState {
            ordinal: state as u32,
            index,
            returned,
            keep: keep[state]
                .iter()
                .enumerate()
                .filter_map(|(field, live)| live.then_some(field))
                .collect(),
        })
        .collect();
    Some(NativeTextLineRetentionFrontier {
        read,
        iteration,
        join,
        result_fields,
        states,
        scans_per_ordinal,
    })
}

#[cfg(test)]
mod progress_bound_tests {
    use super::*;

    fn edge(source: u32, target: u64, advance: u64) -> Box<[Word]> {
        vec![
            Word::constant(target),
            Word {
                atom: Some(Atom::State(source, 1)),
                add: advance,
            },
        ]
        .into()
    }

    #[test]
    fn positive_cycles_and_acyclic_zero_transitions_have_finite_scan_bounds() {
        let indices = [Some(1), Some(1), None];
        assert_eq!(
            scans_per_ordinal(&[edge(0, 1, 0), edge(1, 0, 1), edge(1, 2, 0)], &indices),
            Some(2)
        );
        assert_eq!(scans_per_ordinal(&[edge(0, 0, 1)], &[Some(1)]), Some(1));
    }

    #[test]
    fn zero_advance_cycle_does_not_claim_bounded_scan_work() {
        assert_eq!(scans_per_ordinal(&[edge(0, 0, 0)], &[Some(1)]), None);
        assert_eq!(
            scans_per_ordinal(&[edge(0, 1, 0), edge(1, 0, 0)], &[Some(1), Some(1)]),
            None
        );
    }
}

impl Plan {
    /// Prove monotonic current indices and every future selected-range lower
    /// bound. Unsupported graphs retain ordinary snapshot execution. These
    /// rules alone grant no capture capability or change the execution ABI.
    pub fn text_line_retention_frontiers(&self) -> Vec<NativeTextLineRetentionFrontier> {
        let frontiers = self.text_line_read_frontiers();
        if frontiers.is_empty() {
            return Vec::new();
        }
        let Ok(actions) = semantic_actions(self) else {
            return Vec::new();
        };
        let Ok(uses) = reads(self, &actions) else {
            return Vec::new();
        };
        let defs = definitions(self, &actions);
        let aggregates = self.scalar_aggregates();
        frontiers
            .into_iter()
            .filter_map(|read| frontier(self, read, &actions, &defs, &uses, &aggregates))
            .collect()
    }
}

#[cfg(all(test, feature = "source", feature = "semantic-compiler"))]
#[path = "text_line_retention_tests.rs"]
mod tests;

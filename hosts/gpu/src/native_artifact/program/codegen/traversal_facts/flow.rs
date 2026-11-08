//! One transfer/branch/merge engine for all bounded traversal adapters.
use super::*;

pub(in crate::native_artifact::program::codegen) enum Step<'a> {
    Aggregate(&'a NativeScalarAggregates, &'a Aggregate),
    Scalar(&'a Scalar),
    /// Ordering is derived from the same recipe as native frame emission.
    Assign {
        moves: Vec<(usize, usize)>,
        parallel: bool,
    },
    Define(usize),
    Length {
        array: usize,
        output: usize,
    },
    Read {
        array: usize,
        index: u32,
        output: usize,
    },
    /// The numeric adapter supplies only U64 cells. The successful source
    /// read/add must dominate this exact write before any intervening effect.
    Write {
        array: usize,
        index: u32,
        incoming: u32,
    },
    /// Successful checked replacement preserves shape and immutable scalar
    /// snapshots. This domain exports no array-content equality facts. The
    /// write itself remains fallible and supplies no completion authority.
    Replace,
    Keep,
}

/// Every fact is rebuilt from this exact source plan. Exhausting a finite
/// analysis allowance declines substitutions and preserves the checked path.
pub(in crate::native_artifact::program::codegen) fn derive<'a>(
    code: CodePlan<'_>,
    scope: &Scope,
    slots: &BTreeSet<usize>,
    tag: Option<usize>,
    step: impl Fn(usize) -> Option<Step<'a>>,
) -> Option<Proof> {
    with_windows(code, scope, slots, tag, &[], step)
}

pub(in crate::native_artifact::program::codegen) fn with_windows<'a>(
    code: CodePlan<'_>,
    scope: &Scope,
    slots: &BTreeSet<usize>,
    tag: Option<usize>,
    windows: &[Window],
    step: impl Fn(usize) -> Option<Step<'a>>,
) -> Option<Proof> {
    analyze(
        code,
        scope,
        slots,
        tag,
        windows,
        &[],
        None,
        &Queries::new(),
        &step,
    )
}

pub(in crate::native_artifact::program::codegen) fn with_checks<'a>(
    code: CodePlan<'_>,
    scope: &Scope,
    slots: &BTreeSet<usize>,
    checks: &Queries,
    step: impl Fn(usize) -> Option<Step<'a>>,
) -> Option<Proof> {
    analyze(code, scope, slots, None, &[], &[], None, checks, &step)
}

pub(in crate::native_artifact::program::codegen) fn with_captured_windows<'a>(
    code: CodePlan<'_>,
    scope: &Scope,
    slots: &BTreeSet<usize>,
    captures: &[CapturedWindow],
    step: impl Fn(usize) -> Option<Step<'a>>,
) -> Option<Proof> {
    let mut proof = analyze(
        code,
        scope,
        slots,
        None,
        &[],
        captures,
        None,
        &Queries::new(),
        &step,
    )?;
    proof
        .flat_arrays
        .extend(captures.iter().map(|window| window.array));
    Some(proof)
}

pub(in crate::native_artifact::program::codegen) fn with_completion<'a>(
    code: CodePlan<'_>,
    scope: &Scope,
    slots: &BTreeSet<usize>,
    windows: &[Window],
    completion: completion::Candidate,
    step: impl Fn(usize) -> Option<Step<'a>>,
) -> Option<Proof> {
    if !scope.outside.is_empty() {
        return None;
    }
    analyze(
        code,
        scope,
        slots,
        None,
        windows,
        &[],
        Some(completion),
        &Queries::new(),
        &step,
    )
}

fn analyze<'a>(
    code: CodePlan<'_>,
    scope: &Scope,
    slots: &BTreeSet<usize>,
    tag: Option<usize>,
    windows: &[Window],
    captures: &[CapturedWindow],
    completion: Option<completion::Candidate>,
    checks: &Queries,
    step: &impl Fn(usize) -> Option<Step<'a>>,
) -> Option<Proof> {
    let ordered = scope.order(code)?;
    let (mut terms, partitions) = entry::states(
        code,
        &ordered,
        slots,
        tag,
        windows,
        captures,
        completion.as_ref(),
        &step,
    )?;
    let candidate = completion.clone();
    let mut completion = completion.map(completion::Collector::new);
    let mut incoming: BTreeMap<usize, Vec<Option<State>>> = ordered
        .iter()
        .map(|&site| (site, vec![None; partitions.len()]))
        .collect();
    *incoming.get_mut(&(scope.header))? = partitions.into_iter().map(Some).collect();
    let mut proof = Proof::default();
    let mut written = BTreeSet::new();
    let protected = captures
        .iter()
        .flat_map(|window| [window.base, window.induction, window.limit])
        .collect::<BTreeSet<_>>();
    for &site in &ordered {
        let mut current = BTreeSet::new();
        loop_windows::writes(step(site)?, &mut current)?;
        if site != scope.latch && current.iter().any(|word| protected.contains(word)) {
            return None;
        }
        written.extend(current);
    }
    // Prove recurrence independently of the proposed windows. An unchanged
    // value may be copied through its carrier; a changed value cannot inherit
    // the activation's gate. The recursive analysis has no windows, so depth
    // is bounded at two and its joins cover all ordinary reachable backedges.
    let ordinary_unchanged = if windows.is_empty() {
        None
    } else {
        Some(
            analyze(
                code,
                scope,
                slots,
                tag,
                &[],
                &[],
                None,
                &Queries::new(),
                step,
            )?
            .unchanged_at_latch,
        )
    };
    if windows.iter().any(|window| {
        written.contains(&window.count)
            && !ordinary_unchanged
                .as_ref()
                .is_some_and(|words| words.contains(&window.count))
    }) {
        return None;
    }
    let mut unchanged_at_latch: Option<BTreeSet<usize>> = None;
    for site in ordered {
        let mut states = incoming.remove(&site)?;
        if states.iter().all(Option::is_none) {
            continue;
        }
        let block = &code.blocks()[site];
        let operation = step(site)?;
        if let Some(mut recording) = terms.recording.take() {
            recording.terminal(&terms, &states, &operation);
            terms.recording = Some(recording);
        }
        if let Some(completion) = &mut completion {
            completion.action(&terms, &states, &operation, block.terminator());
        }
        match operation {
            Step::Aggregate(layout, action) => {
                if let Aggregate::Project { source, guards, .. }
                | Aggregate::Observe { source, guards, .. } = action
                {
                    for &(offset, ordinal) in guards {
                        let tag = field(layout, *source, offset)?;
                        let value = terms.constant(u64::from(ordinal))?;
                        if states.iter().flatten().all(|state| {
                            state.read(tag).is_some_and(|tag| {
                                state.truth(&terms, Relation::Eq, tag, value) == Some(true)
                            })
                        }) {
                            proof.guards.insert((site, source.get(), offset, ordinal));
                        }
                        for state in &mut states {
                            if let Some(current) = state {
                                let tag = current.read(tag)?;
                                if !current.assume(&terms, Relation::Eq, tag, value)? {
                                    *state = None;
                                }
                            }
                        }
                    }
                }
                for state in states.iter_mut().flatten() {
                    aggregate(state, &mut terms, layout, action)?;
                }
            }
            Step::Scalar(action) => {
                if !captures.is_empty()
                    && let Scalar::Primitive { op, operands, .. } = action
                    && let Some((relation, reverse)) = comparison(*op)
                    && let [a, b] = operands.as_ref()
                {
                    let outcomes = states
                        .iter()
                        .flatten()
                        .map(|state| {
                            let (a, b) =
                                (state.read(a.get() as usize)?, state.read(b.get() as usize)?);
                            state.truth(
                                &terms,
                                relation,
                                if reverse { b } else { a },
                                if reverse { a } else { b },
                            )
                        })
                        .collect::<Option<Vec<_>>>();
                    if let Some(outcomes) = outcomes
                        && let Some(&value) = outcomes.first()
                        && outcomes.iter().all(|&v| v == value)
                    {
                        proof.predicates.insert(site, value);
                    }
                }
                if let Scalar::Primitive {
                    op: Op::U64Lt,
                    operands,
                    ..
                } = action
                    && operands.len() == 2
                {
                    for state in states.iter().flatten() {
                        if let Some(window) = state.discover_window(
                            &terms,
                            state.read(operands[0].get() as usize)?,
                            state.read(operands[1].get() as usize)?,
                        ) {
                            proof.windows.insert(window);
                        }
                    }
                    if !windows.is_empty() {
                        let outcomes = states
                            .iter()
                            .flatten()
                            .map(|state| {
                                state.truth(
                                    &terms,
                                    Relation::Lt,
                                    state.read(operands[0].get() as usize)?,
                                    state.read(operands[1].get() as usize)?,
                                )
                            })
                            .collect::<Option<Vec<_>>>();
                        if let Some(outcomes) = outcomes
                            && let Some(&value) = outcomes.first()
                            && outcomes.iter().all(|&v| v == value)
                        {
                            proof.predicates.insert(site, value);
                        }
                    }
                }
                if let Scalar::Primitive { op, operands, .. } = action
                    && matches!(op, Op::U64Add | Op::U64Sub)
                    && operands.len() == 2
                {
                    let operation = if *op == Op::U64Add {
                        Arithmetic::Add
                    } else {
                        Arithmetic::Sub
                    };
                    if states.iter().flatten().all(|state| {
                        match (
                            state.read(operands[0].get() as usize),
                            state.read(operands[1].get() as usize),
                        ) {
                            (Some(a), Some(b)) => state.safe(&terms, operation, a, b),
                            _ => false,
                        }
                    }) {
                        proof.arithmetic.insert(site, (*op, operands.to_vec()));
                    }
                }
                for state in states.iter_mut().flatten() {
                    scalar(state, &mut terms, site, action)?;
                }
            }
            Step::Assign { moves, parallel } => {
                for state in states.iter_mut().flatten() {
                    state.assign(&moves, parallel)?;
                }
            }
            Step::Define(output) => {
                let value = terms.intern(Expr::Definition(site, output))?;
                for state in states.iter_mut().flatten() {
                    state.checked_reads.clear();
                    state.words.insert(output, value);
                }
            }
            Step::Length { array, output } => {
                let length = terms.intern(Expr::ArrayLength(array))?;
                for state in states.iter_mut().flatten() {
                    state.words.insert(output, length);
                }
            }
            Step::Read {
                array,
                index,
                output,
            } => {
                let length = terms.intern(Expr::ArrayLength(array))?;
                // An implicit one-based read has the same interval obligation
                // as an explicit base<length comparison. Discover candidates
                // only from its checked mathematical index; the selected
                // version still needs a fresh gate and this visit's k<count.
                for state in states.iter().flatten() {
                    if let Some(base) = state
                        .read(index as usize)
                        .and_then(|index| super::windows::read_base(&terms, index))
                        && let Some(window) = state.discover_window(&terms, base, length)
                    {
                        proof.windows.insert(window);
                    }
                }
                if states.iter().flatten().all(|state| {
                    state
                        .read(index as usize)
                        .is_some_and(|index| state.readable(&terms, index, length))
                }) {
                    proof.reads.insert((site, array, index));
                }
                let value = terms.intern(Expr::Definition(site, output))?;
                for state in states.iter_mut().flatten() {
                    if let Some(recording) = &mut terms.recording {
                        recording.record(
                            value,
                            Node::Read {
                                array,
                                index: state.read(index as usize)?,
                            },
                        );
                    }
                    state.remember_read(array, state.read(index as usize)?, site, output, value)?;
                    state.words.insert(output, value);
                }
            }
            Step::Write {
                array,
                index,
                incoming,
            } => {
                let candidate = states
                    .iter()
                    .flatten()
                    .next()
                    .and_then(|state| state.checked_increment(&terms, array, index, incoming));
                if let Some(candidate) = candidate
                    && states.iter().flatten().all(|state| {
                        state.checked_increment(&terms, array, index, incoming) == Some(candidate)
                    })
                {
                    proof.increment_writes.insert(site, candidate);
                }
                for state in states.iter_mut().flatten() {
                    state.checked_reads.clear();
                }
            }
            Step::Replace | Step::Keep => {
                for state in states.iter_mut().flatten() {
                    state.checked_reads.clear();
                }
            }
        }
        // A child initialize action has just assigned its current arguments.
        // Query this visit's transferred state without admitting the child or
        // carrying any facts across its body/return edge.
        if let Some(checks) = checks.get(&site) {
            for &check in checks {
                if states
                    .iter()
                    .flatten()
                    .all(|state| check.holds(state, &mut terms) == Some(true))
                {
                    proof.checks.insert((site, check));
                }
            }
        }
        if let Some(mut recording) = terms.recording.take() {
            recording.terminal_condition(&terms, &states, block.terminator());
            terms.recording = Some(recording);
        }
        if site == scope.latch {
            for state in states.iter().flatten() {
                let unchanged = state
                    .words
                    .iter()
                    .filter_map(|(&word, &value)| {
                        (terms.terms[value] == Expr::Initial(word)).then_some(word)
                    })
                    .collect::<BTreeSet<_>>();
                if let Some(words) = &mut unchanged_at_latch {
                    words.retain(|word| unchanged.contains(word));
                } else {
                    unchanged_at_latch = Some(unchanged);
                }
            }
            continue;
        }
        let successors = scope::successors(block.terminator())?;
        let branch_count = successors.len();
        let mut reached = Vec::new();
        for (branch, next) in successors.into_iter().enumerate() {
            let mut edge_reached = false;
            let mut outgoing = states.clone();
            let target = &mut outgoing;
            for (target, state) in target.iter_mut().zip(&states) {
                let Some(mut state) = state.clone() else {
                    continue;
                };
                let reachable = match block.terminator() {
                    Term::Scalar(Branch::Branch { condition, .. }) => {
                        let condition = state.read(condition.get() as usize)?;
                        state.condition(&terms, condition, branch == 0)?
                    }
                    Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                        // Float equality is not raw-word equality (NaNs/-0).
                        if code.kind(left) == Some(NativeScalarKind::F64) {
                            true
                        } else {
                            let (a, b) = (
                                state.read(left.get() as usize)?,
                                state.read(right.get() as usize)?,
                            );
                            state.literal(&terms, a, b, branch == 0)?
                        }
                    }
                    _ => true,
                };
                if !reachable {
                    *target = None;
                    continue;
                }
                edge_reached = true;
                *target = Some(state);
            }
            if edge_reached {
                reached.push(next);
            }
            if next as usize == scope.latch {
                if let Some(candidate) = &candidate
                    && let Some(mut recording) = terms.recording.take()
                {
                    for state in outgoing.iter().flatten() {
                        recording.latch(candidate, state, &terms);
                    }
                    terms.recording = Some(recording);
                }
                if let Some(completion) = &mut completion {
                    completion.latch(&terms, &outgoing);
                }
            }
            if next == scope.exit {
                if let Some(completion) = &mut completion {
                    completion.exit(&terms, &outgoing);
                }
            } else if !scope.outside.contains(&next) {
                for (target, state) in incoming.get_mut(&(next as usize))?.iter_mut().zip(outgoing)
                {
                    if let Some(state) = state {
                        if let Some(target) = target {
                            target.merge(&state, next as usize, &mut terms)?;
                        } else {
                            *target = Some(state);
                        }
                    }
                }
            }
        }
        if (!windows.is_empty() || !captures.is_empty()) && branch_count == 2 && reached.len() == 1
        {
            proof.successors.insert(site, reached[0]);
        }
    }
    proof.unchanged_at_latch = unchanged_at_latch.unwrap_or_default();
    let unchanged = ordinary_unchanged
        .as_ref()
        .unwrap_or(&proof.unchanged_at_latch);
    proof
        .windows
        .retain(|window| !written.contains(&window.count) || unchanged.contains(&window.count));
    proof.completed = completion.and_then(completion::Collector::finish);
    if proof.completed.is_some()
        && let Some(recording) = terms.recording.take()
    {
        proof.expressions = Some(recording.finish(&terms, written));
    }
    Some(proof)
}

fn comparison(op: Op) -> Option<(Relation, bool)> {
    Some(match op {
        Op::U64Lt => (Relation::Lt, false),
        Op::U64Le => (Relation::Le, false),
        Op::U64Gt => (Relation::Lt, true),
        Op::U64Ge => (Relation::Le, true),
        Op::U64Eq => (Relation::Eq, false),
        Op::U64Ne => (Relation::Ne, false),
        _ => return None,
    })
}

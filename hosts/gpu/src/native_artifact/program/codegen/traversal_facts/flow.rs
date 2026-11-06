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
    if slots.len() > MAX_WORDS {
        return None;
    }
    let ordered = scope.order(code)?;
    let mut terms = Terms::default();
    let mut initial = State::default();
    initial.windows.extend_from_slice(windows);
    for &slot in slots {
        initial
            .words
            .insert(slot, terms.intern(Expr::Initial(slot))?);
    }
    for i in 0..code.constant_count() {
        let (value, constant) = code.constant(i).ok()?;
        if let Some(constant) = constant
            && slots.contains(&(value.get() as usize))
        {
            initial
                .words
                .insert(value.get() as usize, terms.constant(constant)?);
        }
    }
    let tag = tag.and_then(|slot| initial.read(slot));
    let mut ordinals = BTreeSet::new();
    for &site in &ordered {
        if let Step::Aggregate(_, action) = step(site)? {
            match action {
                Aggregate::Observe {
                    ordinal: Some(ordinal),
                    ..
                } => {
                    ordinals.insert(*ordinal);
                }
                _ => {}
            }
        }
    }
    let mut partitions = Vec::new();
    if let Some(tag) = tag
        && !ordinals.is_empty()
        && ordinals.len() < MAX_PARTITIONS
    {
        for &ordinal in &ordinals {
            let mut state = initial.clone();
            let value = terms.constant(u64::from(ordinal))?;
            state.assume(&terms, Relation::Eq, tag, value)?;
            partitions.push(state);
        }
        // The native entry may contain a malformed tag. This exhaustive
        // remainder preserves its original Reject/ABI path and origin.
        for ordinal in ordinals {
            let value = terms.constant(u64::from(ordinal))?;
            initial.assume(&terms, Relation::Ne, tag, value)?;
        }
    }
    partitions.push(initial);
    let mut incoming: BTreeMap<usize, Vec<Option<State>>> = ordered
        .iter()
        .map(|&site| (site, vec![None; partitions.len()]))
        .collect();
    *incoming.get_mut(&(scope.header))? = partitions.into_iter().map(Some).collect();
    let mut proof = Proof::default();
    let mut written = BTreeSet::new();
    for &site in &ordered {
        loop_windows::writes(step(site)?, &mut written)?;
    }
    // A gate is checked once at activation. A capture changed on a latch
    // cannot inherit that gate on later visits, even if its home is reused.
    if windows.iter().any(|window| written.contains(&window.count)) {
        return None;
    }
    for site in ordered {
        let mut states = incoming.remove(&site)?;
        if states.iter().all(Option::is_none) {
            continue;
        }
        let block = &code.blocks()[site];
        match step(site)? {
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
                        ) && !written.contains(&window.count)
                        {
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
                if states.iter().flatten().all(|state| {
                    state
                        .read(index as usize)
                        .is_some_and(|index| state.readable(&terms, index, length))
                }) {
                    proof.reads.insert((site, array, index));
                }
                let value = terms.intern(Expr::Definition(site, output))?;
                for state in states.iter_mut().flatten() {
                    state.words.insert(output, value);
                }
            }
            Step::Keep => {}
        }
        if site == scope.latch {
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
            if next != scope.exit {
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
        if !windows.is_empty() && branch_count == 2 && reached.len() == 1 {
            proof.successors.insert(site, reached[0]);
        }
    }
    Some(proof)
}

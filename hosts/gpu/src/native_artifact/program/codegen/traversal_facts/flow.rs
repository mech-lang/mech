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
    if slots.len() > MAX_WORDS {
        return None;
    }
    let ordered = scope.order(code)?;
    let mut terms = Terms::default();
    let mut initial = State::default();
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
            Step::Keep => {}
        }
        if site == scope.latch {
            continue;
        }
        for (branch, next) in scope::successors(block.terminator())?
            .into_iter()
            .enumerate()
        {
            if next == scope.exit {
                continue;
            }
            let target = incoming.get_mut(&(next as usize))?;
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
                            state.assume(
                                &terms,
                                if branch == 0 {
                                    Relation::Eq
                                } else {
                                    Relation::Ne
                                },
                                a,
                                b,
                            )?
                        }
                    }
                    _ => true,
                };
                if !reachable {
                    continue;
                }
                if let Some(target) = target {
                    target.merge(&state, next as usize, &mut terms)?;
                } else {
                    *target = Some(state);
                }
            }
        }
    }
    Some(proof)
}

//! Fresh entry partitions; completion gates explicitly admit their valid tags.
use super::*;

pub(super) fn states<'a>(
    code: CodePlan<'_>,
    ordered: &[usize],
    slots: &BTreeSet<usize>,
    tag: Option<usize>,
    windows: &[Window],
    captures: &[CapturedWindow],
    completion: Option<&completion::Candidate>,
    step: &impl Fn(usize) -> Option<Step<'a>>,
) -> Option<(Terms, Vec<State>)> {
    if slots.len() > MAX_WORDS {
        return None;
    }
    let mut terms = Terms::default();
    terms.constant(0)?;
    terms.constant(1)?;
    let mut initial = State::default();
    initial.windows.extend_from_slice(windows);
    initial.captures.extend_from_slice(captures);
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
    for window in captures {
        let k = initial.read(window.induction)?;
        let limit = initial.read(window.limit)?;
        initial.assume(&terms, Relation::Le, k, limit)?;
    }
    if let Some(candidate) = completion {
        terms.recording = Some(expressions::Recording::new(candidate.clone()));
        let mut partitions = Vec::new();
        let tag = initial.read(candidate.tag)?;
        for ordinal in [candidate.active, candidate.terminal] {
            let mut state = initial.clone();
            let value = terms.constant(ordinal)?;
            state.assume(&terms, Relation::Eq, tag, value)?;
            partitions.push(state);
        }
        return Some((terms, partitions));
    }
    let tag = tag.and_then(|slot| initial.read(slot));
    let mut ordinals = BTreeSet::new();
    for &site in ordered {
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
    Some((terms, partitions))
}

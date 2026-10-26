//! Plan-local header continuations, not acquisition or a second FSM evaluator.
//! Only the emitted Mech body reads/transitions its scalar aggregate carrier.
use super::{iteration_abi::NativeIterationVisit, program_borrowed_layout};
use mech_engine::native_artifact::{
    NativeControlLoop, NativeProgramArtifactPlan, NativeTailLoop, NativeTextLineRetentionFrontier,
};

#[derive(Debug, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Region {
    pub frontier: NativeTextLineRetentionFrontier,
    pub tail: NativeTailLoop,
    pub borrowed: usize,
    #[allow(
        dead_code,
        reason = "build-only descriptors retain the runtime carrier offset in the shared image layout"
    )]
    pub carrier: usize,
    pub header_work: u64,
}

/// Descriptor preparation supplies its freshly derived, same-plan inventory;
/// sharing it with identity hashing must not discard unsupported frontiers.
pub(super) fn derive_with_frontiers(
    plan: &NativeProgramArtifactPlan,
    borrowed: &program_borrowed_layout::Layout,
    visits: &[NativeIterationVisit],
    aggregates: &mech_engine::native_artifact::NativeScalarAggregates,
    frontiers: &[NativeTextLineRetentionFrontier],
) -> Result<Option<Region>, &'static str> {
    let [frontier] = frontiers else {
        return Ok(None);
    };
    let NativeControlLoop::Tail(tail) = &plan.iterations()[frontier.iteration() as usize] else {
        return Ok(None);
    };
    let [ordinal] = frontier.read().count().lines().borrowed_regions() else {
        return Ok(None);
    };
    let ordinal = *ordinal as usize;
    if tail.parent().is_some()
        || borrowed.regions[ordinal].proof.iteration() != frontier.iteration()
    {
        return Ok(None);
    }
    let mut seen = vec![false; visits.len()];
    let mut pending = vec![plan.entry()];
    while let Some(site) = pending.pop() {
        if site == tail.initialize() || core::mem::replace(&mut seen[site as usize], true) {
            continue;
        }
        if visits[site as usize].is_completion() {
            return Ok(None);
        }
        pending.extend(visits[site as usize].successors().into_iter().flatten());
    }
    let Some((_carrier, width)) = aggregates.slot(tail.carrier()) else {
        return Ok(None);
    };
    if frontier.states().iter().any(|state| {
        state.index_field().is_some_and(|field| field >= width)
            || (state.returned_range().is_none()
                && (state.index_field().is_none() || state.keep_fields().is_empty()))
            || state.keep_fields().iter().any(|field| *field >= width)
            || state
                .returned_range()
                .is_some_and(|(first, end)| first >= width || end >= width)
    }) {
        return Err("prefix carrier fields exceed their proved storage");
    }
    let retention_work = frontier
        .states()
        .iter()
        .try_fold(0u64, |work, state| {
            work.checked_add((state.keep_fields().len() as u64).checked_mul(32)?)
        })
        .ok_or("prefix retention work overflows")?;
    let header_work = (frontier.states().len() as u64)
        .checked_mul(128)
        .and_then(|n| n.checked_add(retention_work))
        .and_then(|n| {
            n.checked_add(
                ((borrowed.regions[ordinal].proof.matrices().len()
                    + borrowed.regions[ordinal].proof.literals().len()
                    + borrowed.regions[ordinal].proof.values().len()) as u64)
                    .checked_mul(32)?,
            )
        })
        .and_then(|n| n.checked_add(512))
        .ok_or("prefix header work overflows")?;
    Ok(Some(Region {
        frontier: frontier.clone(),
        tail: tail.clone(),
        borrowed: ordinal,
        carrier: _carrier,
        header_work,
    }))
}

pub(super) fn extra_work(region: Option<&Region>, site: usize) -> u64 {
    let Some(region) = region else {
        return 0;
    };
    if region.tail.header() as usize == site || region.tail.exit() as usize == site {
        return region.header_work;
    }
    let count = region.frontier.read().count();
    if count.lines().lengths().contains(&(site as u32))
        || count.aliases().contains(&(site as u32))
        || count
            .guards()
            .iter()
            .any(|guard| guard.comparison() as usize == site)
    {
        128
    } else {
        0
    }
}

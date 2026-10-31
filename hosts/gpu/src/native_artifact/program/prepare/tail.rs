//! Closed native Tail regions derived from the canonical CFG and representation
//! proofs. One acyclic body traversal has a finite conservative instruction cost.
use crate::native_artifact::iteration_abi::NativeIterationVisit;
use crate::native_artifact::program::prepare::borrowed as program_borrowed_layout;
use mech_engine::native_artifact::{
    NativeBorrowedStringAction as View, NativeControlLoop, NativeIterationAction as Action,
    NativeMatchAction as Scalar, NativeProgramArtifactPlan, NativeTailLoop,
};

pub(in crate::native_artifact) const OVERHEAD: u64 = 512;
// Charged on every traversal, including the terminal one. The packed emitter
// checks setup, tag dispatch and shared boundary templates against this lane.
pub(in crate::native_artifact) const FIXED_OVERHEAD: u64 = 1024;
#[derive(Debug, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Region {
    pub iteration: u32,
    pub tail: NativeTailLoop,
    pub borrowed: Option<usize>,
    /// Exclusive retained-map owner. Its private storage callbacks may debit
    /// the same account; scalar body work is prepaid in bounded batches.
    pub map: Option<usize>,
    pub numeric: Option<super::readonly_numeric::Proof>,
    /// Invariant key width only; actual packed storage requires runtime entry.
    pub packed: Option<crate::native_artifact::program::prepare::packed_map::Proof>,
    // Emission and checkpoint validation use the identical admitted word lane.
    pub words: usize,
    /// Numeric-only scratch. Readiness/fuel always retain private word homes.
    pub shadow: Option<usize>,
    pub compute: u64,
    pub comparison: u64,
}
#[derive(Debug, Default, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
    pub sites: Vec<Option<usize>>,
    pub initializers: Vec<Option<usize>>,
    pub exits: Vec<Option<usize>>,
    pub words: usize,
    /// Original independent-bank count for unchanged setup work/admission.
    pub logical_words: usize,
}
impl Layout {
    pub fn derive(
        plan: &NativeProgramArtifactPlan,
        borrowed: &program_borrowed_layout::Layout,
        visits: &[NativeIterationVisit],
        aggregates: &mech_engine::native_artifact::NativeScalarAggregates,
        owners: &mech_engine::native_artifact::NativeMapOwnerWords,
        frames: &mech_engine::native_artifact::NativeBufferFrames,
        calls: &[Option<mech_engine::native_artifact::NativePreparedLibraryCall>],
    ) -> Result<Self, &'static str> {
        let mut layout = Self {
            sites: vec![None; plan.blocks().len()],
            initializers: vec![None; plan.blocks().len()],
            exits: vec![None; plan.blocks().len()],
            ..Self::default()
        };
        let base = plan
            .values()
            .len()
            .checked_add(plan.iterations().len())
            .and_then(|n| n.checked_add(aggregates.words()))
            .and_then(|n| n.checked_add(borrowed.words))
            .ok_or("Tail word count overflow")?;
        for (iteration, control) in plan.iterations().iter().enumerate() {
            let NativeControlLoop::Tail(tail) = control else {
                continue;
            };
            let map = owners
                .initializer(tail.initialize() as usize)
                .filter(|&owner| {
                    let spans = owners
                        .spans()
                        .iter()
                        .enumerate()
                        .filter(|(_, span)| span.region() == owner)
                        .collect::<Vec<_>>();
                    // One non-escaping projection over an invariant String. The
                    // source body still computes every range endpoint and value.
                    spans.len() == 1
                        && owners.regions()[owner]
                            .reads()
                            .iter()
                            .chain(owners.regions()[owner].writes())
                            .all(|&site| owners.call_span(site as usize) == Some(spans[0].0))
                });
            // A closed map body may run inside an ordinary comprehension.
            // Its own body must still contain no child loop; the parent is
            // suspended until this private owner publishes at the Tail exit.
            let numeric = if map.is_none() && frames.action(tail.initialize() as usize).is_some() {
                super::readonly_numeric::derive(plan, tail, frames, aggregates, calls).ok()
            } else {
                None
            };
            if (tail.parent().is_some() && map.is_none() && numeric.is_none())
                || (map.is_none()
                    && numeric.is_none()
                    && aggregates.action(tail.initialize() as usize).is_none())
            {
                continue;
            }
            let mut sites = Vec::new();
            let mut seen = vec![false; plan.blocks().len()];
            let mut pending = vec![tail.header()];
            let mut closed = true;
            while let Some(site) = pending.pop() {
                if site == tail.exit() || seen[site as usize] {
                    continue;
                }
                seen[site as usize] = true;
                let block = &plan.blocks()[site as usize];
                let native = numeric
                    .as_ref()
                    .is_some_and(|p| p.action(site as usize).is_some())
                    || aggregates.action(site as usize).is_some()
                    || borrowed.action(site as usize).is_some()
                    || map.is_some_and(|owner| {
                        owners.action_region(site as usize) == Some(owner)
                            || owners.call_region(site as usize) == Some(owner)
                            || owners
                                .span_site(site as usize)
                                .is_some_and(|id| owners.spans()[id].region() == owner)
                    })
                    || matches!(
                        block.action(),
                        Action::Scalar(Scalar::None | Scalar::Primitive { .. })
                    )
                    || matches!(block.action(), Action::Scalar(Scalar::Copy { output, .. })
                        if plan.values()[output.get() as usize].scalar_kind().is_some());
                if block.iteration() != Some(iteration as u32)
                    || !native
                    || (map.is_some() && borrowed.action(site as usize).is_some())
                    || plan.activation_boundary() == Some(site)
                {
                    closed = false;
                    break;
                }
                for next in visits[site as usize].successors.into_iter().flatten() {
                    if next == tail.exit() && site != tail.finish() {
                        closed = false;
                    }
                    pending.push(next);
                }
                sites.push(site);
            }
            if !closed || !seen[tail.finish() as usize] || !seen[tail.latch() as usize] {
                continue;
            }
            // Removing the sole declared latch backedge must make the region a
            // DAG. Then summing every block bounds any one header-to-boundary path.
            let mut incoming = vec![0usize; plan.blocks().len()];
            for &site in &sites {
                for next in visits[site as usize].successors.into_iter().flatten() {
                    if next != tail.exit() && !(site == tail.latch() && next == tail.header()) {
                        incoming[next as usize] += 1;
                    }
                }
            }
            let mut ready = sites
                .iter()
                .copied()
                .filter(|s| incoming[*s as usize] == 0)
                .collect::<Vec<_>>();
            let mut count = 0;
            while let Some(site) = ready.pop() {
                count += 1;
                for next in visits[site as usize].successors.into_iter().flatten() {
                    if next == tail.exit() || (site == tail.latch() && next == tail.header()) {
                        continue;
                    }
                    incoming[next as usize] -= 1;
                    if incoming[next as usize] == 0 {
                        ready.push(next);
                    }
                }
            }
            if count != sites.len() {
                continue;
            }
            let mut compute = FIXED_OVERHEAD;
            let mut comparison = 0u64;
            for &site in &sites {
                compute = compute
                    .checked_add(visits[site as usize].work)
                    .and_then(|n| n.checked_add(OVERHEAD))
                    .ok_or("Tail body work overflow")?;
                if matches!(
                    borrowed.action(site as usize),
                    Some((_, View::Equal { .. }))
                ) {
                    compute = compute
                        .checked_add(32)
                        .ok_or("Tail equality work overflow")?;
                    comparison = comparison
                        .checked_add(32)
                        .ok_or("Tail comparison work overflow")?;
                }
                if numeric
                    .as_ref()
                    .and_then(|p| p.action(site as usize))
                    .is_some_and(|a| matches!(a, super::readonly_numeric::Operation::Write { .. }))
                {
                    comparison = comparison
                        .checked_add(1)
                        .ok_or("Tail store comparison work overflow")?;
                }
            }
            if let Some(fields) = numeric
                .as_ref()
                .and_then(|p| p.result.frame().and_then(|value| p.fields(value)))
            {
                // Current result fields synchronize once at exit, in the
                // already admitted frame lanes. Conservatively fund that
                // bounded work on every traversal, including the terminal one.
                compute = compute
                    .checked_add(
                        (fields.len() as u64)
                            .checked_mul(64)
                            .ok_or("numeric result work overflow")?,
                    )
                    .ok_or("numeric result synchronization work overflow")?;
            }
            let index = layout.regions.len();
            for site in sites {
                layout.sites[site as usize] = Some(index);
            }
            layout.initializers[tail.initialize() as usize] = Some(index);
            layout.exits[tail.exit() as usize] = Some(index);
            // Overflow validation remains unconditional, including builds
            // without the code generator that consumes the concrete offset.
            let _word_offset = base
                .checked_add(layout.words)
                .ok_or("Tail words overflow")?;
            layout.regions.push(Region {
                iteration: iteration as u32,
                tail: tail.clone(),
                borrowed: borrowed.initializers[tail.initialize() as usize],
                map,
                packed: map.and_then(|owner| {
                    crate::native_artifact::program::prepare::packed_map::derive(
                        plan, &owners, owner, visits,
                    )
                }),
                numeric,
                words: _word_offset,
                shadow: None,
                compute,
                comparison,
            });
            layout.logical_words = layout
                .logical_words
                .checked_add(
                    if map.is_some() { 5 } else { 3 }
                        + layout
                            .regions
                            .last()
                            .and_then(|r| r.numeric.as_ref())
                            .map_or(0, |p| p.words),
                )
                .ok_or("Tail words overflow")?;
            layout.words = layout
                .words
                .checked_add(if map.is_some() { 5 } else { 3 })
                .ok_or("Tail state words overflow")?;
        }
        // Every selected numeric body is child-free. The runtime admits only
        // one closed loan and reconciles it before ordinary/parent execution.
        // Sharing the flags too would let global abort export the wrong region.
        let maximum = layout
            .regions
            .iter()
            .filter_map(|r| r.numeric.as_ref().map(|p| p.words))
            .max()
            .unwrap_or(0);
        let shadow = base
            .checked_add(layout.words)
            .ok_or("numeric shadow offset overflows")?;
        for region in &mut layout.regions {
            if region.numeric.is_some() {
                region.shadow = Some(shadow);
            }
        }
        layout.words = layout
            .words
            .checked_add(maximum)
            .ok_or("numeric shadow size overflows")?;
        Ok(layout)
    }
    pub fn extra_work(&self, site: usize) -> u64 {
        if self.sites.get(site).copied().flatten().is_some()
            || self.initializers.get(site).copied().flatten().is_some()
            || self.exits.get(site).copied().flatten().is_some()
        {
            OVERHEAD
        } else {
            0
        }
    }
}

/// Independently reconstruct all raw-view offsets, carrier/factory proofs,
/// boundaries and finite body prices from the validated plan. Deserialized
/// closed-tail metadata never grants new numeric access authority.
pub(in crate::native_artifact) fn validate_image(
    plan: &NativeProgramArtifactPlan,
    frames: &mech_engine::native_artifact::NativeBufferFrames,
    serialized_aggregates: &mech_engine::native_artifact::NativeScalarAggregates,
    calls: &[Option<mech_engine::native_artifact::NativePreparedLibraryCall>],
    serialized: &Layout,
) -> Result<(), String> {
    let facts = plan.facts();
    let aggregates = facts.scalar_aggregates();
    // The numeric result proof and the emitted shared aggregate actions use
    // these same global lanes. A rederived region cannot validate a different
    // serialized word bank, even if that bank has a plausible total size.
    if serde_json::to_vec(&aggregates).map_err(|e| e.to_string())?
        != serde_json::to_vec(serialized_aggregates).map_err(|e| e.to_string())?
    {
        return Err(
            "scalar aggregate lanes differ from independently validated source uses".into(),
        );
    }
    let borrowed = super::borrowed::Layout::derive(plan, &aggregates)?;
    let selected = super::selected_join::Layout::derive(plan, &plan.string_map_regions())?;
    let mut visits = crate::native_artifact::iteration_abi::control_visits(
        plan.blocks(),
        plan.iterations().iter().map(Into::into),
        0,
    )
    .map_err(str::to_owned)?;
    selected
        .fund_visit_checks(&mut visits)
        .map_err(str::to_owned)?;
    let prefix = super::prefix::derive_with_frontiers(
        plan,
        &borrowed,
        &visits,
        &aggregates,
        &facts.text_line_retention_frontiers(),
    )?;
    let owners = facts.map_owner_words().map_err(str::to_owned)?;
    for (site, visit) in visits.iter_mut().enumerate() {
        visit.work = visit
            .work
            .checked_add(borrowed.extra_work(site)?)
            .and_then(|n| n.checked_add(super::prefix::extra_work(prefix.as_ref(), site)))
            .and_then(|n| n.checked_add(super::map_owner::extra_work(&owners, site).ok()?))
            .ok_or("restored closed-tail price overflows")?;
    }
    let proved = Layout::derive(
        plan,
        &borrowed,
        &visits,
        &aggregates,
        &owners,
        frames,
        calls,
    )?;
    if serde_json::to_vec(&proved).map_err(|e| e.to_string())?
        != serde_json::to_vec(serialized).map_err(|e| e.to_string())?
    {
        return Err(
            "closed-tail proof differs from independently validated source dataflow and prices"
                .into(),
        );
    }
    Ok(())
}

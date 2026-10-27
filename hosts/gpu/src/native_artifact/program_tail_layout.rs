//! Closed native Tail regions derived from the canonical CFG and representation
//! proofs. One acyclic body traversal has a finite conservative instruction cost.
use super::{iteration_abi::NativeIterationVisit, program_borrowed_layout};
use mech_engine::native_artifact::{
    NativeBorrowedStringAction as View, NativeControlLoop, NativeIterationAction as Action,
    NativeMatchAction as Scalar, NativeProgramArtifactPlan, NativeTailLoop,
};

pub(super) const OVERHEAD: u64 = 512;
// Charged on every traversal, including the terminal one. The packed emitter
// checks setup, tag dispatch and shared boundary templates against this lane.
pub(super) const FIXED_OVERHEAD: u64 = 1024;
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Region {
    pub iteration: u32,
    #[serde(deserialize_with = "super::program_descriptor::image::proof")]
    pub tail: NativeTailLoop,
    pub borrowed: Option<usize>,
    /// Exclusive retained-map owner. Its private storage callbacks may debit
    /// the same account; scalar body work is prepaid in bounded batches.
    pub map: Option<usize>,
    /// Invariant key width only; actual packed storage requires runtime entry.
    pub packed: Option<super::program_map_packed_layout::Proof>,
    // Emission and checkpoint validation use the identical admitted word lane.
    pub words: usize,
    pub compute: u64,
    pub comparison: u64,
}
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Layout {
    pub regions: Vec<Region>,
    pub sites: Vec<Option<usize>>,
    pub initializers: Vec<Option<usize>>,
    pub exits: Vec<Option<usize>>,
    pub words: usize,
}
impl Layout {
    pub fn derive(
        plan: &NativeProgramArtifactPlan,
        borrowed: &program_borrowed_layout::Layout,
        visits: &[NativeIterationVisit],
    ) -> Result<Self, &'static str> {
        let mut layout = Self {
            sites: vec![None; plan.blocks().len()],
            initializers: vec![None; plan.blocks().len()],
            exits: vec![None; plan.blocks().len()],
            ..Self::default()
        };
        let aggregates = plan.scalar_aggregates();
        let owners = plan.map_owner_words()?;
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
            if (tail.parent().is_some() && map.is_none())
                || (map.is_none() && aggregates.action(tail.initialize() as usize).is_none())
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
                let native = aggregates.action(site as usize).is_some()
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
                    super::program_map_packed_layout::derive(plan, &owners, owner, visits)
                }),
                words: _word_offset,
                compute,
                comparison,
            });
            layout.words = layout
                .words
                .checked_add(if map.is_some() { 5 } else { 3 })
                .ok_or("Tail words overflow")?;
        }
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

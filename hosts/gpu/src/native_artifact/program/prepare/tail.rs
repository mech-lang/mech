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
    /// One independently priced structured subtree shares the root loan and
    /// scalar/view bank. Children have no successful callback boundary.
    pub numeric_tree: Option<super::numeric_tree::Proof>,
    /// Immutable enclosing-invocation matrices used by a scalar-state loop.
    pub captures: Option<super::captured_numeric::Proof>,
    /// Invariant key width only; actual packed storage requires runtime entry.
    pub packed: Option<crate::native_artifact::program::prepare::packed_map::Proof>,
    // Emission and checkpoint validation use the identical admitted word lane.
    pub words: usize,
    /// Numeric-only scratch. Readiness/fuel always retain private word homes.
    pub shadow: Option<usize>,
    pub compute: u64,
    pub comparison: u64,
}
impl Region {
    #[cfg(feature = "canonical-native-program")]
    pub fn numeric_subtree(&self) -> bool {
        self.numeric_tree.is_some()
            || self
                .numeric
                .as_ref()
                .is_some_and(|proof| proof.phase.is_some())
    }
    /// Ordinary children may use the phases' shared scratch bank. Pending
    /// continuation identity therefore lives beside this region's fuel.
    pub fn cold_resume(&self) -> Option<usize> {
        self.numeric
            .as_ref()
            .is_some_and(|proof| !proof.exits.is_empty())
            .then(|| self.words + 3)
    }
    fn state_words(&self) -> usize {
        if self.map.is_some() {
            5
        } else {
            3 + usize::from(self.cold_resume().is_some())
        }
    }
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
        fixed_keys: &super::fixed_key_loops::Layout,
    ) -> Result<Self, &'static str> {
        let discovery = super::numeric_tree::Discovery::new(plan)?;
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
        let mut secondary_sites = vec![false; plan.blocks().len()];
        for (iteration, control) in plan.iterations().iter().enumerate() {
            let NativeControlLoop::Tail(tail) = control else {
                continue;
            };
            // Primary source ownership remains outermost. A complete child
            // may additionally enter after its partial parent exported a cold
            // continuation; never select overlapping secondary children.
            let primary = layout.sites[tail.header() as usize].filter(|&id| {
                layout.regions[id]
                    .numeric
                    .as_ref()
                    .is_some_and(|proof| !proof.exits.is_empty() || !proof.prepaid_sites.is_empty())
            });
            if secondary_sites[tail.header() as usize]
                || primary.is_some_and(|id| {
                    !layout.regions[id]
                        .numeric
                        .as_ref()
                        .is_some_and(|proof| !proof.exits.is_empty())
                })
            {
                continue;
            }
            let map = owners
                .initializer(tail.initialize() as usize)
                .filter(|&owner| {
                    // Compound Map results use checked constructor admission
                    // at their ordinary source sites. The existing packed
                    // closed loop has no such constructor boundary yet.
                    if !matches!(
                        plan.values()[tail.returned().get() as usize]
                            .definition()
                            .body(),
                        mech_core::SchemaBody::Map { .. }
                    ) {
                        return false;
                    }
                    let spans = owners
                        .spans()
                        .iter()
                        .enumerate()
                        .filter(|(_, span)| span.region() == owner)
                        .collect::<Vec<_>>();
                    // One non-escaping projection over an invariant String. The
                    // source body still computes every range endpoint and value.
                    fixed_keys.owner(owner).is_some()
                        || (spans.len() == 1
                            && owners.regions()[owner]
                                .reads()
                                .iter()
                                .chain(owners.regions()[owner].writes())
                                .all(|&site| owners.call_span(site as usize) == Some(spans[0].0)))
                });
            // A closed map body may run inside an ordinary comprehension.
            // Its own body must still contain no child loop; the parent is
            // suspended until this private owner publishes at the Tail exit.
            let mut numeric_tree = None;
            // Hoisting immutable matrix arguments can leave a scalar-only
            // carrier. The numeric proof also owns such trees: requiring a
            // managed-frame initializer here would strand their array reads
            // on the ordinary callback path. The proof itself rejects loops
            // without arrays, preserving the established scalar-only entry.
            let phase = (map.is_none())
                .then(|| {
                    super::readonly_numeric::derive(plan, tail, frames, aggregates, calls, visits)
                        .ok()
                })
                .flatten()
                .filter(|proof| proof.phase.is_some() || !proof.exits.is_empty());
            // A readonly partial phase may escape to an ordinary child on
            // every visit. A complete readonly tree can share one loan without
            // adding first-change ownership or publication state. Mutable and
            // current-funded phases keep their established compact transport.
            let partial_phase = phase.as_ref().is_some_and(|proof| {
                !proof.writes()
                    && proof
                        .exits
                        .iter()
                        .any(|exit| matches!(exit.kind, super::numeric_plan::ExitKind::Unsupported))
            });
            let mut numeric = if phase.is_some() && !partial_phase {
                phase
            } else if map.is_none() {
                if let Ok(tree) = discovery.discover(tail) {
                    match super::readonly_numeric::derive_tree(
                        plan, tail, frames, aggregates, calls, &tree,
                    ) {
                        Ok(proof) if !partial_phase || !proof.writes() => {
                            numeric_tree = Some(tree);
                            Some(proof)
                        }
                        Ok(_) => phase,
                        Err(_) => phase.or_else(|| {
                            super::readonly_numeric::derive(
                                plan, tail, frames, aggregates, calls, visits,
                            )
                            .ok()
                        }),
                    }
                } else {
                    phase.or_else(|| {
                        super::readonly_numeric::derive(
                            plan, tail, frames, aggregates, calls, visits,
                        )
                        .ok()
                    })
                }
            } else {
                None
            };
            let captures = if map.is_none()
                && numeric.is_none()
                && aggregates.slot(tail.carrier()).is_some()
                && borrowed.initializers[tail.initialize() as usize].is_none()
            {
                super::captured_numeric::derive(plan, tail, calls).ok()
            } else {
                None
            };
            if let Some(owner) = primary {
                let parent = layout.regions[owner].numeric.as_ref().unwrap();
                if map.is_some()
                    || layout.initializers[tail.initialize() as usize].is_some()
                    || layout.exits[tail.exit() as usize].is_some()
                    || !numeric.as_ref().is_some_and(|proof| {
                        proof.exits.is_empty()
                            && proof.prepaid_sites.is_empty()
                            && proof
                                .sites
                                .iter()
                                .all(|site| parent.sites.binary_search(site).is_ok())
                    })
                {
                    continue;
                }
            }
            let successors = |site: usize| {
                if numeric.as_ref().is_some_and(|proof| {
                    proof.exits.iter().any(|exit| {
                        exit.to == site
                            && matches!(exit.kind, super::numeric_plan::ExitKind::Unsupported)
                    })
                }) {
                    return Vec::new();
                }
                numeric
                    .as_ref()
                    .and_then(|proof| proof.phase.as_ref())
                    .and_then(|phase| phase.cold.iter().find(|child| child.initialize == site))
                    .map_or_else(
                        || {
                            visits[site]
                                .successors
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                        },
                        |child| vec![child.exit as u32],
                    )
            };
            if (tail.parent().is_some() && map.is_none() && numeric.is_none() && captures.is_none())
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
                    || captures
                        .as_ref()
                        .is_some_and(|p| p.action(site as usize).is_some())
                    || aggregates.action(site as usize).is_some()
                    || borrowed.action(site as usize).is_some()
                    || map.is_some_and(|owner| {
                        fixed_keys
                            .owner(owner)
                            .is_some_and(|p| p.cell == site as usize)
                            || owners.action_region(site as usize) == Some(owner)
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
                if !(block.iteration() == Some(iteration as u32)
                    || numeric
                        .as_ref()
                        .is_some_and(|p| p.sites.contains(&(site as usize))))
                    || !native
                    || (map.is_some() && borrowed.action(site as usize).is_some())
                    || plan.activation_boundary() == Some(site)
                {
                    closed = false;
                    break;
                }
                for next in successors(site as usize) {
                    if next == tail.exit() && site != tail.finish() {
                        closed = false;
                    }
                    pending.push(next);
                }
                sites.push(site);
            }
            let partial = numeric
                .as_ref()
                .is_some_and(|proof| !proof.prepaid_sites.is_empty());
            if !closed
                || (!partial && (!seen[tail.finish() as usize] || !seen[tail.latch() as usize]))
            {
                continue;
            }
            // Cut all and only the declared backedges in the selected phase.
            let backedges = numeric.as_ref().and_then(|p| p.phase.as_ref()).map_or_else(
                || {
                    numeric_tree.as_ref().map_or_else(
                        || std::collections::BTreeSet::from([(tail.latch(), tail.header())]),
                        |tree| {
                            tree.nodes
                                .iter()
                                .map(|node| (node.tail.latch(), node.tail.header()))
                                .collect()
                        },
                    )
                },
                |phase| {
                    phase
                        .loops
                        .iter()
                        .map(|loop_| {
                            let NativeControlLoop::Tail(t) =
                                &plan.iterations()[loop_.iteration as usize]
                            else {
                                unreachable!()
                            };
                            (t.latch(), t.header())
                        })
                        .collect()
                },
            );
            let mut incoming = vec![0usize; plan.blocks().len()];
            for &site in &sites {
                for next in successors(site as usize) {
                    if next != tail.exit() && !backedges.contains(&(site, next)) {
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
                for next in successors(site as usize) {
                    if next == tail.exit() || backedges.contains(&(site, next)) {
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
            if let Some(proof) = &numeric {
                compute = proof.boundary_work;
            }
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
                if captures
                    .as_ref()
                    .and_then(|proof| proof.action(site as usize))
                    .is_some_and(|action| {
                        matches!(action, super::captured_numeric::Operation::Length { .. })
                    })
                {
                    comparison = comparison
                        .checked_add(1)
                        .ok_or("captured length comparison work overflow")?;
                }
            }
            if let Some((proof, carrier)) = numeric.as_ref().and_then(|proof| {
                proof
                    .frame(tail.carrier().get())
                    .map(|carrier| (proof, carrier))
            }) {
                if carrier.scalar_paths.iter().any(|(path, _)| path.len() > 1) {
                    let steps = carrier
                        .scalar_paths
                        .iter()
                        .try_fold(0u64, |n, (path, _)| n.checked_add(path.len() as u64))
                        .ok_or("numeric carrier paths overflow")?;
                    // Each array has at most one first-change bridge, followed
                    // by one successful/cold exit. Prepay the recursive scalar
                    // synchronization even on a one-traversal invocation.
                    let boundaries =
                        proof.arrays.iter().filter(|array| array.writable).count() as u64 + 1;
                    compute = steps
                        .checked_mul(boundaries)
                        .and_then(|n| n.checked_mul(64))
                        .and_then(|n| compute.checked_add(n))
                        .ok_or("numeric carrier synchronization work overflow")?;
                }
            }
            if let Some(frame) = numeric
                .as_ref()
                .and_then(|p| p.result.frame().and_then(|value| p.frame(value)))
            {
                // Current result fields synchronize once at exit, in the
                // already admitted frame lanes. Conservatively fund that
                // bounded work on every traversal, including the terminal one.
                compute = compute
                    .checked_add(
                        (frame
                            .scalar_paths
                            .iter()
                            .try_fold(frame.arrays.iter().flatten().count(), |count, (path, _)| {
                                count.checked_add(path.len())
                            })
                            .ok_or("numeric result path count overflows")?
                            as u64)
                            .checked_mul(64)
                            .ok_or("numeric result work overflow")?,
                    )
                    .ok_or("numeric result synchronization work overflow")?;
            }
            if let (Some(tree), Some(proof)) = (&mut numeric_tree, &mut numeric) {
                tree.counters = proof.words;
                proof.words = proof
                    .words
                    .checked_add(tree.words())
                    .ok_or("numeric tree counter words overflow")?;
                proof.boundary_words = proof
                    .boundary_words
                    .checked_add(tree.words())
                    .ok_or("numeric tree boundary words overflow")?;
                let mut synchronization = 0u64;
                for node in &mut tree.nodes {
                    if let Some(frame) = proof.frame(node.tail.carrier().get()) {
                        node.scalar_bindings = super::readonly_numeric::scalar_bindings(
                            frames,
                            frame.value,
                            &frame.scalar_paths,
                        )?;
                        if proof.nested_managed {
                            synchronization = synchronization
                                .checked_add(
                                    (frame.array_bindings.len() as u64).checked_mul(64).ok_or(
                                        "nested numeric owner synchronization work overflow",
                                    )?,
                                )
                                .ok_or("nested numeric owner synchronization work overflow")?;
                        }
                        synchronization = node
                            .scalar_bindings
                            .iter()
                            .try_fold(synchronization, |n, binding| {
                                n.checked_add((binding.steps as u64).checked_mul(64)?)
                            })
                            .ok_or("numeric tree carrier synchronization work overflow")?;
                    }
                }
                // Every possible first change and terminal/cold boundary may
                // reify the entire active ancestry. Price that static maximum
                // on each node traversal before entering its source body.
                let boundaries = proof.arrays.iter().filter(|a| a.writable).count() as u64 + 1;
                let templates = (tree.nodes.len() as u64)
                    .checked_mul(4)
                    .and_then(|n| n.checked_add(8))
                    .ok_or("numeric tree template count overflow")?;
                // Newly live scalar-frame words spill at each node bailout,
                // normal exit and failure. The selected header additionally
                // loads/exports each exact leaf. Count duplicate header words
                // conservatively; this bound is independent of emitted IR.
                let cold_scalar_templates = proof
                    .cold_scalars
                    .iter()
                    .try_fold(0u64, |n, (_, bindings)| {
                        n.checked_add(bindings.len() as u64)
                    })
                    .and_then(|n| n.checked_mul((tree.nodes.len() as u64).checked_add(4)?))
                    .ok_or("numeric tree cold scalar template work overflow")?;
                let boundary = synchronization
                    .checked_mul(boundaries)
                    .and_then(|n| {
                        n.checked_add((proof.boundary_words as u64).checked_mul(templates)?)
                    })
                    .and_then(|n| n.checked_add(FIXED_OVERHEAD))
                    .and_then(|n| n.checked_add(cold_scalar_templates))
                    .ok_or("numeric tree boundary price overflow")?;
                for (ordinal, node) in tree.nodes.iter_mut().enumerate() {
                    node.compute = boundary;
                    node.comparison = 0;
                    for &(site, owner) in &tree.sites {
                        if owner != ordinal {
                            continue;
                        }
                        node.compute = node
                            .compute
                            .checked_add(visits[site].work)
                            .and_then(|n| n.checked_add(OVERHEAD))
                            .ok_or("numeric tree node price overflow")?;
                        if proof.action(site).is_some_and(|a| {
                            matches!(a, super::readonly_numeric::Operation::Write { .. })
                        }) {
                            node.comparison = node
                                .comparison
                                .checked_add(1)
                                .ok_or("numeric tree comparison price overflow")?;
                        }
                    }
                }
                compute = tree.nodes[0].compute;
                comparison = tree.nodes[0].comparison;
                proof.boundary_work = boundary;
            } else if let Some(phase) = numeric.as_ref().and_then(|p| p.phase.as_ref()) {
                compute = phase.loops[0].compute;
                comparison = phase.loops[0].comparison;
            }
            let index = layout.regions.len();
            for site in sites {
                if primary.is_some() {
                    secondary_sites[site as usize] = true;
                } else {
                    layout.sites[site as usize] = Some(index);
                }
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
                numeric_tree,
                captures,
                words: _word_offset,
                shadow: None,
                compute,
                comparison,
            });
            layout.logical_words = layout
                .logical_words
                .checked_add(
                    layout.regions.last().unwrap().state_words()
                        + layout
                            .regions
                            .last()
                            .and_then(|r| r.numeric.as_ref())
                            .map_or_else(
                                || {
                                    layout
                                        .regions
                                        .last()
                                        .and_then(|r| r.captures.as_ref())
                                        .map_or(0, |p| p.words())
                                },
                                |p| p.words,
                            ),
                )
                .ok_or("Tail words overflow")?;
            layout.words = layout
                .words
                .checked_add(layout.regions.last().unwrap().state_words())
                .ok_or("Tail state words overflow")?;
        }
        // Distinct phases never overlap at runtime. Each includes its own
        // simultaneously live parent/child homes. The runtime admits only
        // one closed loan and reconciles it before ordinary/parent execution.
        // Sharing the flags too would let global abort export the wrong region.
        let maximum = layout
            .regions
            .iter()
            .map(|r| {
                r.numeric
                    .as_ref()
                    .map_or_else(|| r.captures.as_ref().map_or(0, |p| p.words()), |p| p.words)
            })
            .max()
            .unwrap_or(0);
        let shadow = base
            .checked_add(layout.words)
            .ok_or("numeric shadow offset overflows")?;
        for region in &mut layout.regions {
            if region.numeric.is_some() || region.captures.is_some() {
                region.shadow = Some(shadow);
            }
        }
        layout.words = layout
            .words
            .checked_add(maximum)
            .ok_or("numeric shadow size overflows")?;
        Ok(layout)
    }
    /// Find the independently reconstructed entry and readiness home for a header.
    #[cfg(feature = "canonical-native-program")]
    pub fn header(&self, site: usize) -> Option<usize> {
        self.regions
            .iter()
            .position(|region| region.tail.header() as usize == site)
    }
    pub fn extra_work(&self, site: usize) -> u64 {
        if self.sites.get(site).copied().flatten().is_some()
            || self.initializers.get(site).copied().flatten().is_some()
            || self.exits.get(site).copied().flatten().is_some()
            || self.prepaid_site(site)
        {
            OVERHEAD
        } else {
            0
        }
    }
    pub fn prepaid_site(&self, site: usize) -> bool {
        self.regions
            .iter()
            .filter_map(|region| region.numeric.as_ref())
            .any(|proof| proof.prepaid_sites.binary_search(&site).is_ok())
    }
}

/// Independently reconstruct all raw-view offsets, carrier/factory proofs,
/// boundaries and finite body prices from the validated plan. Deserialized
/// closed-tail metadata never grants new numeric access authority.
pub(in crate::native_artifact) fn validate_image(
    plan: &NativeProgramArtifactPlan,
    frames: &mech_engine::native_artifact::NativeBufferFrames,
    owners: &mech_engine::native_artifact::NativeMapOwnerWords,
    serialized_aggregates: &mech_engine::native_artifact::NativeScalarAggregates,
    calls: &[Option<mech_engine::native_artifact::NativePreparedLibraryCall>],
    serialized: &Layout,
    fixed_keys: &super::fixed_key_loops::Layout,
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
    for (site, visit) in visits.iter_mut().enumerate() {
        visit.work = visit
            .work
            .checked_add(borrowed.extra_work(site)?)
            .and_then(|n| n.checked_add(super::prefix::extra_work(prefix.as_ref(), site)))
            .and_then(|n| n.checked_add(super::map_owner::extra_work(owners, site).ok()?))
            .ok_or("restored closed-tail price overflows")?;
    }
    let proved = Layout::derive(
        plan,
        &borrowed,
        &visits,
        &aggregates,
        owners,
        frames,
        calls,
        fixed_keys,
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

//! Checked parent/child boundaries. This inventory does not authorize execution:
//! owner forwarding, finite settlement and continuation emission remain required.
use super::{readonly_numeric::ResultRecipe, tail};
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame, NativeBufferFrames,
    NativeCollectionPrimitive as Primitive, NativeControlLoop as Loop,
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativePreparedLibraryCall, NativeProgramArtifactPlan as Plan,
    NativeScalarAggregates,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroUsize;
pub(in crate::native_artifact) mod frames;
pub(in crate::native_artifact) mod image;
pub(in crate::native_artifact) mod numeric;

#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Segment {
    pub entry: u32,
    pub sites: Vec<u32>,
    /// Exact last executed source site and its next, still ordinary frontier.
    pub exits: Vec<(u32, u32)>,
    pub frame_plan: Option<frames::Layout>,
    pub numeric: Option<numeric::Layout>,
    /// Exact root traversal whose source visits can already have been paid.
    /// The invocation must still hold that root's current cold continuation.
    pub prepaid_region: Option<usize>,
    pub rejection: Option<std::borrow::Cow<'static, str>>,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Parent {
    pub iteration: u32,
    /// Innermost suspended ancestor edge, rederived from validated loop ancestry
    /// after final segment selection. Runtime compares the current marker.
    pub suspended_child: Option<u32>,
    pub children: Vec<Child>,
    pub segments: Vec<Segment>,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Child {
    pub child: u32,
    pub child_initialize: u32,
    /// Collection initialization waits for its ordinary dynamic-count helper;
    /// counted/Tail initialization is already active after the source visit.
    pub initialize_progress: u64,
    pub child_exit: u32,
    pub child_successor: u32,
    /// Existing scalar-child classification and returned-array proof evidence.
    /// All child boundaries stay ordinary, including unselected/collection
    /// children. Neither field grants an owner, view or fuel across a call.
    pub scalar_child: bool,
    /// Returned tuple/record field -> CURRENT child carrier array field.
    pub returned_arrays: Vec<(usize, usize)>,
}
#[derive(Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub parents: Vec<Parent>,
    #[serde(skip)]
    entries: Box<[Option<NonZeroUsize>]>,
    #[serde(skip)]
    regions: Vec<(usize, usize)>,
    #[serde(skip)]
    boundaries: Vec<(usize, usize)>,
    /// Nearest tracked edge at each native root, derived once from ancestry.
    #[serde(skip)]
    native_contexts: Box<[Option<u32>]>,
    #[serde(skip)]
    pub words: usize,
}
impl Layout {
    pub(in crate::native_artifact) fn native_context(&self, region: usize) -> Option<Option<u32>> {
        self.native_contexts.get(region).copied()
    }
    pub(in crate::native_artifact) fn entry(&self, site: usize) -> Option<usize> {
        let encoded = self.entries.get(site)?.map(|v| v.get() - 1)?;
        (encoded % 4 == 0).then_some(encoded / 4)
    }
    /// 1 arms the child edge; 2 identifies the completed child exit.
    /// Both reuse the existing immutable site map; no second per-site bank.
    pub(in crate::native_artifact) fn boundary(&self, site: usize) -> Option<(usize, bool)> {
        let encoded = self.entries.get(site)?.map(|v| v.get() - 1)?;
        match encoded % 4 {
            1 => Some((encoded / 4, true)),
            2 => Some((encoded / 4, false)),
            _ => None,
        }
    }
    pub(in crate::native_artifact) fn region(&self, index: usize) -> Option<(&Parent, &Segment)> {
        let &(parent, segment) = self.regions.get(index)?;
        let parent = self.parents.get(parent)?;
        Some((parent, parent.segments.get(segment)?))
    }
    pub(in crate::native_artifact) fn child_boundary(
        &self,
        index: usize,
    ) -> Option<(&Parent, &Child)> {
        let &(parent, child) = self.boundaries.get(index)?;
        let parent = self.parents.get(parent)?;
        Some((parent, parent.children.get(child)?))
    }
    pub(in crate::native_artifact) fn argument_words(&self) -> usize {
        self.regions
            .iter()
            .map(|&(p, s)| {
                let segment = &self.parents[p].segments[s];
                segment
                    .numeric
                    .as_ref()
                    .unwrap()
                    .words
                    .max(segment.frame_plan.as_ref().unwrap().sources.len())
            })
            .max()
            .unwrap_or(0)
    }
    /// Canonical owner pins use source ordinals, not numeric word homes.
    pub(in crate::native_artifact) fn argument_values(&self) -> usize {
        self.regions
            .iter()
            .map(|&(p, s)| {
                self.parents[p].segments[s]
                    .frame_plan
                    .as_ref()
                    .unwrap()
                    .sources
                    .len()
            })
            .max()
            .unwrap_or(0)
    }
    /// Source preparation and independently validated image loading use this
    /// same pure derivation. No serialized candidate inventory is accepted.
    pub(in crate::native_artifact) fn derive(
        plan: &Plan,
        frames: &NativeBufferFrames,
        aggregates: &NativeScalarAggregates,
        calls: &[Option<NativePreparedLibraryCall>],
        closed: &tail::Layout,
        visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
        retirements: &super::frame_retirement::FrameRetirements,
        admissions: &super::map_admission::Layout,
    ) -> Result<Self, &'static str> {
        if calls.len() != plan.blocks().len() || visits.len() != plan.blocks().len() {
            return Err("composition factories differ from source blocks");
        }
        let facts = plan.facts();
        let reachable = (0..plan.blocks().len())
            .map(|site| facts.is_reachable(site as u32))
            .collect::<Result<Vec<_>, _>>()?;
        let cold_predecessors = closed
            .regions
            .iter()
            .filter_map(|region| region.numeric.as_ref().filter(|p| p.phase.is_some()))
            .flat_map(|proof| proof.exits.iter().map(|exit| exit.from))
            .collect::<BTreeSet<_>>();
        let mut parents = Vec::new();
        for (iteration, control) in plan.iterations().iter().enumerate() {
            let Loop::Tail(parent) = control else {
                continue;
            };
            let closed_region = closed
                .sites
                .get(parent.header() as usize)
                .copied()
                .flatten();
            // Preserve independently proved ordinary parent pieces even when
            // a native phase covers this loop. A declined phase grants no
            // source credit; actual paid overlap still declines at admission.
            // Descendants keep the full ordinary source price. Their native
            // entry declines if any member belongs to a current paid traversal.
            // An interval's custom native CFG and its ordinary cold suffix
            // are distinct execution routes. Keep the maintained parent pieces
            // on that suffix. Reconstruct the exact already-paid source price;
            // physical entry, synchronization and retirement stay separately
            // funded. No child scope or previous traversal can lend credit.
            let prepaid_region = closed_region.filter(|&id| {
                let region = &closed.regions[id];
                region.iteration == iteration as u32
                    && region.numeric.as_ref().is_some_and(|proof| {
                        !proof.prepaid_sites.is_empty()
                            && proof.phase.as_ref().is_some_and(|phase| {
                                proof
                                    .prepaid_sites
                                    .iter()
                                    .try_fold(0u64, |n, &site| n.checked_add(visits[site].work))
                                    .is_some_and(|price| price <= phase.loops[0].compute)
                            })
                    })
            });
            let children = plan
                .iterations()
                .iter()
                .enumerate()
                .filter(|(_, child)| child.as_ref().parent() == Some(iteration as u32))
                .collect::<Vec<_>>();
            if children.is_empty() {
                continue;
            }
            if !(frames.fields(parent.carrier()).is_some_and(|f| {
                f.iter().all(|f| {
                    !matches!(f.lane(), Lane::Tagged { .. })
                        && (!matches!(f.lane(), Lane::Composite { .. })
                            || super::readonly_numeric::scalar_children(frames, *f))
                })
            }) || aggregates.slot(parent.carrier()).is_some())
            {
                continue;
            }
            let mut child_boundaries = Vec::with_capacity(children.len());
            for &(child_id, child) in &children {
                let control = child.as_ref();
                let Term::Scalar(mech_engine::native_artifact::NativeMatchTerminator::Jump(
                    child_successor,
                )) = plan.blocks()[control.exit() as usize].terminator()
                else {
                    break;
                };
                let proof = closed
                    .regions
                    .iter()
                    .find(|r| r.iteration as usize == child_id)
                    .and_then(|r| r.numeric.as_ref());
                let returned_arrays = if let Some(proof) = proof
                    && let ResultRecipe::Frame { arrays, .. } = &proof.result
                {
                    // Parent-piece forwarding names child carrier homes. An
                    // immutable capture has no such home; the child's own
                    // numeric proof remains independently usable.
                    if arrays.iter().any(|&(_, array)| {
                        proof
                            .arrays
                            .get(array)
                            .is_some_and(|a| a.physical().is_none())
                    }) {
                        break;
                    }
                    arrays
                        .iter()
                        .map(|&(field, array)| {
                            proof
                                .arrays
                                .get(array)
                                .map(|a| (field, a.root))
                                .ok_or("composition child result array is outside its proof")
                        })
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    Vec::new()
                };
                let scalar_child = matches!(child, Loop::Tail(tail)
                    if super::readonly_numeric::scalar_aggregate(plan, aggregates, tail.result())
                        && scalar_child_tree(plan, frames, aggregates, child_id));
                child_boundaries.push(Child {
                    child: child_id as u32,
                    child_initialize: control.initialize(),
                    initialize_progress: if matches!(child, Loop::Collection(_)) {
                        2
                    } else {
                        1
                    },
                    child_exit: control.exit(),
                    child_successor,
                    scalar_child,
                    returned_arrays,
                });
            }
            if child_boundaries.len() != children.len() {
                continue;
            }
            // Keep the original ordinary completion when a native cut could
            // land after a child's exit but before its successor consumes it.
            if child_boundaries
                .iter()
                .any(|child| cold_predecessors.contains(&(child.child_exit as usize)))
            {
                continue;
            }
            // Every lifecycle transition and managed scalar handoff stays an
            // ordinary boundary. A native piece must finish before the child
            // initializer, and may only reopen after its original exit/copies.
            let sites = plan
                .blocks()
                .iter()
                .enumerate()
                .filter_map(|(site, block)| {
                    (reachable[site]
                        && block.iteration() == Some(iteration as u32)
                        && site != parent.header() as usize
                        && site != parent.latch() as usize
                        && site != parent.finish() as usize
                        && child_boundaries.iter().all(|child| {
                            site != child.child_initialize as usize
                                && site != child.child_exit as usize
                        })
                        && plan.activation_boundary() != Some(site as u32)
                        && matches!(block.terminator(), Term::Scalar(_))
                        && eligible(plan, frames, aggregates, calls, site))
                    .then_some(site as u32)
                })
                .collect::<BTreeSet<_>>();
            let mut incoming = BTreeMap::new();
            for &site in &sites {
                incoming.insert(
                    site,
                    facts
                        .predecessors(site)?
                        .iter()
                        .filter(|p| sites.contains(p))
                        .count(),
                );
            }
            let mut ready = incoming
                .iter()
                .filter(|(_, count)| **count == 0)
                .map(|(&site, _)| site)
                .collect::<VecDeque<_>>();
            let mut order = Vec::new();
            while let Some(site) = ready.pop_front() {
                order.push(site);
                for next in facts
                    .successors(site)?
                    .into_iter()
                    .flatten()
                    .filter(|s| sites.contains(s))
                {
                    let count = incoming
                        .get_mut(&next)
                        .ok_or("composition edge lacks a node")?;
                    *count -= 1;
                    if *count == 0 {
                        ready.push_back(next)
                    }
                }
            }
            // A second backedge cannot be hidden inside a native segment.
            if order.len() != sites.len() {
                continue;
            }
            let mut owner = BTreeMap::new();
            for &site in &order {
                let predecessors = facts.predecessors(site)?;
                let mut roots = predecessors
                    .iter()
                    .filter_map(|p| owner.get(p).copied())
                    .collect::<BTreeSet<_>>();
                if predecessors.iter().any(|p| !sites.contains(p)) {
                    roots.insert(site);
                }
                // A join of independently entered pieces becomes a new entry,
                // rather than letting either predecessor own a shared suffix.
                let root = if roots.len() == 1 {
                    *roots.first().unwrap()
                } else {
                    site
                };
                owner.insert(site, root);
            }
            let mut segments = BTreeMap::<u32, Segment>::new();
            for &site in &order {
                let root = owner[&site];
                let segment = segments.entry(root).or_insert_with(|| Segment {
                    entry: root,
                    sites: Vec::new(),
                    exits: Vec::new(),
                    frame_plan: None,
                    numeric: None,
                    prepaid_region,
                    rejection: None,
                });
                segment.sites.push(site);
                for next in facts.successors(site)?.into_iter().flatten() {
                    if owner.get(&next) != Some(&root) {
                        segment.exits.push((site, next));
                    }
                }
            }
            let mut segments = segments.into_values().collect::<Vec<_>>();
            for segment in &mut segments {
                if let Some(id) = segment.prepaid_region
                    && !segment.sites.iter().all(|&site| {
                        closed.regions[id]
                            .numeric
                            .as_ref()
                            .unwrap()
                            .prepaid_sites
                            .binary_search(&(site as usize))
                            .is_ok()
                    })
                {
                    segment.prepaid_region = None;
                }
                match frames::Layout::derive(plan, &facts, frames, segment) {
                    Ok(mut proof) => {
                        match numeric::Layout::derive(
                            plan,
                            &facts,
                            frames,
                            aggregates,
                            calls,
                            visits,
                            retirements,
                            segment,
                            &mut proof,
                        ) {
                            Ok(numeric) => segment.numeric = Some(numeric),
                            Err(reason) => segment.rejection = Some(reason.into()),
                        }
                        segment.frame_plan = Some(proof);
                    }
                    Err(reason) => segment.rejection = Some(reason.into()),
                }
            }
            // Validate the complete graph partition independently of construction.
            for segment in &segments {
                let members = segment.sites.iter().copied().collect::<BTreeSet<_>>();
                for &site in &segment.sites {
                    if site != segment.entry
                        && facts
                            .predecessors(site)?
                            .iter()
                            .any(|p| !members.contains(p))
                    {
                        return Err("composition segment has a second entry");
                    }
                }
            }
            parents.push(Parent {
                iteration: iteration as u32,
                suspended_child: None,
                children: child_boundaries,
                segments,
            });
        }
        let mut regions = Vec::new();
        let mut words = 0usize;
        for (parent, proof) in parents.iter().enumerate() {
            for (segment, part) in proof.segments.iter().enumerate() {
                let Some(numeric) = &part.numeric else {
                    continue;
                };
                // Two new boundary calls must replace more existing calls.
                // This is a profitability choice, never a source-size allowance.
                let saved = part.sites.len().saturating_sub(1) + numeric.actions.len();
                // An ordinary scalar prefix has already paid its own source
                // visits. The larger piece may follow it only when it owns the
                // complete prefix. Owner groups and partial/interior overlaps
                // retain their original boundaries and fallback protocol.
                let conflict = admissions.group(part.entry as usize).is_some_and(|group| {
                    group.owner.is_some()
                        || group.start() != part.entry
                        || !group.sites.iter().all(|site| part.sites.contains(site))
                });
                if saved <= 2 || conflict {
                    continue;
                }
                words = words.max(
                    numeric
                        .words
                        .checked_add(1)
                        .ok_or("parent native scratch overflows")?,
                );
                regions.push((parent, segment));
            }
        }
        let mut boundaries = Vec::new();
        let mut tracked_children = vec![false; plan.iterations().len()];
        let entries = if regions.is_empty() {
            Box::default()
        } else {
            let mut entries = vec![None; plan.blocks().len()];
            for (id, &(parent, segment)) in regions.iter().enumerate() {
                entries[parents[parent].segments[segment].entry as usize] = NonZeroUsize::new(
                    id.checked_mul(4)
                        .and_then(|n| n.checked_add(1))
                        .ok_or("parent region index overflows")?,
                );
            }
            for parent in regions.iter().map(|&(p, _)| p).collect::<BTreeSet<_>>() {
                for (child_id, child) in parents[parent].children.iter().enumerate() {
                    let boundary = boundaries.len();
                    boundaries.push((parent, child_id));
                    if core::mem::replace(&mut tracked_children[child.child as usize], true) {
                        return Err("composition child has two parent boundaries");
                    }
                    for (site, tag) in [(child.child_initialize, 1), (child.child_exit, 2)] {
                        let encoded = boundary
                            .checked_mul(4)
                            .and_then(|n| n.checked_add(tag + 1))
                            .ok_or("parent boundary index overflows")?;
                        if entries[site as usize]
                            .replace(NonZeroUsize::new(encoded).unwrap())
                            .is_some()
                        {
                            return Err("parent boundary overlaps another selected region");
                        }
                    }
                }
            }
            entries.into_boxed_slice()
        };
        // Determine the expected outer marker once, using immutable ancestry.
        // Child calls and piece entry must not walk the graph on every visit.
        let suspended_child = |iteration| {
            let mut current = Some(iteration);
            while let Some(id) = current {
                if tracked_children[id as usize] {
                    return Some(id);
                }
                current = plan.iterations()[id as usize].as_ref().parent();
            }
            None
        };
        for parent in &mut parents {
            parent.suspended_child = suspended_child(parent.iteration);
        }
        let native_contexts = closed
            .regions
            .iter()
            .map(|region| suspended_child(region.iteration))
            .collect();
        Ok(Self {
            parents,
            entries,
            regions,
            boundaries,
            native_contexts,
            words,
        })
    }
}

/// Retain the existing scalar-child diagnostic classification. Other children
/// also remain ordinary boundaries; a piece never keeps a loan across any child.
fn scalar_child_tree(
    plan: &Plan,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    child: usize,
) -> bool {
    let mut pending = vec![child];
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            return false;
        }
        let Some(Loop::Tail(tail)) = plan.iterations().get(id) else {
            return false;
        };
        if aggregates.slot(tail.carrier()).is_none() || frames.fields(tail.carrier()).is_some() {
            return false;
        }
        pending.extend(
            plan.iterations()
                .iter()
                .enumerate()
                .filter(|(_, control)| control.as_ref().parent() == Some(id as u32))
                .map(|(id, _)| id),
        );
    }
    true
}

fn eligible(
    plan: &Plan,
    frames: &NativeBufferFrames,
    aggregates: &NativeScalarAggregates,
    calls: &[Option<NativePreparedLibraryCall>],
    site: usize,
) -> bool {
    let action = plan.blocks()[site].action();
    if aggregates.action(site).is_some() {
        return true;
    }
    if let Some(frame) = frames.action(site) {
        let native_frame = |value| {
            frames.fields(value).is_some_and(|fields| {
                fields.iter().all(|f| {
                    !matches!(f.lane(), Lane::Tagged { .. })
                        && (!matches!(f.lane(), Lane::Composite { .. })
                            || super::readonly_numeric::scalar_children(frames, *f))
                })
            })
        };
        return match frame.operation() {
            Frame::Pack { output } => {
                native_frame(*output)
                    && calls[site]
                        .as_ref()
                        .is_some_and(|c| c.supports_invariant_buffer_frame())
            }
            Frame::EnumPack { .. } | Frame::Observe { .. } => false,
            Frame::Copy { source, output } => native_frame(*source) && native_frame(*output),
            Frame::Project {
                source,
                ordinal,
                output,
            } => {
                native_frame(*source)
                    && (plan.values()[output.get() as usize].scalar_kind().is_some()
                        || (native_frame(*output)
                            && frames
                                .fields(*source)
                                .and_then(|fields| fields.get(*ordinal))
                                .is_some_and(|field| {
                                    matches!(field.lane(), Lane::Composite { .. })
                                        && super::readonly_numeric::scalar_children(frames, *field)
                                }))
                        || ((matches!(action, Action::Structural { .. })
                            || matches!(action, Action::LibraryCall { template, .. }
                                if template.primitive() == Primitive::RecordFieldAccess
                                    && calls[site].as_ref().is_some_and(|call| call.template() == template)))
                            && frames
                                .fields(*source)
                                .and_then(|fields| fields.get(*ordinal))
                                .is_some_and(|field| matches!(field.lane(), Lane::Managed { .. }))))
            }
        };
    }
    match action {
        Action::Scalar(Scalar::None | Scalar::Primitive { .. }) => true,
        Action::Scalar(Scalar::Copy { output, .. }) => {
            plan.values()[output.get() as usize].scalar_kind().is_some()
        }
        Action::LibraryCall { .. } => calls[site].as_ref().is_some_and(|c| {
            c.supports_unsigned_matrix_length() || c.supports_unsigned_matrix_element()
        }),
        _ => false,
    }
}

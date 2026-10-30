//! Checked parent/child boundaries. This inventory does not authorize execution:
//! owner forwarding, finite settlement and continuation emission remain required.
use super::{readonly_numeric::ResultRecipe, tail};
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame, NativeBufferFrames,
    NativeControlLoop as Loop, NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativePreparedLibraryCall, NativeProgramArtifactPlan as Plan,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroUsize;
pub(in crate::native_artifact) mod frames;
pub(in crate::native_artifact) mod numeric;

#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Segment {
    pub entry: u32,
    pub sites: Vec<u32>,
    /// Exact last executed source site and its next, still ordinary frontier.
    pub exits: Vec<(u32, u32)>,
    pub frame_plan: Option<frames::Layout>,
    pub numeric: Option<numeric::Layout>,
    pub rejection: Option<&'static str>,
}
#[derive(Debug, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Parent {
    pub iteration: u32,
    pub child: u32,
    pub child_initialize: u32,
    pub child_exit: u32,
    pub child_successor: u32,
    /// Returned tuple/record field -> CURRENT child carrier array field.
    pub returned_arrays: Vec<(usize, usize)>,
    pub segments: Vec<Segment>,
}
#[derive(Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub parents: Vec<Parent>,
    #[serde(skip)]
    entries: Box<[Option<NonZeroUsize>]>,
    #[serde(skip)]
    regions: Vec<(usize, usize)>,
    #[serde(skip)]
    pub words: usize,
}
impl Layout {
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
    /// Source preparation and independently validated image loading use this
    /// same pure derivation. No serialized candidate inventory is accepted.
    pub(in crate::native_artifact) fn derive(
        plan: &Plan,
        frames: &NativeBufferFrames,
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
        let mut parents = Vec::new();
        for (iteration, control) in plan.iterations().iter().enumerate() {
            let Loop::Tail(parent) = control else {
                continue;
            };
            let children = plan
                .iterations()
                .iter()
                .enumerate()
                .filter(|(_, child)| child.as_ref().parent() == Some(iteration as u32))
                .collect::<Vec<_>>();
            let [(child_id, Loop::Tail(child))] = children.as_slice() else {
                continue;
            };
            if plan
                .iterations()
                .iter()
                .any(|c| c.as_ref().parent() == Some(*child_id as u32))
                || !frames.fields(parent.carrier()).is_some_and(|f| {
                    f.iter()
                        .all(|f| !matches!(f.lane(), Lane::Composite { .. }))
                })
            {
                continue;
            }
            let Some(proof) = closed
                .regions
                .iter()
                .find(|r| r.iteration as usize == *child_id)
                .and_then(|r| r.numeric.as_ref())
            else {
                continue;
            };
            if proof.arrays.iter().any(|a| a.appends) {
                continue;
            }
            let ResultRecipe::Frame { arrays, .. } = &proof.result else {
                continue;
            };
            let Term::Scalar(mech_engine::native_artifact::NativeMatchTerminator::Jump(
                child_successor,
            )) = plan.blocks()[child.exit() as usize].terminator()
            else {
                continue;
            };
            let returned_arrays = arrays
                .iter()
                .map(|&(field, array)| {
                    proof
                        .arrays
                        .get(array)
                        .map(|a| (field, a.field))
                        .ok_or("composition child result array is outside its proof")
                })
                .collect::<Result<Vec<_>, _>>()?;

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
                        && site != child.initialize() as usize
                        && site != child.exit() as usize
                        && plan.activation_boundary() != Some(site as u32)
                        && matches!(block.terminator(), Term::Scalar(_))
                        && eligible(plan, frames, calls, site))
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
                match frames::Layout::derive(plan, &facts, frames, segment) {
                    Ok(mut proof) => {
                        match numeric::Layout::derive(
                            plan,
                            &facts,
                            frames,
                            calls,
                            visits,
                            retirements,
                            segment,
                            &mut proof,
                        ) {
                            Ok(numeric) => segment.numeric = Some(numeric),
                            Err(reason) => segment.rejection = Some(reason),
                        }
                        segment.frame_plan = Some(proof);
                    }
                    Err(reason) => segment.rejection = Some(reason),
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
                child: *child_id as u32,
                child_initialize: child.initialize(),
                child_exit: child.exit(),
                child_successor,
                returned_arrays,
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
                if saved <= 2 || admissions.group(part.entry as usize).is_some() {
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
                for (site, tag) in [
                    (parents[parent].child_initialize, 1),
                    (parents[parent].child_exit, 2),
                ] {
                    let encoded = parent
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
            entries.into_boxed_slice()
        };
        Ok(Self {
            parents,
            entries,
            regions,
            words,
        })
    }
}

fn eligible(
    plan: &Plan,
    frames: &NativeBufferFrames,
    calls: &[Option<NativePreparedLibraryCall>],
    site: usize,
) -> bool {
    let action = plan.blocks()[site].action();
    if let Some(frame) = frames.action(site) {
        let flat = |value| {
            frames.fields(value).is_some_and(|fields| {
                fields
                    .iter()
                    .all(|f| !matches!(f.lane(), Lane::Composite { .. }))
            })
        };
        return match frame.operation() {
            Frame::Pack { output } => {
                flat(*output)
                    && calls[site]
                        .as_ref()
                        .is_some_and(|c| c.supports_invariant_buffer_frame())
            }
            Frame::Copy { source, output } => flat(*source) && flat(*output),
            Frame::Project { source, output, .. } => {
                flat(*source) && plan.values()[output.get() as usize].scalar_kind().is_some()
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

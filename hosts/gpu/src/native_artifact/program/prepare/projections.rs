//! Same-source array field transport. This is an immutable storage recipe;
//! every reached field still performs its original admission and retirement.
use super::{composition, tail};
use crate::native_artifact::iteration_abi::{NativeIterationVisit, Progress};
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Frame, NativeBufferFrames,
    NativeIterationAction as Action, NativeIterationTerminator as Term, NativeMapOwnerWords,
    NativeMatchTerminator as ScalarTerm, NativeMatchValue, NativeProgramArtifactPlan as Plan,
    NativeStructuralObservation,
};
use std::collections::BTreeSet;

#[derive(Debug, serde::Serialize)]
pub(in crate::native_artifact) struct Batch {
    pub source: NativeMatchValue,
    pub sites: Vec<u32>,
    pub successor: u32,
}
#[derive(Debug, Default, serde::Serialize)]
pub(in crate::native_artifact) struct Layout {
    pub batches: Box<[Batch]>,
}
impl Layout {
    pub(in crate::native_artifact) fn entry(&self, site: usize) -> Option<&Batch> {
        self.batches
            .binary_search_by_key(&site, |batch| batch.sites[0] as usize)
            .ok()
            .map(|index| &self.batches[index])
    }

    pub(in crate::native_artifact) fn derive(
        plan: &Plan,
        frames: &NativeBufferFrames,
        visits: &[NativeIterationVisit],
        closed: &tail::Layout,
        pieces: &composition::Layout,
        owners: Option<&NativeMapOwnerWords>,
        parallel: &super::parallel::Layout,
    ) -> Result<Self, &'static str> {
        if visits.len() != plan.blocks().len() {
            return Err("projection visit inventory differs from source");
        }
        let occupied = pieces
            .parents
            .iter()
            .flat_map(|p| &p.segments)
            .filter(|s| pieces.entry(s.entry as usize).is_some())
            .flat_map(|s| s.sites.iter().copied())
            .collect::<BTreeSet<_>>();
        let candidate = |site: usize| {
            let block = plan.blocks().get(site)?;
            let visit = visits[site];
            if visit.progress != Progress::Normal
                || visit.complete
                || visit.integrity
                || visit.reject
                || plan.activation_boundary() == Some(site as u32)
                || occupied.contains(&(site as u32))
                || parallel
                    .regions
                    .iter()
                    .any(|region| region.contains(site as u32))
                || closed.sites.get(site).copied().flatten().is_some()
                || owners.is_some_and(|owners| {
                    owners
                        .regions()
                        .iter()
                        .any(|owner| block.iteration() == Some(owner.iteration()))
                })
            {
                return None;
            }
            let Action::Structural {
                observation: NativeStructuralObservation::CanonicalValue,
                ..
            } = block.action()
            else {
                return None;
            };
            let Frame::Project {
                source,
                ordinal,
                output,
            } = frames.action(site)?.operation()
            else {
                return None;
            };
            let field = *frames.fields(*source)?.get(*ordinal)?;
            if !matches!(field.lane(), Lane::Managed { .. }) {
                return None;
            }
            let ty = &plan.values()[output.get() as usize];
            if ty.schema() != field.schema()
                || !matches!(ty.definition().body(),
                SchemaBody::Matrix { element, dimensions } if dimensions.len() == 2
                && matches!(element.as_ref(), SchemaBody::UnsignedInteger(
                    IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64)))
            {
                return None;
            }
            let Term::Scalar(ScalarTerm::Jump(next)) = block.terminator() else {
                return None;
            };
            Some((*source, block.iteration(), next))
        };
        let facts = plan.facts();
        let mut covered = BTreeSet::new();
        let mut batches = Vec::new();
        for site in 0..plan.blocks().len() {
            if covered.contains(&(site as u32)) {
                continue;
            }
            let Some((source, iteration, mut next)) = candidate(site) else {
                continue;
            };
            let mut sites = vec![site as u32];
            loop {
                let previous = *sites.last().unwrap();
                if next <= previous
                    || covered.contains(&next)
                    || facts.predecessors(next)? != [previous]
                {
                    break;
                }
                let Some((current, scope, successor)) = candidate(next as usize) else {
                    break;
                };
                if current != source || scope != iteration {
                    break;
                }
                sites.push(next);
                next = successor;
            }
            if sites.len() >= 2 {
                covered.extend(sites.iter().copied());
                batches.push(Batch {
                    source,
                    sites,
                    successor: next,
                });
            }
        }
        Ok(Self {
            batches: batches.into_boxed_slice(),
        })
    }
}

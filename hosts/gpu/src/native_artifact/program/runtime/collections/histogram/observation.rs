//! Test-only snapshots at the real preflight, after the real selected search.
//! Header observations never compare keys, scan/copy payloads, or set policies.
use crate::native_artifact::program::runtime::collections::histogram::*;
use crate::native_artifact::program::runtime::invocation::CollectionRead;
use mech_engine::native_artifact::NativeIterationAction;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum HistogramStage {
    Yield,
    Growth,
    Finish,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct HistogramVisit {
    pub collection: u32,
    pub stage: HistogramStage,
    pub accepted_before: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct HistogramDemand {
    pub stage: HistogramStage,
    pub visit: HistogramVisit,
    pub origin: mech_engine::native_artifact::NativeMatchOrigin,
    pub temporary_required: u64,
    pub reservation_bytes: u64,
    pub compute_required: u64,
    pub compute_debit: u64,
    pub cloned_required: u64,
    pub comparison_required: u64,
    pub output_required: u64,
    pub elements_required: u64,
    pub nodes_required: u64,
    pub distinct_before: u64,
    pub payload_before: u64,
    pub capacity_before: u64,
    pub current_yield_bytes: u64,
    pub untracked_yield_bytes: u64,
    pub old_backing_bytes: u64,
    pub live_bytes: u64,
}
thread_local! {
    static LAST:std::cell::Cell<Option<HistogramDemand>>=const{std::cell::Cell::new(None)};
}
impl State {
    pub(in crate::native_artifact) const fn test_histogram_entry_bytes() -> u64 {
        size_of::<Entry>() as u64
    }
    pub(in crate::native_artifact) fn test_histogram_clear_observation() {
        LAST.with(|slot| slot.set(None));
    }
    pub(in crate::native_artifact) fn test_histogram_take_observation() -> Option<HistogramDemand> {
        LAST.with(|slot| slot.take())
    }
    pub(in crate::native_artifact) fn test_histogram_visit(
        &self,
        ctx: &impl CollectionRead,
        site: usize,
    ) -> Option<HistogramVisit> {
        let (collection, stage) = match ctx
            .descriptor()
            .metadata()
            .plan
            .blocks()
            .get(site)?
            .action()
        {
            NativeIterationAction::CollectionPush { collection, .. } => {
                (*collection, HistogramStage::Growth)
            }
            NativeIterationAction::HistogramFinish { collection } => {
                (*collection, HistogramStage::Finish)
            }
            _ => return None,
        };
        let histogram = self
            .builders
            .get(collection as usize)?
            .as_ref()?
            .histogram
            .as_ref()?;
        Some(HistogramVisit {
            collection,
            stage,
            accepted_before: histogram.accepted,
        })
    }
    pub(in crate::native_artifact) fn test_histogram_storage(
        &self,
        id: u32,
    ) -> Option<(u64, u64, u64, u64)> {
        let histogram = self
            .builders
            .get(id as usize)?
            .as_ref()?
            .histogram
            .as_ref()?;
        Some((
            histogram.accepted,
            histogram.entries.len() as u64,
            histogram.key_payload,
            histogram.entries.capacity() as u64,
        ))
    }
    pub(in crate::native_artifact) fn test_histogram_record(
        &self,
        ctx: &impl CollectionRead,
        id: u32,
        stage: HistogramStage,
        reservation_bytes: u64,
        temporary_required: u64,
        compute_debit: u64,
        clones: u64,
        elements_required: u64,
        payload: u64,
        current_yield_bytes: u64,
        untracked_yield_bytes: u64,
    ) -> Result<(), Error> {
        let definition = sink(ctx.descriptor(), id)?;
        let builder = self.builders[id as usize]
            .as_ref()
            .ok_or_else(|| invalid("observed Histogram builder"))?;
        let histogram = builder
            .histogram
            .as_ref()
            .ok_or_else(|| invalid("observed Histogram state"))?;
        let selected = ctx.descriptor().metadata().calls[definition.consumer() as usize]
            .as_ref()
            .ok_or_else(|| invalid("observed Histogram selection"))?;
        let footprint = selected
            .selected_histogram_footprint(elements_required, payload)
            .map_err(|error| Error::Library {
                origin: definition.template().origin(),
                error,
            })?;
        let demand = HistogramDemand {
            stage,
            visit: HistogramVisit {
                collection: id,
                stage: if stage == HistogramStage::Yield {
                    HistogramStage::Growth
                } else {
                    stage
                },
                accepted_before: histogram.accepted,
            },
            origin: definition.template().origin(),
            temporary_required,
            reservation_bytes,
            compute_required: add(builder.compute, compute_debit)?,
            compute_debit,
            cloned_required: add(builder.cloned_bytes, clones)?,
            comparison_required: add(
                histogram.comparisons,
                if stage == HistogramStage::Finish {
                    selected
                        .selected_histogram_finalization_work(elements_required, payload)
                        .map_err(|error| Error::Library {
                            origin: definition.template().origin(),
                            error,
                        })?
                } else {
                    0
                },
            )?,
            output_required: footprint.retained_bytes,
            elements_required,
            nodes_required: add(
                add(mul(footprint.node_count, 3)?, 5)?,
                self.retained_nodes()?,
            )?,
            distinct_before: histogram.entries.len() as u64,
            payload_before: histogram.key_payload,
            capacity_before: histogram.entries.capacity() as u64,
            current_yield_bytes,
            untracked_yield_bytes,
            old_backing_bytes: add(
                histogram.entry_reservation.capacity_bytes(),
                histogram.key_reservation.capacity_bytes(),
            )?,
            live_bytes: self.live_bytes,
        };
        LAST.with(|slot| slot.set(Some(demand)));
        Ok(())
    }
}

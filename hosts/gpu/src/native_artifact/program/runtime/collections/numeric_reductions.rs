//! A completed native scalar fold, never an escaped intermediate Matrix.
use super::*;
use crate::native_artifact::program::prepare::numeric_maps::Region;

pub(super) struct Pending {
    pub header: usize,
    pub count: u64,
    pub word: u64,
    pub epoch: u64,
    pub visited: usize,
    pub work: mech_engine::native_artifact::NativeLibraryWorkBudget,
    pub _metadata: ManagedMemoryReservation,
    pub _nodes: ManagedNodeReservation,
}
#[derive(Default)]
pub(super) struct Bound {
    pub temporary: u64,
    pub nodes: u64,
    pub output: u64,
    pub elements: u64,
    pub comparison: u64,
    pub setup: u64,
    pub replay: u64,
    pub commit: u64,
    pub cloned: u64,
}
pub(super) fn bound(
    descriptor: &NativeProgramDescriptor,
    region: &Region,
    count: u64,
) -> Option<Bound> {
    let Some(reduction) = &region.reduction else {
        return Some(Bound::default());
    };
    let metadata = descriptor.metadata();
    // Source grouping has a distinct paid frontier. A future extension needs
    // its own composition proof; it cannot inherit ordinary visit permission.
    if reduction
        .finish_path
        .iter()
        .chain(&reduction.continuation)
        .any(|&site| metadata.map_admission.group(site as usize).is_some())
    {
        return None;
    }
    let sum = metadata.calls.get(reduction.sum as usize)?.as_ref()?;
    let select = metadata.calls.get(reduction.select as usize)?.as_ref()?;
    if !sum.supports_f64_row_sum() || !select.supports_f64_matrix_element() {
        return None;
    }
    let bytes = sum
        .binding_metadata_bytes()
        .checked_add(select.binding_metadata_bytes())?;
    let work = sum.binding_work().checked_add(select.binding_work())?;
    let cells = count.checked_add(4)?.checked_mul(
        (size_of::<ValueDataDraft>() as u64)
            .checked_mul(16)?
            .checked_add(512)?,
    )?;
    let temporary = bytes
        .checked_mul(128)?
        .checked_add(metadata.observation_schema_bytes.checked_mul(32)?)?
        .checked_add(cells)?
        .checked_add(65536)?;
    let output = count
        .checked_add(4)?
        .checked_mul(size_of::<ValueDataDraft>() as u64)?
        .checked_add(bytes)?
        .checked_add(4096)?;
    let commit = work
        .checked_mul(16)?
        .checked_add(temporary.checked_mul(16)?)?
        .checked_add(8192)?;
    let replay = work
        .checked_mul(1024)?
        .checked_add(temporary.checked_mul(128)?)?
        .checked_add(65536)?;
    Some(Bound {
        temporary,
        nodes: count.checked_mul(4)?.checked_add(bytes)?.checked_add(256)?,
        output,
        // Even an empty producer has a 1x1 sum and scalar selection.
        elements: count.max(1),
        comparison: output,
        setup: bytes.checked_mul(128)?.checked_add(4096)?,
        replay,
        commit,
        cloned: cells.checked_add(bytes)?,
    })
}
impl State {
    pub(in crate::native_artifact::program::runtime) fn has_numeric_reduction(&self) -> bool {
        self.native_reduction.is_some()
    }
    pub(in crate::native_artifact::program::runtime) fn advance_numeric_reduction_visit(
        &mut self,
        descriptor: &NativeProgramDescriptor,
        site: usize,
        visits: u64,
    ) -> Result<(), Error> {
        let Some(pending) = self.native_reduction.as_mut() else {
            return Ok(());
        };
        let region = descriptor
            .metadata()
            .numeric_maps
            .at_header(pending.header)
            .ok_or_else(|| invalid("parked numeric reduction lost source"))?;
        let proof = region
            .reduction
            .as_ref()
            .ok_or_else(|| invalid("parked numeric reduction lost proof"))?;
        if proof.finish_path.get(pending.visited).copied() != u32::try_from(site).ok()
            || pending
                .epoch
                .checked_add(pending.visited as u64)
                .and_then(|n| n.checked_add(1))
                != Some(visits)
        {
            return Err(invalid(
                "source visit outside exact numeric reduction finish path",
            ));
        }
        pending.visited += 1;
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn finish_numeric_reduction(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
    ) -> Result<Option<(u64, usize)>, Error> {
        let Some(pending) = self.native_reduction.as_ref() else {
            return Ok(None);
        };
        let descriptor = ctx.descriptor();
        let metadata = descriptor.metadata();
        let region = metadata
            .numeric_maps
            .at_header(pending.header)
            .ok_or_else(|| invalid("numeric reduction completion lost header"))?;
        let reduction = region
            .reduction
            .as_ref()
            .ok_or_else(|| invalid("numeric reduction completion lost proof"))?;
        let definition = &metadata.plan.collections()[region.collection as usize];
        if site != reduction.finish as usize
            || ctx.ledger().previous != Some(site as u32)
            || pending.visited != reduction.finish_path.len()
            || pending.epoch.checked_add(pending.visited as u64) != Some(ctx.ledger().visits)
            || ctx.ledger().current != definition.parent()
            || self.current != Some(region.collection)
            || !pending.work.shares_account(ctx.library())
        {
            return Err(invalid(
                "numeric reduction completion lost its exact source/account",
            ));
        }
        let count = pending.count;
        let word = pending.word;
        let header = pending.header;
        let builder = self.builder(region.collection)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
        {
            return Err(invalid("elided reduction acquired an intermediate payload"));
        }
        // Entry validated the exact selected shapes and immutable count.
        // No source or descriptor mutation can occur along this finish path.
        let bound = bound(descriptor, region, count)
            .ok_or_else(|| invalid("numeric reduction admission facts changed"))?;
        self.account(ctx, region.collection, bound.cloned, bound.commit)?;
        ctx.library().charge(0, bound.comparison).map_err(|error| {
            crate::native_artifact::NativeExecutionError::LibraryWork {
                phase: Phase::Completion,
                error,
            }
        })?;
        let builder = self.builders[region.collection as usize].take().unwrap();
        self.live_bytes = self
            .live_bytes
            .checked_sub(builder.reservation.capacity_bytes())
            .ok_or_else(|| invalid("numeric reduction workspace accounting"))?;
        self.current = definition.parent_collection();
        drop(self.native_reduction.take());
        Ok(Some((word, header)))
    }
}

pub(super) fn validate(
    ctx: &mut CollectionExecution<'_, '_, '_, '_>,
    region: &Region,
    count: u64,
    bound: &Bound,
) -> Result<bool, Error> {
    let Some(reduction) = &region.reduction else {
        return Ok(true);
    };
    let _scratch = ctx
        .library()
        .memory_budget()
        .reserve_capacity(bound.temporary)?;
    let metadata = ctx.descriptor().metadata();
    let sum = metadata.calls[reduction.sum as usize]
        .as_ref()
        .ok_or_else(|| invalid("numeric reduction sum factory missing"))?;
    let select = metadata.calls[reduction.select as usize]
        .as_ref()
        .ok_or_else(|| invalid("numeric reduction selector factory missing"))?;
    if sum.template()
        != metadata
            .plan
            .prepared_library_template(reduction.sum as usize)
            .map_err(invalid)?
            .ok_or_else(|| invalid("numeric reduction sum template missing"))?
        || select.template()
            != metadata
                .plan
                .prepared_library_template(reduction.select as usize)
                .map_err(invalid)?
                .ok_or_else(|| invalid("numeric reduction selector template missing"))?
    {
        return Err(invalid("numeric reduction factory/template differs"));
    }
    let producer = &metadata.plan.values()[region.output as usize];
    for (schema, extents) in [
        (producer.definition(), [1, count]),
        (&sum.template().inputs()[0], [1, count]),
        (sum.template().output(), [1, 1]),
        (&select.template().inputs()[0], [1, 1]),
    ] {
        if mech_core::shape_for_resolved_extents(schema, &extents).is_err() {
            return Ok(false);
        }
    }
    Ok(true)
}

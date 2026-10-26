//! Original producer shape and original selected consumer publication sites.
//! The full map exists only after every original source candidate has run.
use super::*;
use mech_core::snapshot::{
    MapEntryDraft, SnapshotCanonicalizationBudget, SnapshotValidationContext, ValueDataDraft,
    ValueDraft,
};
#[path = "program_histogram_finish_allocation.rs"]
mod allocation;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_histogram_finish_allocation_tests.rs"]
mod allocation_tests;
impl State {
    pub(in crate::native_artifact) fn park_histogram(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let definition = &ctx.control.descriptor.plan.collections()[id as usize];
        let metadata = ctx.control.descriptor.scalar_bytes;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(self.live_bytes, metadata)?,
            ctx.limits.max_temporary_bytes,
        )?;
        self.histogram_account(ctx, id, 0, add(mul(metadata, 8)?, 64)?, 0)?;
        let _shape = ctx.library.memory_budget().reserve_capacity(metadata)?;
        let histogram = self
            .builder(id)?
            .histogram
            .as_mut()
            .ok_or_else(|| invalid("Histogram producer finish before begin"))?;
        if histogram.parked {
            return Err(invalid("duplicate Histogram producer finish"));
        }
        mech_core::shape_for_resolved_extents(
            ctx.control.descriptor.plan.values()[definition.output().get() as usize].definition(),
            &[1, histogram.accepted],
        )
        .map_err(|_| invalid("collected output shape"))?;
        histogram.parked = true;
        self.current = definition.parent_collection();
        Ok(())
    }
    pub(in crate::native_artifact) fn finish_histogram(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let definition = sink(ctx.control.descriptor, id)?;
        let origin = definition.template().origin();
        let selected = ctx.control.descriptor.calls[definition.consumer() as usize]
            .as_ref()
            .ok_or_else(|| invalid("Histogram finish lacks selected factory"))?;
        let parent = ctx.control.descriptor.plan.collections()[id as usize].parent_collection();
        if self.current != parent {
            return Err(invalid(
                "Histogram consumer outside exact enclosing builder",
            ));
        }
        let histogram = self
            .builders
            .get(id as usize)
            .and_then(Option::as_ref)
            .and_then(|b| b.histogram.as_ref())
            .filter(|h| h.parked)
            .ok_or_else(|| invalid("Histogram consumer before original producer finish"))?;
        let count = histogram.entries.len() as u64;
        let payload = histogram.key_payload;
        let backing = add(
            histogram.entry_reservation.capacity_bytes(),
            histogram.key_reservation.capacity_bytes(),
        )?;
        let footprint = selected
            .selected_histogram_footprint(count, payload)
            .map_err(|error| Error::Library { origin, error })?;
        let drafts = selected
            .selected_histogram_draft_bytes(count)
            .map_err(|error| Error::Library { origin, error })?;
        let finalization = selected
            .selected_histogram_finalization_work(count, payload)
            .map_err(|error| Error::Library { origin, error })?;
        // Actual MapEntryDraft/items/String metadata, their possible boxed
        // conversion, key copies, frozen data and independent canonical owners
        // coexist with the complete admitted accumulator. No input-size array.
        let envelope = add(
            ctx.control.descriptor.scalar_bytes,
            add(
                add(mul(drafts, 2)?, mul(payload, 4)?)?,
                add(mul(footprint.retained_bytes, 4)?, 4096)?,
            )?,
        )?;
        let compute = add(add(mul(envelope, 16)?, finalization)?, 256)?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        self.test_histogram_record(
            ctx,
            id,
            HistogramStage::Finish,
            envelope,
            add(self.live_bytes, envelope)?,
            compute,
            payload,
            count,
            payload,
            0,
            0,
        )?;
        self.histogram_output(ctx, id, count, payload)?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(self.live_bytes, envelope)?,
            ctx.limits.max_temporary_bytes,
        )?;
        self.histogram_account(ctx, id, payload, compute, finalization)?;
        let mut reservation = ctx.library.memory_budget().reserve_capacity(envelope)?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        allocation::observation::admitted(&ctx.library, envelope);
        let histogram = self.builders[id as usize]
            .as_ref()
            .unwrap()
            .histogram
            .as_ref()
            .unwrap();
        let mut entries = allocation::vector(
            histogram.entries.len(),
            allocation::HistogramAllocationSite::Entries,
            "Histogram result draft allocation",
        )?;
        for entry in &histogram.entries {
            let mut items = allocation::vector(
                2,
                allocation::HistogramAllocationSite::Items,
                "Histogram result item allocation",
            )?;
            let key = allocation::key(&entry.key)?;
            items.push(ValueDataDraft::String(key));
            items.push(ValueDataDraft::U64(entry.count));
            entries.push(MapEntryDraft {
                items: items.into_boxed_slice(),
            });
        }
        let output = &ctx.control.descriptor.plan.values()[definition.output().get() as usize];
        let canonical = SnapshotCanonicalizationBudget::new(finalization);
        let value = ValueDraft {
            schema: output.schema(),
            shape_values: Box::new([]),
            data: ValueDataDraft::Map(entries.into_boxed_slice()),
        }
        .finalize(
            &SnapshotValidationContext::with_shared_schemas(&ctx.control.descriptor.schemas)
                .with_canonicalization_budget(&canonical),
        )
        .map_err(|_| Error::Library {
            origin,
            error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                mech_core::ResidentKernelError::InvalidOutput,
            ),
        })?
        .into_memory_budget(&mut reservation)?;
        ctx.arena
            .put(definition.output().get(), &value, &mut ctx.control.work)?;
        let builder = self.builders[id as usize].take().unwrap();
        drop(builder); // Drop every actual key/vector before capacity credit.
        self.live_bytes = self
            .live_bytes
            .checked_sub(backing)
            .ok_or_else(|| invalid("Histogram final backing accounting"))?;
        Ok(())
    }
}

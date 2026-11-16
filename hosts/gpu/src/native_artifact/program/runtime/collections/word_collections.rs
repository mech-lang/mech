//! Admitted views for a proved total scalar collection. No source loop runs here.
use super::{CollectionExecution, Error, SourceBacking, State, add, charge, data, invalid, mul};
use crate::native_artifact::program::prepare::{
    retirement::RetirementKind, word_collections::Region,
};
use mech_core::{ValueData, snapshot::SequenceView};

fn fits(value: u64, limit: Option<u64>) -> bool {
    limit.is_some_and(|limit| value <= limit)
}

impl State {
    pub(in crate::native_artifact::program::runtime) fn prepare_word_collection(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &Region,
        count: u64,
    ) -> Result<Option<[u64; 3]>, Error> {
        let descriptor = ctx.descriptor();
        let id = region.collection;
        let iteration = region.iteration as usize;
        let source = self
            .sources
            .get(iteration)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("word collection has no current source"))?;
        if count == 0
            || count != source.count
            || source.index_offset != 0
            || source.element.is_some()
            || ctx.progress().get(iteration * 2..iteration * 2 + 2) != Some([count, 1].as_slice())
        {
            return Ok(None);
        }
        let SourceBacking::Value(value) = &source.backing else {
            return Ok(None);
        };
        let ValueData::Matrix(matrix) = value.data() else {
            return Ok(None);
        };
        let (address, length) = match (region.input_width, matrix.elements()) {
            (1, SequenceView::U8(v)) => (v.as_ptr() as u64, v.len()),
            (2, SequenceView::U16(v)) => (v.as_ptr() as u64, v.len()),
            (4, SequenceView::U32(v)) => (v.as_ptr() as u64, v.len()),
            (8, SequenceView::U64(v)) => (v.as_ptr() as u64, v.len()),
            _ => return Ok(None),
        };
        if length as u64 != count {
            return Err(invalid("word collection source count changed"));
        }
        let slots = descriptor
            .metadata()
            .retirements
            .slots(
                ctx.retirements(),
                region.iteration,
                RetirementKind::Collection,
            )
            .map_err(invalid)?;
        if !slots.is_empty()
            || descriptor
                .metadata()
                .frozen_maps
                .regions
                .iter()
                .any(|r| r.parent == Some(region.iteration))
        {
            return Ok(None);
        }
        let frozen_work = ctx.frozen_retirement_work()?;
        let outside_nodes = self.retained_nodes()?;
        let old_live = self.live_bytes;
        let output = &descriptor.metadata().plan.values()[descriptor.metadata().plan.collections()
            [id as usize]
            .output()
            .get() as usize];
        let node_count = data::output_nodes(output.definition().body(), count)?;
        let builder = self.builder(id)?;
        if !matches!(region.output_width, 1 | 2 | 4 | 8)
            || builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.values.packed_width() != Some(u64::from(region.output_width))
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.join.is_some()
            || builder.histogram.is_some()
            || builder.query.is_some()
        {
            return Err(invalid(
                "word collection requires its exact fresh packed builder",
            ));
        }
        // These are the ordinary collection's capacities and prices. The pure
        // total body permits one physical allocation, but retains every old
        // element/read/push/retirement/relocation debit and conservative peak.
        let geometry = (|| {
            let host_count = usize::try_from(count).ok()?;
            let capacity = host_count.max(4).checked_next_power_of_two()?;
            let width = u64::from(region.output_width);
            let bytes = (capacity as u64).checked_mul(width)?;
            let payload = count.checked_mul(width)?;
            let growth_scratch =
                (if capacity == 4 { 0 } else { capacity / 2 } as u64).checked_mul(width)?;
            let moved = ((capacity - 4) as u64).checked_mul(width)?;
            let allocation = count
                .checked_mul(128)?
                .checked_add(bytes.checked_mul(16)?)?
                .checked_add(moved.checked_mul(4)?)?;
            let element = 32u64.checked_add(64)?.checked_add(
                u64::from(region.input_width)
                    .checked_mul(16)?
                    .checked_add(128)?,
            )?;
            let push = width.checked_mul(16)?.checked_add(128)?;
            let compute = count.checked_mul(element.checked_add(push)?)?;
            let source_work = count.checked_mul(region.visit_work.checked_add(frozen_work)?)?;
            let total = compute.checked_add(source_work)?.checked_add(allocation)?;
            let clones = count.checked_mul(u64::from(region.input_width).checked_add(width)?)?;
            let envelope = descriptor
                .metadata()
                .scalar_bytes
                .checked_add(bytes.checked_mul(4)?)?
                .checked_add(4096)?;
            let finish = envelope.checked_mul(16)?.checked_add(256)?;
            let final_compute = builder.compute.checked_add(compute)?.checked_add(finish)?;
            let final_clones = builder.cloned_bytes.checked_add(clones)?;
            let peak = old_live
                .checked_add(bytes)?
                .checked_add(growth_scratch.max(envelope))?;
            let output_bytes = builder
                .values
                .output_bytes(
                    payload,
                    count,
                    capacity,
                    output.definition().dimension_parameters().len(),
                )
                .ok()?;
            Some((
                capacity,
                bytes,
                payload,
                growth_scratch,
                compute,
                total,
                clones,
                envelope,
                finish,
                final_compute,
                final_clones,
                peak,
                output_bytes,
            ))
        })();
        let Some((
            capacity,
            bytes,
            payload,
            growth_scratch,
            compute,
            total,
            clones,
            envelope,
            finish,
            final_compute,
            final_clones,
            peak,
            output_bytes,
        )) = geometry
        else {
            return Ok(None);
        };
        let limits = ctx.limits();
        if !fits(count, limits.max_output_elements)
            || !fits(final_clones, limits.max_cloned_bytes)
            || !fits(final_compute, limits.max_compute_work)
            || !fits(peak, limits.max_temporary_bytes)
            || !fits(output_bytes, limits.max_output_bytes)
            || !fits(add(node_count, outside_nodes)?, limits.max_retained_nodes)
        {
            return Ok(None);
        }
        let remaining = {
            let work = ctx.work();
            work.limit().saturating_sub(work.used())
        };
        let Some((library_remaining, _)) = ctx.library().unscoped_remaining() else {
            return Ok(None);
        };
        let Some(funded) = total.checked_add(finish) else {
            return Ok(None);
        };
        if funded > remaining || funded > library_remaining {
            return Ok(None);
        }
        let memory = ctx.library().memory_budget();
        match memory.reserve_capacity(add(bytes, growth_scratch.max(envelope))?) {
            Ok(probe) => drop(probe),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        match memory.reserve_nodes(add(node_count, 1)?) {
            Ok(probe) => drop(probe),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        self.account(ctx, id, clones, compute)?;
        charge(
            &mut ctx.work(),
            total
                .checked_sub(compute)
                .ok_or_else(|| invalid("word collection charge underflow"))?,
        )?;
        let pending = memory.reserve_nodes(1)?;
        let scratch = (growth_scratch != 0)
            .then(|| memory.reserve_capacity(growth_scratch))
            .transpose()?;
        let builder = self.builder(id)?;
        builder.reservation.resize_capacity(bytes)?;
        builder.values.reserve(capacity)?;
        let sink = builder.values.unsigned_pointer()?;
        builder.payload_bytes = payload;
        self.live_bytes = add(old_live, bytes)?;
        drop((scratch, pending));
        Ok(Some([address, sink, count]))
    }

    pub(in crate::native_artifact::program::runtime) fn finish_word_collection(
        &mut self,
        region: &Region,
        count: u64,
    ) -> Result<(), Error> {
        let count =
            usize::try_from(count).map_err(|_| invalid("word collection count exceeds host"))?;
        let capacity = count
            .max(4)
            .checked_next_power_of_two()
            .ok_or_else(|| invalid("word collection capacity overflow"))?;
        let builder = self.builder(region.collection)?;
        if builder.values.capacity() != capacity
            || builder.payload_bytes != mul(count as u64, u64::from(region.output_width))?
            || builder.reservation.capacity_bytes()
                != mul(capacity as u64, u64::from(region.output_width))?
        {
            return Err(invalid("word collection lost its admitted sink geometry"));
        }
        // SAFETY: the exact same-build total scalar kernel returned success
        // after writing every element in this unique, bounded unsigned loan.
        // The ordinary finish will validate and publish its canonical owner.
        unsafe {
            builder.values.initialize_unsigned(count)?;
        }
        Ok(())
    }
}

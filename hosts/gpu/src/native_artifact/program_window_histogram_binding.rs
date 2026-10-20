//! Admitted storage for compiler-emitted dense windows; no source loop here.
use super::*;
use mech_core::ValueData;
use mech_engine::native_artifact::NativeWindowRegion;

pub(super) struct Bins {
    values: Vec<u64>,
    width: usize,
    _reservation: ManagedMemoryReservation,
}

impl State {
    pub(in crate::native_artifact::program_runtime) fn prepare_compiled_histogram(
        &mut self,
        ctx: &mut Context<'_, '_>,
        region: &NativeWindowRegion,
        width: u64,
        length: u64,
    ) -> Result<Option<([u64; 5], u64)>, Error> {
        // A bounded representation variant of the ordinary selected operations.
        // Wider windows keep the same compiler CFG and selected Histogram sink.
        if width > 2 {
            return Ok(None);
        }
        let id = region.collection();
        super::super::charge(
            &mut ctx.control.work,
            super::super::super::amount(ctx.call_inputs.values().len(), 16, 128)?,
        )?;
        let histogram = self
            .builder(id)?
            .histogram
            .as_ref()
            .ok_or_else(|| invalid("compiled Histogram has no selected sink"))?;
        if histogram.parked
            || histogram.accepted != 0
            || !histogram.entries.is_empty()
            || histogram.compiled_bins.is_some()
            || ctx.call_inputs.values().len() < 2
            || ctx.call_inputs.values().iter().any(Option::is_some)
        {
            return Err(invalid("compiled Histogram has stale owners or sink state"));
        }
        let source = ctx
            .arena
            .copy_current(region.source().get(), &mut ctx.control.work)?;
        let ValueData::String(text) = source.data() else {
            return Err(invalid("compiled Histogram requires a genuine String root"));
        };
        // Before inspecting representation, pay its full scan, including a
        // declined Unicode/CRLF variant. CRLF is one grapheme, not two bytes.
        self.histogram_account(ctx, id, 0, add(mul(text.len() as u64, 64)?, 256)?, 0)?;
        if !text.is_ascii() || mech_engine::intrinsics::text_graphemes::has_crlf(text) {
            return Ok(None);
        }
        if text.len() as u64 != length {
            return Err(invalid("compiled Histogram length differs from its source"));
        }
        let accepted = if width > 0 && width <= length {
            length - width + 1
        } else {
            0
        };
        let cells = if accepted == 0 {
            0
        } else {
            1usize << (width as usize * 7)
        };
        let bytes = mul(cells as u64, size_of::<u64>() as u64)?;
        let source_bytes = source
            .memory_budget_retained_bytes(&ctx.library.memory_budget())
            .ok_or_else(|| invalid("compiled Histogram source is unadmitted"))?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(add(self.live_bytes, source_bytes)?, bytes)?,
            ctx.limits.max_temporary_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(self.retained_nodes()?, 2)?,
            ctx.limits.max_retained_nodes,
        )?;
        // Covers native byte loads/count updates, scratch initialization and
        // drop, and BOTH bounded materialization scans and per-key allocation.
        // Final canonical Map validation retains its existing finite accounting.
        let work = add(add(mul(accepted, 32)?, mul(cells as u64, 1024)?)?, 1024)?;
        self.histogram_account(ctx, id, 0, work, 0)?;
        let reservation = ctx.library.memory_budget().reserve_capacity(bytes)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(cells)
            .map_err(|_| invalid("compiled Histogram scratch allocation"))?;
        if values.capacity() > cells {
            return Err(invalid(
                "compiled Histogram scratch exceeds admitted layout",
            ));
        }
        values.resize(cells, 0);
        let words = [
            text.as_ptr() as u64,
            length,
            values.as_mut_ptr() as u64,
            cells as u64,
            width,
        ];
        self.builder(id)?.histogram.as_mut().unwrap().compiled_bins = Some(Bins {
            values,
            width: width as usize,
            _reservation: reservation,
        });
        self.live_bytes = add(self.live_bytes, bytes)?;
        // The actual immutable source and actual mutable scratch allocation
        // remain owned across the synchronous native call. No Rust reference
        // to the scratch spans generated writes; no other callback may run.
        ctx.call_inputs.values_mut()[0] = Some(source);
        Ok(Some((words, accepted)))
    }

    pub(in crate::native_artifact::program_runtime) fn finish_compiled_histogram(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        accepted: u64,
        counted: u64,
    ) -> Result<(), Error> {
        if counted != accepted {
            return Err(invalid(
                "compiled Histogram did not exhaust its admitted source",
            ));
        }
        let histogram = self
            .builder(id)?
            .histogram
            .as_ref()
            .ok_or_else(|| invalid("compiled Histogram lost its sink"))?;
        if histogram.parked || histogram.accepted != 0 || !histogram.entries.is_empty() {
            return Err(invalid("compiled Histogram finish replay"));
        }
        let bins = histogram
            .compiled_bins
            .as_ref()
            .ok_or_else(|| invalid("compiled Histogram lost its scratch"))?;
        let mut distinct = 0u64;
        let mut total = 0u64;
        // This fixed-size scan was prepaid before the native body. It does
        // not revisit source windows or charge once per source operation.
        for &count in &bins.values {
            distinct += u64::from(count != 0);
            total = add(total, count)?;
        }
        if total != accepted {
            return Err(invalid("compiled Histogram scratch count mismatch"));
        }
        let width = bins.width;
        let scratch_bytes = bins._reservation.capacity_bytes();
        let payload = mul(distinct, width as u64)?;
        let vector_bytes = mul(distinct, size_of::<Entry>() as u64)?;
        let backing = add(vector_bytes, payload)?;
        let source_bytes = ctx.call_inputs.values()[0]
            .as_ref()
            .and_then(|value| value.memory_budget_retained_bytes(&ctx.library.memory_budget()))
            .ok_or_else(|| invalid("compiled Histogram source loan disappeared"))?;
        self.histogram_output(ctx, id, distinct, payload)?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(add(self.live_bytes, source_bytes)?, backing)?,
            ctx.limits.max_temporary_bytes,
        )?;
        self.histogram_account(ctx, id, payload, 0, 0)?;
        let mut reservation = ctx.library.memory_budget().reserve_capacity(backing)?;
        let key_reservation = reservation.split_capacity(payload)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(distinct as usize)
            .map_err(|_| invalid("compiled Histogram entry allocation"))?;
        if entries.capacity() > distinct as usize {
            return Err(invalid("compiled Histogram entries exceed admitted layout"));
        }
        let bins = self
            .builder(id)?
            .histogram
            .as_mut()
            .unwrap()
            .compiled_bins
            .take()
            .unwrap();
        for (index, count) in bins.values.iter().copied().enumerate() {
            if count == 0 {
                continue;
            }
            let mut key = String::new();
            key.try_reserve_exact(width)
                .map_err(|_| invalid("compiled Histogram key allocation"))?;
            if key.capacity() > width {
                return Err(invalid("compiled Histogram key exceeds admitted layout"));
            }
            // Ascending base-128 indices are the selected canonical String
            // order for all ASCII keys, including NUL and control characters.
            if width == 2 {
                key.push((index >> 7) as u8 as char);
            }
            key.push((index & 127) as u8 as char);
            entries.push(Entry { key, count });
        }
        drop(bins); // Release actual scratch before crediting its reservation.
        let histogram = self.builder(id)?.histogram.as_mut().unwrap();
        histogram.entries = entries;
        histogram.key_payload = payload;
        histogram.accepted = accepted;
        histogram.entry_reservation = reservation;
        histogram.key_reservation = key_reservation;
        self.live_bytes = add(
            self.live_bytes
                .checked_sub(scratch_bytes)
                .ok_or_else(|| invalid("compiled Histogram scratch accounting"))?,
            backing,
        )?;
        Ok(())
    }
}

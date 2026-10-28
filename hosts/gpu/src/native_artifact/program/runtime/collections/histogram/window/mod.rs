//! Admitted storage for compiler-emitted dense windows; no source loop here.
use crate::native_artifact::program::prepare::window_histogram as layout;
use crate::native_artifact::program::runtime::collections::histogram::*;
use crate::native_artifact::program::runtime::invocation::CollectionExecution;
use mech_core::{ManagedMemoryBudget, MemoryRuntimeError, ValueData};
use mech_engine::native_artifact::NativeWindowRegion;

pub(in crate::native_artifact::program::runtime::collections::histogram) struct Bins {
    values: Vec<u64>,
    width: usize,
    _reservation: ManagedMemoryReservation,
}

impl Bins {
    fn result_capacity(&self) -> Result<u64, Error> {
        self._reservation
            .capacity_bytes()
            .checked_sub(mul(self.values.capacity() as u64, size_of::<u64>() as u64)?)
            .ok_or_else(|| invalid("compiled Histogram scratch exceeds reservation"))
    }

    fn reserve_result(
        &mut self,
        memory: &ManagedMemoryBudget,
        backing: u64,
    ) -> Result<ManagedMemoryReservation, Error> {
        let available = self.result_capacity()?;
        if available != 0 {
            if backing > available {
                return Err(invalid("compiled Histogram result exceeds banked envelope"));
            }
            // Transfer already admitted backing; shared account activity cannot
            // steal this capacity between the native scan and materialization.
            Ok(self._reservation.split_capacity(backing)?)
        } else {
            Ok(memory.reserve_capacity(backing)?)
        }
    }

    fn counts(&self) -> Result<&[u64], Error> {
        let logical = if self.values.is_empty() {
            0
        } else {
            1usize << (self.width * 7)
        };
        self.values
            .get(..logical)
            .ok_or_else(|| invalid("compiled Histogram logical extent"))
    }
}

fn reserve_layout(
    memory: &ManagedMemoryBudget,
    cells: usize,
    width: u64,
    banked: bool,
) -> Result<(usize, ManagedMemoryReservation), Error> {
    if banked {
        let peak = layout::banked_peak(width, size_of::<Entry>() as u64)
            .ok_or_else(|| invalid("compiled Histogram banked envelope overflow"))?;
        match memory.reserve_capacity(peak) {
            Ok(reservation) => {
                let physical = layout::banked_cells(width)
                    .ok_or_else(|| invalid("compiled Histogram banked width"))?;
                return Ok((physical, reservation));
            }
            // The earlier headroom observation is not an allocation permit.
            // Losing this race must not rule out the ordinary serial layout.
            Err(MemoryRuntimeError::BudgetExceeded { .. }) => {}
            Err(error) => return Err(error.into()),
        }
    }
    let bytes = mul(cells as u64, size_of::<u64>() as u64)?;
    Ok((cells, memory.reserve_capacity(bytes)?))
}

impl State {
    pub(in crate::native_artifact::program::runtime) fn publish_hash_window_bins(
        &mut self,
        id: u32,
        hashes: &crate::native_artifact::program::runtime::invocation::window::hashes::WindowHashes,
    ) -> Result<(), Error> {
        let bins = self
            .builder(id)?
            .histogram
            .as_mut()
            .and_then(|h| h.compiled_bins.as_mut())
            .ok_or_else(|| invalid("hash Histogram has no result storage"))?;
        for (key, count) in hashes.entries() {
            let index = usize::try_from(key).map_err(|_| invalid("hash Histogram result index"))?;
            *bins
                .values
                .get_mut(index)
                .ok_or_else(|| invalid("hash Histogram result outside bins"))? = count;
        }
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn prepare_compiled_histogram(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
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
        ctx.charge_input_bank_headers()?;
        let histogram = self
            .builder(id)?
            .histogram
            .as_ref()
            .ok_or_else(|| invalid("compiled Histogram has no selected sink"))?;
        if histogram.parked
            || histogram.accepted != 0
            || !histogram.entries.is_empty()
            || histogram.compiled_bins.is_some()
            || ctx.inputs().len() < 2
            || ctx.inputs().iter().any(Option::is_some)
        {
            return Err(invalid("compiled Histogram has stale owners or sink state"));
        }
        let source = ctx.arena_copy_current(region.source().get())?;
        let ValueData::String(text) = source.data() else {
            return Err(invalid("compiled Histogram requires a genuine String root"));
        };
        // Before inspecting representation, pay its full scan, including a
        // declined Unicode/CRLF variant. CRLF is one grapheme, not two bytes.
        self.histogram_account(ctx, id, 0, add(mul(text.len() as u64, 64)?, 256)?, 0)?;
        if !self.ascii_facts.inspect(&source)? {
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
        let source_bytes = source
            .memory_budget_retained_bytes(&ctx.library().memory_budget())
            .ok_or_else(|| invalid("compiled Histogram source is unadmitted"))?;
        let temporary_base = add(self.live_bytes, source_bytes)?;
        let memory = ctx.library().memory_budget();
        let banked = layout::prefer_banks(
            width,
            accepted,
            temporary_base,
            ctx.limits().max_temporary_bytes,
            memory.limit_bytes().saturating_sub(memory.used_bytes()),
            size_of::<Entry>() as u64,
        );
        let peak = if banked {
            layout::banked_peak(width, size_of::<Entry>() as u64)
                .ok_or_else(|| invalid("compiled Histogram banked envelope overflow"))?
        } else {
            mul(cells as u64, size_of::<u64>() as u64)?
        };
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(temporary_base, peak)?,
            ctx.limits().max_temporary_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(self.retained_nodes()?, 2)?,
            ctx.limits().max_retained_nodes,
        )?;
        // Covers native byte loads/count updates, scratch initialization and
        // drop, and BOTH bounded materialization scans and per-key allocation.
        // Four banks still fit this existing envelope: each start needs <32
        // primitive operations, and each logical cell adds four zero stores,
        // four reduction loads, three adds and one store (<32 with control).
        // That leaves >992 of the 1024 units per logical cell for the unchanged
        // two materialization scans and single Entry/key allocation. No extra
        // key allocations or per-start callbacks are introduced.
        // Final canonical Map validation retains its existing finite accounting.
        let work = add(add(mul(accepted, 32)?, mul(cells as u64, 1024)?)?, 1024)?;
        self.histogram_account(ctx, id, 0, work, 0)?;
        let (physical_cells, reservation) = reserve_layout(&memory, cells, width, banked)?;
        let reserved_bytes = reservation.capacity_bytes();
        #[cfg(all(test, feature = "canonical-native-program"))]
        tests::record(physical_cells, reserved_bytes, ctx.library().used().0);
        let mut values = Vec::new();
        values
            .try_reserve_exact(physical_cells)
            .map_err(|_| invalid("compiled Histogram scratch allocation"))?;
        if values.capacity() > physical_cells {
            return Err(invalid(
                "compiled Histogram scratch exceeds admitted layout",
            ));
        }
        values.resize(physical_cells, 0);
        let words = [
            text.as_ptr() as u64,
            length,
            values.as_mut_ptr() as u64,
            physical_cells as u64,
            width,
        ];
        self.builder(id)?.histogram.as_mut().unwrap().compiled_bins = Some(Bins {
            values,
            width: width as usize,
            _reservation: reservation,
        });
        self.live_bytes = add(self.live_bytes, reserved_bytes)?;
        // The actual immutable source and actual mutable scratch allocation
        // remain owned across the synchronous native call. No Rust reference
        // to the scratch spans generated writes; no other callback may run.
        ctx.inputs_mut()[0] = Some(source);
        Ok(Some((words, accepted)))
    }

    pub(in crate::native_artifact::program::runtime) fn finish_compiled_histogram(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
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
        for &count in bins.counts()? {
            distinct += u64::from(count != 0);
            total = add(total, count)?;
        }
        if total != accepted {
            return Err(invalid("compiled Histogram scratch count mismatch"));
        }
        let width = bins.width;
        let held_bytes = bins._reservation.capacity_bytes();
        let reserved_result = bins.result_capacity()?;
        let payload = mul(distinct, width as u64)?;
        let vector_bytes = mul(distinct, size_of::<Entry>() as u64)?;
        let backing = add(vector_bytes, payload)?;
        let source_bytes = ctx.inputs()[0]
            .as_ref()
            .and_then(|value| value.memory_budget_retained_bytes(&ctx.library().memory_budget()))
            .ok_or_else(|| invalid("compiled Histogram source loan disappeared"))?;
        self.histogram_output(ctx, id, distinct, payload)?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(
                add(self.live_bytes, source_bytes)?,
                backing.saturating_sub(reserved_result),
            )?,
            ctx.limits().max_temporary_bytes,
        )?;
        self.histogram_account(ctx, id, payload, 0, 0)?;
        let mut bins = self
            .builder(id)?
            .histogram
            .as_mut()
            .unwrap()
            .compiled_bins
            .take()
            .unwrap();
        let mut reservation = bins.reserve_result(&ctx.library().memory_budget(), backing)?;
        let key_reservation = reservation.split_capacity(payload)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(distinct as usize)
            .map_err(|_| invalid("compiled Histogram entry allocation"))?;
        if entries.capacity() > distinct as usize {
            return Err(invalid("compiled Histogram entries exceed admitted layout"));
        }
        for (index, count) in bins.counts()?.iter().copied().enumerate() {
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
                .checked_sub(held_bytes)
                .ok_or_else(|| invalid("compiled Histogram scratch accounting"))?,
            backing,
        )?;
        Ok(())
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;

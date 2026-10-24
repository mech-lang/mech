//! One admitted full String histogram sink. Ordinary generated CFG consumes
//! each selected String; proved window regions can fill its accumulator in bulk.
use super::{Context, Dimension, Error, State, add, check, invalid, mul};
use core::mem::size_of;
use mech_core::ManagedMemoryReservation;
use mech_engine::native_artifact::{NativeCollectionResult, NativeHistogramSink, NativeMatchValue};

#[path = "program_histogram_finish.rs"]
mod finish;
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_histogram_observation.rs"]
mod observation;
#[path = "program_window_histogram_binding.rs"]
mod window;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact) use observation::{HistogramDemand, HistogramStage, HistogramVisit};

struct Entry {
    key: String,
    count: u64,
}
pub(super) struct Builder {
    // Actual payload/vector owners precede their exact capacity reservations.
    entries: Vec<Entry>,
    compiled_bins: Option<window::Bins>,
    key_payload: u64,
    accepted: u64,
    pub(super) comparisons: u64,
    parked: bool,
    entry_reservation: ManagedMemoryReservation,
    key_reservation: ManagedMemoryReservation,
}
impl Builder {
    pub(super) fn caller_retained_nodes(&self) -> Result<u64, Error> {
        // One independently owned String per entry; packed counter bins own
        // one flat backing, irrespective of their numeric element count.
        add(add(self.entries.len() as u64, 3)?, u64::from(self.compiled_bins.is_some()))
    }
}
fn sink(
    descriptor: &crate::native_artifact::NativeProgramDescriptor,
    id: u32,
) -> Result<&NativeHistogramSink, Error> {
    match descriptor
        .plan
        .collections()
        .get(id as usize)
        .map(|c| c.result())
    {
        Some(NativeCollectionResult::Histogram(sink)) => Ok(sink),
        _ => Err(invalid(
            "full Histogram helper lacks exact selected result proof",
        )),
    }
}
fn resource(
    ctx: &Context<'_, '_>,
    id: u32,
    dimension: Dimension,
    required: u64,
    limit: Option<u64>,
) -> Result<(), Error> {
    check(
        sink(ctx.control.descriptor, id)?.template().origin(),
        dimension,
        required,
        limit,
    )
}
impl State {
    fn histogram_account(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        clones: u64,
        compute: u64,
        comparison: u64,
    ) -> Result<(), Error> {
        let definition = &ctx.control.descriptor.plan.collections()[id as usize];
        let origin = sink(ctx.control.descriptor, id)?.template().origin();
        let builder = self
            .builders
            .get_mut(id as usize)
            .and_then(Option::as_mut)
            .ok_or_else(|| invalid("Histogram account has no current builder"))?;
        let histogram = builder
            .histogram
            .as_mut()
            .ok_or_else(|| invalid("Histogram account lacks selected state"))?;
        let current = if histogram.parked {
            definition.parent_collection()
        } else {
            Some(id)
        };
        if self.current != current {
            return Err(invalid(
                "Histogram account outside exact lexical invocation",
            ));
        }
        let cloned = add(builder.cloned_bytes, clones)?;
        let computed = add(builder.compute, compute)?;
        let compared = add(histogram.comparisons, comparison)?;
        resource(
            ctx,
            id,
            Dimension::ClonedBytes,
            cloned,
            ctx.limits.max_cloned_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::ComputeWork,
            computed,
            ctx.limits.max_compute_work,
        )?;
        resource(
            ctx,
            id,
            Dimension::ComparisonWork,
            compared,
            ctx.limits.max_comparison_work,
        )?;
        // NativeWorkMeter is attached to this very account. Do not debit both
        // meters or introduce/reset a per-yield comparison allowance.
        ctx.library
            .charge(compute, comparison)
            .map_err(|error| Error::Collection { origin, error })?;
        builder.cloned_bytes = cloned;
        builder.compute = computed;
        histogram.comparisons = compared;
        Ok(())
    }
    fn histogram_output(
        &self,
        ctx: &Context<'_, '_>,
        id: u32,
        count: u64,
        payload: u64,
    ) -> Result<(), Error> {
        let definition = sink(ctx.control.descriptor, id)?;
        let origin = definition.template().origin();
        let selected = ctx.control.descriptor.calls[definition.consumer() as usize]
            .as_ref()
            .ok_or_else(|| invalid("Histogram output lacks selected factory"))?;
        let footprint = selected
            .selected_histogram_footprint(count, payload)
            .map_err(|error| Error::Library { origin, error })?;
        resource(
            ctx,
            id,
            Dimension::OutputElements,
            count,
            ctx.limits.max_output_elements,
        )?;
        resource(
            ctx,
            id,
            Dimension::OutputBytes,
            footprint.retained_bytes,
            ctx.limits.max_output_bytes,
        )?;
        // Three complete semantic generations (accumulator/draft/frozen), plus
        // current yield, vector replacement and fixed builder metadata slack.
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(
                add(mul(footprint.node_count, 3)?, 5)?,
                self.retained_nodes()?,
            )?,
            ctx.limits.max_retained_nodes,
        )
    }
    pub(super) fn begin_histogram(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let NativeCollectionResult::Histogram(definition) =
            ctx.control.descriptor.plan.collections()[id as usize].result()
        else {
            return Ok(());
        };
        let selected = ctx.control.descriptor.calls[definition.consumer() as usize]
            .as_ref()
            .ok_or_else(|| invalid("Histogram begin lacks actual selected factory"))?;
        let bytes = selected.binding_metadata_bytes();
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(self.live_bytes, bytes)?,
            ctx.limits.max_temporary_bytes,
        )?;
        self.account(ctx, id, 0, selected.binding_work())?;
        let _binding = ctx.library.memory_budget().reserve_capacity(bytes)?;
        if selected.template() != definition.template() {
            return Err(invalid(
                "Histogram selection differs after complete binding admission",
            ));
        }
        // Every append prepays 128 compute; every yielded byte prepays at
        // least one compute, including duplicates. The SAME finite ledger
        // bounds accepted A<=W/128, all yielded payload<=W and actual D<=A.
        // Prove original producer/canonical Map representability before any
        // yield can be discarded. No allocation, guessed D, or late fallback.
        let work = ctx.control.work.limit().min(
            ctx.limits
                .max_compute_work
                .ok_or_else(|| invalid("Histogram requires finite compute work"))?,
        );
        let parameters = definition.template().inputs()[0]
            .dimension_parameters()
            .len()
            .max(definition.template().output().dimension_parameters().len());
        self.account(ctx, id, 0, super::histogram_layout::CERTIFICATION_WORK)?;
        let original_scratch = selected
            .selected_histogram_original_scratch_bytes(work / 128)
            .map_err(|error| Error::Library {
                origin: definition.template().origin(),
                error,
            })?;
        super::histogram_layout::certify(
            work,
            parameters,
            original_scratch,
            ctx.control.descriptor.scalar_bytes,
        )?;
        let memory = ctx.library.memory_budget();
        self.builder(id)?.histogram = Some(Builder {
            entries: Vec::new(),
            compiled_bins: None,
            key_payload: 0,
            accepted: 0,
            comparisons: 0,
            parked: false,
            entry_reservation: memory.reserve_capacity(0)?,
            key_reservation: memory.reserve_capacity(0)?,
        });
        self.histogram_account(ctx, id, 0, 128, 0)?;
        self.histogram_output(ctx, id, 0, 0)
    }
    pub(super) fn push_histogram(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        value: NativeMatchValue,
    ) -> Result<(), Error> {
        let yielded = self.take_string_yield(ctx, id, value)?;
        let result = self.append_histogram(ctx, id, &yielded);
        self.release_string_yield(yielded)?;
        result
    }

    fn append_histogram(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        yielded: &super::string_span::Yield,
    ) -> Result<(), Error> {
        let definition = sink(ctx.control.descriptor, id)?;
        let origin = definition.template().origin();
        let selected = ctx.control.descriptor.calls[definition.consumer() as usize]
            .as_ref()
            .ok_or_else(|| invalid("Histogram append lacks selected semantics"))?;
        let part = yielded.text()?;
        self.histogram_account(ctx, id, 0, 128, 0)?;
        // A duplicate still retains its actual current yielded owner during
        // comparison/update. Fund alias inspection and that finite peak even
        // when neither a new key nor a vector allocation will be needed.
        let alias_work = self.string_yield_alias_work(yielded)?;
        self.histogram_account(ctx, id, 0, alias_work, 0)?;
        let untracked = self.untracked_string_yield(ctx, yielded)?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        if untracked > 0 {
            let (distinct, payload) = {
                let histogram = self.builder(id)?.histogram.as_ref().unwrap();
                (histogram.entries.len() as u64, histogram.key_payload)
            };
            self.test_histogram_record(
                ctx,
                id,
                HistogramStage::Yield,
                0,
                add(self.live_bytes, untracked)?,
                0,
                0,
                distinct,
                payload,
                yielded
                    .owner
                    .memory_budget_retained_bytes(&ctx.library.memory_budget())
                    .ok_or_else(|| invalid("observed current Histogram yield"))?,
                untracked,
            )?;
        }
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(self.live_bytes, untracked)?,
            ctx.limits.max_temporary_bytes,
        )?;
        let mut left = 0usize;
        let mut right = self.builder(id)?.histogram.as_ref().unwrap().entries.len();
        let mut found = None;
        while left < right {
            self.histogram_account(ctx, id, 0, 64, 0)?;
            let middle = left + (right - left) / 2;
            let peer_length = self.builder(id)?.histogram.as_ref().unwrap().entries[middle]
                .key
                .len();
            let comparison = (part.len().max(peer_length) as u64).max(1);
            self.histogram_account(ctx, id, 0, comparison, comparison)?;
            let order = selected
                .selected_histogram_compare(
                    &self.builder(id)?.histogram.as_ref().unwrap().entries[middle].key,
                    part,
                )
                .map_err(|error| Error::Library { origin, error })?;
            match order {
                core::cmp::Ordering::Less => left = middle + 1,
                core::cmp::Ordering::Greater => right = middle,
                core::cmp::Ordering::Equal => {
                    found = Some(middle);
                    break;
                }
            }
        }
        let accepted = self
            .builder(id)?
            .histogram
            .as_ref()
            .unwrap()
            .accepted
            .checked_add(1)
            .ok_or_else(|| invalid("Histogram accepted count exceeds u64"))?;
        if let Some(index) = found {
            let old = self.builder(id)?.histogram.as_ref().unwrap().entries[index].count;
            self.histogram_account(ctx, id, 0, 128, 0)?;
            let count = selected
                .selected_histogram_increment(old)
                .map_err(|error| Error::Library { origin, error })?;
            let (distinct, payload) = {
                let histogram = self.builder(id)?.histogram.as_ref().unwrap();
                (histogram.entries.len() as u64, histogram.key_payload)
            };
            self.histogram_output(ctx, id, distinct, payload)?;
            let histogram = self.builder(id)?.histogram.as_mut().unwrap();
            histogram.entries[index].count = count;
            histogram.accepted = accepted;
            return Ok(());
        }
        let histogram = self.builder(id)?.histogram.as_ref().unwrap();
        if histogram.parked {
            return Err(invalid("Histogram append after producer finish"));
        }
        let count = histogram
            .entries
            .len()
            .checked_add(1)
            .ok_or_else(|| invalid("Histogram distinct count overflow"))?;
        let old_capacity = histogram.entries.capacity();
        let capacity = if count > old_capacity {
            old_capacity
                .checked_mul(2)
                .map(|n| n.max(4))
                .ok_or_else(|| invalid("Histogram vector capacity overflow"))?
        } else {
            old_capacity
        };
        let growing = capacity > old_capacity;
        let vector_bytes = if growing {
            mul(capacity as u64, size_of::<Entry>() as u64)?
        } else {
            0
        };
        let key_bytes = part.len() as u64;
        let payload = add(histogram.key_payload, key_bytes)?;
        let pending_bytes = add(vector_bytes, key_bytes)?;
        let moved = mul(
            (if growing {
                histogram.entries.len()
            } else {
                histogram.entries.len() - left
            }) as u64,
            size_of::<Entry>() as u64,
        )?;
        let compute = add(add(mul(pending_bytes, 16)?, mul(moved, 4)?)?, 256)?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        self.test_histogram_record(
            ctx,
            id,
            HistogramStage::Growth,
            pending_bytes,
            add(add(self.live_bytes, pending_bytes)?, untracked)?,
            compute,
            key_bytes,
            count as u64,
            payload,
            yielded
                .owner
                .memory_budget_retained_bytes(&ctx.library.memory_budget())
                .ok_or_else(|| invalid("observed Histogram yield must be admitted"))?,
            untracked,
        )?;
        self.histogram_output(ctx, id, count as u64, payload)?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(add(self.live_bytes, pending_bytes)?, untracked)?,
            ctx.limits.max_temporary_bytes,
        )?;
        // Include all new backing, owned key copying, element relocation and
        // eventual cleanup before reserving/allocating/copying either buffer.
        self.histogram_account(ctx, id, key_bytes, compute, 0)?;
        let mut pending = ctx
            .library
            .memory_budget()
            .reserve_capacity(pending_bytes)?;
        let mut key = String::new();
        key.try_reserve_exact(part.len())
            .map_err(|_| invalid("Histogram key allocation"))?;
        if key.capacity() as u64 > key_bytes {
            return Err(invalid(
                "Histogram key capacity exceeds exact admitted layout",
            ));
        }
        let mut replacement = if growing {
            let mut entries = Vec::new();
            entries
                .try_reserve_exact(capacity)
                .map_err(|_| invalid("Histogram entry allocation"))?;
            if entries.capacity() > capacity {
                return Err(invalid(
                    "Histogram vector capacity exceeds exact admitted layout",
                ));
            }
            Some(entries)
        } else {
            None
        };
        key.push_str(part);
        let key_reservation = pending.split_capacity(key.capacity() as u64)?;
        let old_vector_bytes = self
            .builder(id)?
            .histogram
            .as_ref()
            .unwrap()
            .entry_reservation
            .capacity_bytes();
        let histogram = self.builder(id)?.histogram.as_mut().unwrap();
        histogram.key_reservation.merge_capacity(key_reservation)?;
        let entry = Entry { key, count: 1 };
        if let Some(entries) = replacement.as_mut() {
            // Moving String owners neither clones payload nor releases their
            // reservation. Old Vec metadata coexists until its iterator drops.
            let previous = core::mem::take(&mut histogram.entries);
            let mut fresh = Some(entry);
            for (index, old) in previous.into_iter().enumerate() {
                if index == left {
                    entries.push(fresh.take().unwrap());
                }
                entries.push(old);
            }
            if let Some(entry) = fresh {
                entries.push(entry);
            }
            histogram.entries = replacement.take().unwrap();
            histogram.entry_reservation = pending;
        } else {
            histogram.entries.insert(left, entry);
        }
        histogram.key_payload = payload;
        histogram.accepted = accepted;
        self.live_bytes = add(
            self.live_bytes
                .checked_sub(if growing { old_vector_bytes } else { 0 })
                .ok_or_else(|| invalid("Histogram old vector accounting"))?,
            pending_bytes,
        )?;
        Ok(())
    }
}

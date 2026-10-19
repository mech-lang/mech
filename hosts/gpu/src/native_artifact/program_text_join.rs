//! An admitted selected TextJoin append sink. This helper accepts ONE already
//! evaluated yield; emitted CFG alone runs every source/body/filter/branch.
use super::{Context, Dimension, Error, State, add, charge, check, invalid, mul};
use crate::native_artifact::NativeProgramDescriptor;
use mech_core::{
    ManagedMemoryReservation, Value, ValueData,
    snapshot::{
        SnapshotCanonicalizationBudget, SnapshotValidationContext, ValueDataDraft, ValueDraft,
        prospective_string_value_footprint,
    },
};
use mech_engine::native_artifact::{NativeCollectionResult, NativeMatchValue, NativeTextJoinSink};
#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_text_join_observation.rs"]
mod observation;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact) use observation::{TextJoinDemand, TextJoinStage};

pub(super) struct Builder {
    // Payloads are destroyed before the parent reservation is released.
    text: String,
    separator: Value,
    separator_bytes: u64,
    accepted: u64,
    parked: bool,
}
fn sink(descriptor: &NativeProgramDescriptor, id: u32) -> Result<&NativeTextJoinSink, Error> {
    match descriptor
        .plan
        .collections()
        .get(id as usize)
        .map(|c| c.result())
    {
        Some(NativeCollectionResult::TextJoin(sink)) => Ok(sink),
        _ => Err(invalid("TextJoin helper lacks exact selected strategy")),
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
    pub(super) fn begin_text_join(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let NativeCollectionResult::TextJoin(definition) =
            ctx.control.descriptor.plan.collections()[id as usize].result()
        else {
            return Ok(());
        };
        let selected = ctx.control.descriptor.calls[definition.consumer() as usize]
            .as_ref()
            .ok_or_else(|| invalid("TextJoin sink lacks retained selected factory"))?;
        let binding_bytes = selected.binding_metadata_bytes();
        let binding_work = selected.binding_work();
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(self.live_bytes, binding_bytes)?,
            ctx.limits.max_temporary_bytes,
        )?;
        self.account(ctx, id, 0, binding_work)?;
        let _binding = ctx
            .library
            .memory_budget()
            .reserve_capacity(binding_bytes)?;
        // Retained exact template equality is deep; its full binding envelope
        // is prepaid above, once for this actual builder invocation.
        if selected.template() != definition.template() {
            return Err(invalid("TextJoin sink selection differs"));
        }
        let separator = ctx
            .arena
            .copy_current(definition.separator().get(), &mut ctx.control.work)?;
        if !matches!(separator.data(), ValueData::String(_)) {
            return Err(invalid("TextJoin separator is not String"));
        }
        let separator_bytes = separator
            .memory_budget_retained_bytes(&ctx.library.memory_budget())
            .ok_or_else(|| invalid("unadmitted TextJoin separator"))?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(add(self.live_bytes, separator_bytes)?, binding_bytes)?,
            ctx.limits.max_temporary_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::OutputElements,
            1,
            ctx.limits.max_output_elements,
        )?;
        // Prospective two-node String + current yielded Value (two), captured
        // separator (two), and an old backing during replacement (one).
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(7, self.retained_nodes()?)?,
            ctx.limits.max_retained_nodes,
        )?;
        resource(
            ctx,
            id,
            Dimension::ComparisonWork,
            0,
            ctx.limits.max_comparison_work,
        )?;
        resource(
            ctx,
            id,
            Dimension::ClonedBytes,
            0,
            ctx.limits.max_cloned_bytes,
        )?;
        let empty = prospective_string_value_footprint(0, false, 1, 0)
            .map_err(|_| invalid("TextJoin empty footprint"))?;
        resource(
            ctx,
            id,
            Dimension::OutputBytes,
            empty.retained_bytes,
            ctx.limits.max_output_bytes,
        )?;
        self.builder(id)?.join = Some(Builder {
            text: String::new(),
            separator,
            separator_bytes,
            accepted: 0,
            parked: false,
        });
        self.live_bytes = add(self.live_bytes, separator_bytes)?;
        Ok(())
    }

    pub(super) fn push_text_join(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        value: NativeMatchValue,
    ) -> Result<(), Error> {
        let yielded = self.take_string_yield(ctx, id, value)?;
        let result = self.append_text_join(ctx, id, &yielded);
        self.release_string_yield(yielded)?;
        result
    }

    fn append_text_join(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
        yielded: &super::string_span::Yield,
    ) -> Result<(), Error> {
        let definition = sink(ctx.control.descriptor, id)?;
        let selected = ctx.control.descriptor.calls[definition.consumer() as usize]
            .as_ref()
            .ok_or_else(|| invalid("TextJoin missing selected append"))?;
        let part = yielded.text()?;
        let (count, length, capacity, old_length, append) = {
            let builder = self
                .builder(id)?
                .join
                .as_ref()
                .ok_or_else(|| invalid("TextJoin append before begin"))?;
            if builder.parked {
                return Err(invalid("TextJoin append after producer finish"));
            }
            let ValueData::String(separator) = builder.separator.data() else {
                return Err(invalid("TextJoin retained separator kind"));
            };
            let append = add(
                part.len() as u64,
                if builder.accepted != 0 {
                    separator.len() as u64
                } else {
                    0
                },
            )?;
            let length = add(builder.text.len() as u64, append)?;
            let old_capacity = builder.text.capacity() as u64;
            let capacity = if length > old_capacity {
                mul(old_capacity, 2)?.max(64).max(length)
            } else {
                old_capacity
            };
            (
                add(builder.accepted, 1)?,
                length,
                capacity,
                builder.text.len() as u64,
                append,
            )
        };
        let growing = capacity > self.builder(id)?.reservation.capacity_bytes();
        let alias_work = self.string_yield_alias_work(yielded)?;
        self.account(ctx, id, 0, alias_work)?;
        let untracked = self.untracked_string_yield(ctx, yielded)?;
        let footprint = prospective_string_value_footprint(0, false, 1, length)
            .map_err(|_| invalid("TextJoin output footprint overflow"))?;
        resource(
            ctx,
            id,
            Dimension::OutputElements,
            1,
            ctx.limits.max_output_elements,
        )?;
        resource(
            ctx,
            id,
            Dimension::OutputBytes,
            footprint.retained_bytes,
            ctx.limits.max_output_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(add(footprint.node_count, 5)?, self.retained_nodes()?)?,
            ctx.limits.max_retained_nodes,
        )?;
        // A growth uses a new allocation while the complete old allocation
        // and current yielded Value remain live. No future output credit.
        let prospective = add(
            add(self.live_bytes, if growing { capacity } else { 0 })?,
            untracked,
        )?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            prospective,
            ctx.limits.max_temporary_bytes,
        )?;
        let moved = if growing { old_length } else { 0 };
        // Prepay actual append copies, relocation, eventual backing teardown,
        // metadata/identity inspection and all failure cleanup, cumulatively.
        let compute = add(
            add(
                add(mul(append, 4)?, mul(moved, 4)?)?,
                if growing {
                    add(mul(capacity, 4)?, 128)?
                } else {
                    0
                },
            )?,
            128,
        )?;
        self.account(ctx, id, add(append, moved)?, compute)?;
        let mut replacement: Option<(String, ManagedMemoryReservation)> = if growing {
            let reservation = ctx.library.memory_budget().reserve_capacity(capacity)?;
            let mut text = String::new();
            text.try_reserve_exact(
                usize::try_from(capacity).map_err(|_| invalid("TextJoin capacity exceeds host"))?,
            )
            .map_err(|_| invalid("TextJoin backing allocation"))?;
            let previous = &self.builder(id)?.join.as_ref().unwrap().text;
            text.push_str(previous);
            Some((text, reservation))
        } else {
            None
        };
        let old_capacity = self.builder(id)?.reservation.capacity_bytes();
        let builder = self.builder(id)?;
        let join = builder.join.as_mut().unwrap();
        if let Some((text, reservation)) = replacement.take() {
            // Drop actual old backing BEFORE releasing its reservation.
            let previous = core::mem::replace(&mut join.text, text);
            drop(previous);
            builder.reservation = reservation;
        }
        let ValueData::String(separator) = join.separator.data() else {
            unreachable!("validated immutable separator")
        };
        selected
            .selected_text_join_append(&mut join.text, separator, join.accepted != 0, part)
            .map_err(|error| Error::Library {
                origin: definition.template().origin(),
                error,
            })?;
        join.accepted = count;
        builder.payload_bytes = length;
        self.live_bytes = add(
            self.live_bytes
                .checked_sub(old_capacity)
                .ok_or_else(|| invalid("TextJoin previous capacity accounting"))?,
            capacity,
        )?;
        Ok(())
    }

    pub(super) fn park_text_join(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let definition = &ctx.control.descriptor.plan.collections()[id as usize];
        // Eliding the Matrix does NOT elide its declared shape obligations.
        // Validate the exact original 1xN shape at the original producer finish
        // after the same body visits, with metadata allocation/work prepaid.
        let metadata = ctx.control.descriptor.scalar_bytes;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(self.live_bytes, metadata)?,
            ctx.limits.max_temporary_bytes,
        )?;
        self.account(ctx, id, 0, add(mul(metadata, 8)?, 64)?)?;
        let _shape = ctx.library.memory_budget().reserve_capacity(metadata)?;
        let join = self
            .builder(id)?
            .join
            .as_mut()
            .ok_or_else(|| invalid("TextJoin producer finish before begin"))?;
        if join.parked {
            return Err(invalid("duplicate TextJoin producer finish"));
        }
        mech_core::shape_for_resolved_extents(
            ctx.control.descriptor.plan.values()[definition.output().get() as usize].definition(),
            &[1, join.accepted],
        )
        .map_err(|_| invalid("collected output shape"))?;
        join.parked = true;
        self.current = definition.parent_collection();
        Ok(())
    }

    pub(super) fn finish_text_join(
        &mut self,
        ctx: &mut Context<'_, '_>,
        id: u32,
    ) -> Result<(), Error> {
        let definition = sink(ctx.control.descriptor, id)?;
        let collection = &ctx.control.descriptor.plan.collections()[id as usize];
        if self.current != collection.parent_collection() {
            return Err(invalid(
                "TextJoin consumer outside exact lexical invocation",
            ));
        }
        let builder = self
            .builders
            .get(id as usize)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("TextJoin consumer without producer"))?;
        let join = builder
            .join
            .as_ref()
            .filter(|join| join.parked)
            .ok_or_else(|| invalid("TextJoin consumer before producer completion"))?;
        let payload = join.text.len() as u64;
        let footprint = prospective_string_value_footprint(0, false, 1, payload)
            .map_err(|_| invalid("TextJoin final footprint"))?;
        let capacity = builder.reservation.capacity_bytes();
        let separator_bytes = join.separator_bytes;
        let envelope = add(
            ctx.control.descriptor.scalar_bytes,
            add(mul(payload, 4)?, 4096)?,
        )?;
        let finalization = add(mul(payload, 8)?, 32)?;
        let compute = add(add(mul(envelope, 16)?, finalization)?, 256)?;
        // Parent active-builder identity is restored while this parked child
        // is completed. Do not call builder()/account(), which require active.
        let cloned = add(builder.cloned_bytes, payload)?;
        let accumulated = add(builder.compute, compute)?;
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
            accumulated,
            ctx.limits.max_compute_work,
        )?;
        resource(
            ctx,
            id,
            Dimension::TemporaryBytes,
            add(self.live_bytes, envelope)?,
            ctx.limits.max_temporary_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::OutputBytes,
            footprint.retained_bytes,
            ctx.limits.max_output_bytes,
        )?;
        resource(
            ctx,
            id,
            Dimension::OutputElements,
            1,
            ctx.limits.max_output_elements,
        )?;
        resource(
            ctx,
            id,
            Dimension::RetainedNodes,
            add(add(footprint.node_count, 5)?, self.retained_nodes()?)?,
            ctx.limits.max_retained_nodes,
        )?;
        // There is no previous canonical value in an SSA sink-result slot.
        // Keep the existing arena/session transaction-publication behavior;
        // no fabricated per-yield equality or renewed comparison ledger.
        charge(&mut ctx.control.work, compute)?;
        let mut reservation = ctx.library.memory_budget().reserve_capacity(envelope)?;
        let builder = self.builders[id as usize].as_mut().unwrap();
        builder.cloned_bytes = cloned;
        builder.compute = accumulated;
        let text = core::mem::take(&mut builder.join.as_mut().unwrap().text);
        let output = &ctx.control.descriptor.plan.values()[definition.output().get() as usize];
        let canonical = SnapshotCanonicalizationBudget::new(finalization);
        let value = ValueDraft {
            schema: output.schema(),
            shape_values: Box::new([]),
            data: ValueDataDraft::String(text),
        }
        .finalize(
            &SnapshotValidationContext::with_shared_schemas(&ctx.control.descriptor.schemas)
                .with_canonicalization_budget(&canonical),
        )
        .map_err(Error::Snapshot)?
        .into_memory_budget(&mut reservation)?;
        ctx.arena
            .put(definition.output().get(), &value, &mut ctx.control.work)?;
        let builder = self.builders[id as usize].take().unwrap();
        drop(builder); // Retire separator and remaining backing before credit.
        self.live_bytes = self
            .live_bytes
            .checked_sub(add(capacity, separator_bytes)?)
            .ok_or_else(|| invalid("TextJoin completion capacity accounting"))?;
        Ok(())
    }
}

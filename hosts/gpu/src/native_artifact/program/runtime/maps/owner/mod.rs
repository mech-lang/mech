//! Invocation-bound retained tables for a checked generated caller. Transport
//! selection remains separate; these methods never evaluate a source body.
use crate::native_artifact::program::runtime::invocation::{Invocation, MapExecution};
use crate::native_artifact::program::runtime::maps::admission::Error as StorageError;
use crate::native_artifact::program::runtime::maps::binding::{Binding, InitialWords, Key};
use crate::native_artifact::program::runtime::maps::string::PublicationGeometry;
use crate::native_artifact::program::runtime::{
    NativeProgramExecutionError as Error, amount, invalid,
};
use crate::native_artifact::{
    NativeExecutionError, NativeExecutionPhase as Phase, NativeProgramDescriptor, NativeWorkMeter,
};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, ManagedNodeReservation, Value};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMapOwnerRegion};

mod caller_geometry;
mod closed;
pub(in crate::native_artifact::program::runtime) mod frozen;
mod spans;
use crate::native_artifact::program::runtime::maps::owner::frozen::FrozenSlot;

pub(in crate::native_artifact::program::runtime) enum Publication {
    Canonical(Value),
    Frozen(u64),
}

/// Lazy owning-session lane. No table metadata is allocated merely because a
/// descriptor has a candidate; entry must request and admit it explicitly.
pub(in crate::native_artifact::program::runtime) struct Session<'plan> {
    descriptor: &'plan NativeProgramDescriptor,
    state: Option<State<'plan>>,
    refused_visit: Option<u64>,
    pub(in crate::native_artifact::program::runtime) closed_exit: Option<(u32, u32)>,
}
impl<'plan> Session<'plan> {
    pub(in crate::native_artifact::program::runtime) fn new(
        descriptor: &'plan NativeProgramDescriptor,
    ) -> Self {
        Self {
            descriptor,
            state: None,
            refused_visit: None,
            closed_exit: None,
        }
    }
    pub(in crate::native_artifact::program::runtime) fn has_retained_binding(
        &self,
        index: usize,
    ) -> bool {
        self.state
            .as_ref()
            .and_then(|state| state.slots.get(index))
            .is_some_and(Slot::has_retained_binding)
    }
}

struct Slot<'plan> {
    binding: Option<Binding<'plan>>,
    closed: Option<Box<closed::Owner<'plan>>>,
    frozen: Option<FrozenSlot>,
}
// These operations move already admitted storage in the existing order. They
// add no rollback or retry path, and keep the original three Option fields.
impl<'plan> Slot<'plan> {
    fn vacant() -> Self {
        Self {
            binding: None,
            closed: None,
            frozen: None,
        }
    }
    fn has_retained_binding(&self) -> bool {
        self.binding.is_some()
    }
    fn retain(&mut self, binding: Binding<'plan>) {
        self.binding = Some(binding);
    }
    fn close(&mut self, owner: Box<closed::Owner<'plan>>) {
        self.closed = Some(owner);
    }
    fn stage_frozen(&mut self, frozen: FrozenSlot) {
        self.frozen = Some(frozen);
    }
    fn mark_frozen_published(&mut self, token: u64) {
        self.frozen.as_mut().unwrap().token = token;
    }
    fn take_binding(&mut self) -> Option<Binding<'plan>> {
        self.binding.take()
    }
    fn take_closed(&mut self) -> Option<Box<closed::Owner<'plan>>> {
        self.closed.take()
    }
    fn retire_frozen(&mut self) {
        drop(self.frozen.take());
    }
}

#[cfg(test)]
pub(in crate::native_artifact::program::runtime) enum SlotPhase<'slot, 'plan> {
    Vacant,
    Retained(&'slot Binding<'plan>),
    Closed(&'slot closed::Owner<'plan>),
    FrozenPending {
        binding: &'slot Binding<'plan>,
        frozen: &'slot FrozenSlot,
    },
    FrozenPublished(&'slot FrozenSlot),
    // Cleanup-only error states are observable, but never resumable.
    TerminalPartial {
        binding: Option<&'slot Binding<'plan>>,
        closed: Option<&'slot closed::Owner<'plan>>,
        frozen: Option<&'slot FrozenSlot>,
    },
}
#[cfg(test)]
impl core::fmt::Debug for SlotPhase<'_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Vacant => f.write_str("Vacant"),
            Self::Retained(binding) => f
                .debug_tuple("Retained")
                .field(&core::ptr::from_ref(*binding))
                .finish(),
            Self::Closed(owner) => f
                .debug_tuple("Closed")
                .field(&core::ptr::from_ref(*owner))
                .finish(),
            Self::FrozenPending { binding, frozen } => f
                .debug_struct("FrozenPending")
                .field("binding", &core::ptr::from_ref(*binding))
                .field("token", &frozen.token)
                .field("parent", &frozen.parent)
                .field("born_visit", &frozen.born_visit)
                .finish(),
            Self::FrozenPublished(frozen) => f
                .debug_struct("FrozenPublished")
                .field("token", &frozen.token)
                .field("parent", &frozen.parent)
                .field("born_visit", &frozen.born_visit)
                .finish(),
            Self::TerminalPartial {
                binding,
                closed,
                frozen,
            } => f
                .debug_struct("TerminalPartial")
                .field("binding", &binding.is_some())
                .field("closed", &closed.is_some())
                .field("frozen_token", &frozen.map(|f| f.token))
                .finish(),
        }
    }
}
#[cfg(test)]
impl Slot<'_> {
    fn phase(&self) -> SlotPhase<'_, '_> {
        match (&self.binding, &self.closed, &self.frozen) {
            (None, None, None) => SlotPhase::Vacant,
            (Some(binding), None, None) => SlotPhase::Retained(binding),
            (None, Some(owner), None) => SlotPhase::Closed(owner),
            (Some(binding), None, Some(frozen)) if frozen.token == 0 => {
                SlotPhase::FrozenPending { binding, frozen }
            }
            (None, None, Some(frozen)) if frozen.token != 0 => SlotPhase::FrozenPublished(frozen),
            (binding, closed, frozen) => SlotPhase::TerminalPartial {
                binding: binding.as_ref(),
                closed: closed.as_deref(),
                frozen: frozen.as_ref(),
            },
        }
    }
}
#[cfg(test)]
impl Session<'_> {
    pub(in crate::native_artifact::program::runtime) fn slot_phase(
        &self,
        index: usize,
    ) -> Option<SlotPhase<'_, '_>> {
        self.state.as_ref()?.slots.get(index).map(Slot::phase)
    }
}

pub(in crate::native_artifact::program::runtime) struct State<'plan> {
    // Tables/keys retire before the enclosing admitted fixed lane.
    slots: Box<[Slot<'plan>]>,
    spans: Box<[Option<spans::Lane>]>,
    descriptor: &'plan NativeProgramDescriptor,
    work_identity: u64,
    active_tables: u64,
    last_owned_visit: Option<u64>,
    caller: Option<caller_geometry::Caller>,
    reservation: ManagedMemoryReservation,
    _nodes: ManagedNodeReservation,
}

fn storage_error(error: StorageError) -> Error {
    match error {
        StorageError::Memory(error) => NativeExecutionError::Memory { error }.into(),
        StorageError::Work(error) => NativeExecutionError::LibraryWork {
            phase: Phase::Setup,
            error,
        }
        .into(),
        StorageError::Binding(reason) | StorageError::Publication(reason) => invalid(reason),
        StorageError::Overflow => invalid("retained map size/work overflow"),
        StorageError::Allocation => invalid("retained map allocation failed"),
        StorageError::Limit { field, .. } => invalid(field),
    }
}
fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b)
        .ok_or_else(|| invalid("retained map caller geometry overflow"))
}

impl<'plan> State<'plan> {
    #[cfg(test)]
    pub(in crate::native_artifact::program::runtime) fn new(
        descriptor: &'plan NativeProgramDescriptor,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
        limit: u64,
    ) -> Result<Self, Error> {
        Self::try_new(descriptor, memory, work, limit)?
            .ok_or_else(|| invalid("retained map owner lane admission refused"))
    }

    fn try_new(
        descriptor: &'plan NativeProgramDescriptor,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
        limit: u64,
    ) -> Result<Option<Self>, Error> {
        work.check_memory_budget(memory)?;
        let count = descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .map_or(0, |layout| layout.regions().len());
        let span_count = descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .map_or(0, |layout| layout.spans().len());
        let bytes = add(
            amount(
                count,
                (2 * core::mem::size_of::<Slot<'plan>>()) as u64,
                (core::mem::size_of::<Self>() + 128) as u64,
            )?,
            amount(
                span_count,
                (2 * core::mem::size_of::<Option<spans::Lane>>()) as u64,
                0,
            )?,
        )?;
        if bytes > limit {
            return Ok(None);
        }
        work.charge(
            Phase::Setup,
            amount(
                count
                    .checked_add(span_count)
                    .ok_or_else(|| invalid("map lane count overflow"))?,
                512,
                512,
            )?,
        )?;
        let Ok(reservation) = memory.reserve_capacity(bytes) else {
            return Ok(None);
        };
        let Ok(nodes) = memory.reserve_nodes(bytes) else {
            return Ok(None);
        };
        let mut slots = Vec::new();
        if slots.try_reserve_exact(count).is_err() || slots.capacity() != count {
            return Ok(None);
        }
        slots.resize_with(count, Slot::vacant);
        let mut spans = Vec::new();
        if spans.try_reserve_exact(span_count).is_err() || spans.capacity() != span_count {
            return Ok(None);
        }
        spans.resize_with(span_count, || None);
        Ok(Some(Self {
            slots: slots.into_boxed_slice(),
            spans: spans.into_boxed_slice(),
            descriptor,
            work_identity: work.identity(),
            active_tables: 0,
            last_owned_visit: None,
            caller: None,
            reservation,
            _nodes: nodes,
        }))
    }

    fn guard(&self, ctx: &MapExecution<'_, '_, '_, '_>, site: usize) -> Result<(), Error> {
        let site = u32::try_from(site).map_err(|_| invalid("retained map site overflow"))?;
        if !core::ptr::eq(self.descriptor, ctx.descriptor())
            || self.work_identity != ctx.control().work.identity()
            || self.reservation.budget() != ctx.library().memory_budget()
        {
            return Err(invalid(
                "retained map lane belongs to another descriptor/invocation",
            ));
        }
        if ctx.control().previous != Some(site)
            || ctx.control().completed
            || ctx.consumed()
            || self.last_owned_visit == Some(ctx.control().visits)
        {
            return Err(invalid(
                "retained map operation outside fresh generated visit",
            ));
        }
        Ok(())
    }

    fn region(&self, index: usize) -> Result<&'plan NativeMapOwnerRegion, Error> {
        self.descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.regions().get(index))
            .ok_or_else(|| invalid("retained map region is outside descriptor"))
    }

    fn geometry(
        &self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
    ) -> Result<Option<PublicationGeometry>, Error> {
        let Some(mut geometry) = self.caller_geometry(ctx, site)? else {
            return Ok(None);
        };
        ctx.work().charge(
            Phase::Setup,
            add(
                amount(self.slots.len(), 128, 128)?,
                amount(self.spans.len(), 64, 64)?,
            )?,
        )?;
        geometry.live_nodes = add(geometry.live_nodes, self.reservation.capacity_bytes())?;
        for slot in &self.slots {
            if let Some(frozen) = &slot.frozen {
                geometry.live_nodes = add(geometry.live_nodes, frozen.map.retained_nodes())?;
            }
            if let Some(binding) = &slot.binding {
                geometry.live_nodes = add(
                    geometry.live_nodes,
                    binding.retained_nodes().map_err(storage_error)?,
                )?;
            }
        }
        for span in &self.spans {
            if matches!(span, Some(spans::Lane::Span { .. })) {
                geometry.live_nodes = add(geometry.live_nodes, 2)?;
            }
        }
        // Every table/key/lane reservation shares this exact account. Snapshot
        // nodes come from the base view; private table nodes are added above.
        geometry.live_bytes = ctx.library().memory_budget().used_bytes();
        Ok(Some(geometry))
    }

    pub(in crate::native_artifact::program::runtime) fn try_begin_retained(
        &mut self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
    ) -> Result<Option<InitialWords>, Error> {
        self.guard(ctx, site)?;
        let index = self
            .descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.initializer(site))
            .ok_or_else(|| invalid("retained map entry outside initializer"))?;
        let region = self.region(index)?;
        if ctx.control().current != Some(region.iteration())
            || self.slots[index].binding.is_some()
            || self.slots[index].frozen.is_some()
        {
            return Err(invalid("retained map entry overlaps another live owner"));
        }
        let Some(geometry) = self.geometry(ctx, site)? else {
            self.last_owned_visit = Some(ctx.control().visits);
            return Ok(None);
        };
        let source = ctx.arena_copy_current(region.tail().seed().get())?;
        let imported = Binding::import(
            &self.descriptor.metadata().plan,
            region,
            &self.descriptor.metadata().schemas,
            &source,
            ctx.library().clone(),
            geometry,
            ctx.limits(),
        );
        match imported {
            Ok((binding, words)) => {
                let caller = self.capture_caller(ctx, index)?;
                self.active_tables = self
                    .active_tables
                    .checked_add(1)
                    .ok_or_else(|| invalid("retained map owner count overflow"))?;
                ctx.stats().hash_table_regions = ctx
                    .stats()
                    .hash_table_regions
                    .checked_add(1)
                    .ok_or_else(|| invalid("retained map table counter overflow"))?;
                self.slots[index].retain(binding);
                self.caller = caller;
                self.last_owned_visit = Some(ctx.control().visits);
                Ok(Some(words))
            }
            // Admission failure can preserve canonical execution. Its spent
            // prefix remains on the same ledger; no allowance is renewed.
            Err(
                StorageError::Memory(_)
                | StorageError::Work(_)
                | StorageError::Limit { .. }
                | StorageError::Allocation,
            ) => {
                self.last_owned_visit = Some(ctx.control().visits);
                Ok(None)
            }
            Err(error) => Err(storage_error(error)),
        }
    }

    pub(in crate::native_artifact::program::runtime) fn call(
        &mut self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
        words: &[u64],
    ) -> Result<u64, Error> {
        self.guard(ctx, site)?;
        let layout = self
            .descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .ok_or_else(|| invalid("retained map call has no descriptor layout"))?;
        ctx.work()
            .charge(Phase::Setup, amount(layout.regions().len(), 128, 128)?)?;
        let index = layout
            .call_region(site)
            .ok_or_else(|| invalid("retained map call is not selected"))?;
        let region = &layout.regions()[index];
        if ctx.control().current != Some(region.iteration()) {
            return Err(invalid("retained map call is outside owning loop"));
        }
        let Action::LibraryCall { operands, .. } =
            self.descriptor.metadata().plan.blocks()[site].action()
        else {
            return Err(invalid("retained map call lacks exact library action"));
        };
        if words.len() != operands.len() || words.len() < 2 || words[1] != 0 {
            return Err(invalid("retained map operand lane differs"));
        }
        ctx.work().charge(
            Phase::Setup,
            amount(
                region
                    .writes()
                    .len()
                    .checked_add(region.reads().len())
                    .ok_or_else(|| invalid("retained map site inventory overflow"))?,
                128,
                128,
            )?,
        )?;
        let write = region.writes().contains(&(site as u32));
        let geometry = self
            .geometry(ctx, site)?
            .ok_or_else(|| invalid("retained map call lost caller geometry"))?;
        let canonical;
        let key = if let Some(span) = layout.call_span(site) {
            let proof = &layout.spans()[span];
            let Some(spans::Lane::Span {
                handle,
                owner,
                bytes,
                ..
            }) = self.spans[span].as_ref()
            else {
                return Err(invalid("retained map key was not projected"));
            };
            if proof.region() != index || proof.output() != operands[1] {
                return Err(invalid("retained map key belongs to another iteration"));
            }
            ctx.arena_get(*handle)?;
            Key::Span {
                value: proof.output(),
                owner,
                bytes,
            }
        } else {
            canonical = ctx.arena_copy_current(operands[1].get())?;
            Key::Canonical(&canonical)
        };
        let binding = self.slots[index]
            .binding
            .as_mut()
            .ok_or_else(|| invalid("retained map call has no active owner"))?;
        let result = if write {
            if words.len() != 3 {
                return Err(invalid("retained map write arity"));
            }
            binding.insert(site, words[0], key, words[2], geometry, ctx.limits())
        } else {
            binding.read(site, words[0], key, geometry, ctx.limits())
        }
        .map_err(storage_error)?;
        if write {
            ctx.stats().hash_table_updates = ctx
                .stats()
                .hash_table_updates
                .checked_add(1)
                .ok_or_else(|| invalid("retained map update counter overflow"))?;
        }
        self.last_owned_visit = Some(ctx.control().visits);
        Ok(result)
    }

    pub(in crate::native_artifact::program::runtime) fn publish_retained(
        &mut self,
        ctx: &mut MapExecution<'_, '_, '_, '_>,
        site: usize,
        token: u64,
    ) -> Result<Publication, Error> {
        self.guard(ctx, site)?;
        let index = self
            .descriptor
            .metadata()
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.exit(site))
            .ok_or_else(|| invalid("retained map publication outside selected exit"))?;
        if ctx.control().current != self.region(index)?.tail().parent() {
            return Err(invalid("retained map publication outside parent scope"));
        }
        let geometry = self
            .geometry(ctx, site)?
            .ok_or_else(|| invalid("retained map publication lost caller geometry"))?;
        let slot = &mut self.slots[index];
        let binding = slot
            .binding
            .as_mut()
            .ok_or_else(|| invalid("retained map publication lacks owner"))?;
        let value = if let Some(frozen) = slot.frozen.as_ref() {
            let proof = self
                .descriptor
                .metadata()
                .frozen_maps
                .exits
                .get(site)
                .copied()
                .flatten()
                .and_then(|id| self.descriptor.metadata().frozen_maps.regions.get(id))
                .ok_or_else(|| invalid("frozen publication lacks complete-use proof"))?;
            if proof.owner != index || frozen.parent != proof.parent || frozen.token != 0 {
                return Err(invalid("frozen publication owner/phase differs"));
            }
            let schema = binding
                .frozen_exit_schema(site, token, &self.descriptor.metadata().schemas)
                .map_err(storage_error)?;
            frozen
                .map
                .admit_output(&self.descriptor.metadata().schemas, schema, ctx.limits())
                .map_err(storage_error)?;
            slot.mark_frozen_published(token);
            ctx.stats().frozen_map_regions = add(ctx.stats().frozen_map_regions, 1)?;
            Publication::Frozen(token)
        } else {
            Publication::Canonical(
                binding
                    .finish(
                        site,
                        token,
                        &self.descriptor.metadata().schemas,
                        geometry,
                        ctx.limits(),
                    )
                    .map_err(storage_error)?,
            )
        };
        self.active_tables = self
            .active_tables
            .checked_sub(1)
            .ok_or_else(|| invalid("retained map owner count underflow"))?;
        drop(self.slots[index].take_binding());
        if self
            .caller
            .as_ref()
            .is_some_and(|caller| caller.iteration == self.region(index).unwrap().iteration())
        {
            self.caller = None;
        }
        let layout = self.descriptor.metadata().map_owner_words.as_ref().unwrap();
        ctx.work()
            .charge(Phase::Setup, amount(layout.spans().len(), 128, 128)?)?;
        for (id, span) in layout.spans().iter().enumerate() {
            if span.region() == index {
                drop(self.spans[id].take());
            }
        }
        self.last_owned_visit = Some(ctx.control().visits);
        Ok(value)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) mod tests;

#[test]
fn map_slot_consolidation_preserves_the_option_backing_layout() {
    assert_eq!(core::mem::size_of::<Slot<'_>>(), 968);
}

#[path = "../../invocation/map_calls.rs"]
mod map_calls;

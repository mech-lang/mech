//! Invocation-bound retained tables for a checked generated caller. Transport
//! selection remains separate; these methods never evaluate a source body.
use super::{Context, NativeProgramExecutionError as Error, amount, invalid};
use crate::native_artifact::{
    NativeExecutionError, NativeExecutionPhase as Phase, NativeProgramDescriptor,
    NativeWorkMeter, owned_map_binding::{Binding, InitialWords},
    owned_string_map::{Error as StorageError, PublicationGeometry},
};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, Value};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMapOwnerRegion};

/// Lazy owning-session lane. No table metadata is allocated merely because a
/// descriptor has a candidate; entry must request and admit it explicitly.
pub(super) struct Session<'plan> {
    descriptor: &'plan NativeProgramDescriptor,
    state: Option<State<'plan>>,
    refused_visit: Option<u64>,
}
impl<'plan> Session<'plan> {
    pub(super) fn new(descriptor: &'plan NativeProgramDescriptor) -> Self {
        Self { descriptor, state: None, refused_visit: None }
    }
}

struct Slot<'plan> {
    binding: Option<Binding<'plan>>,
}
pub(super) struct State<'plan> {
    // Tables/keys retire before the enclosing admitted fixed lane.
    slots: Box<[Slot<'plan>]>,
    descriptor: &'plan NativeProgramDescriptor,
    work_identity: u64,
    last_owned_visit: Option<u64>,
    reservation: ManagedMemoryReservation,
}

fn storage_error(error: StorageError) -> Error {
    match error {
        StorageError::Memory(error) => NativeExecutionError::Memory { error }.into(),
        StorageError::Work(error) => NativeExecutionError::LibraryWork { phase: Phase::Setup, error }.into(),
        StorageError::Binding(reason) | StorageError::Publication(reason) => invalid(reason),
        StorageError::Overflow => invalid("retained map size/work overflow"),
        StorageError::Allocation => invalid("retained map allocation failed"),
        StorageError::Limit { field, .. } => invalid(field),
    }
}
fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or_else(|| invalid("retained map caller geometry overflow"))
}

impl<'plan> State<'plan> {
    #[cfg(test)]
    pub(super) fn new(
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
        let count = descriptor.map_owner_words.as_ref()
            .map_or(0, |layout| layout.regions().len());
        let bytes = amount(count, (2 * core::mem::size_of::<Slot<'plan>>()) as u64,
            (core::mem::size_of::<Self>() + 128) as u64)?;
        if bytes > limit {
            return Ok(None);
        }
        work.charge(Phase::Setup, amount(count, 512, 512)?)?;
        let Ok(reservation) = memory.reserve_capacity(bytes) else { return Ok(None); };
        let mut slots = Vec::new();
        if slots.try_reserve_exact(count).is_err() || slots.capacity() != count {
            return Ok(None);
        }
        slots.resize_with(count, || Slot { binding: None });
        Ok(Some(Self {
            slots: slots.into_boxed_slice(), descriptor,
            work_identity: work.identity(), last_owned_visit: None, reservation,
        }))
    }

    fn guard(&self, ctx: &Context<'_, '_>, site: usize) -> Result<(), Error> {
        let site = u32::try_from(site).map_err(|_| invalid("retained map site overflow"))?;
        if !core::ptr::eq(self.descriptor, ctx.control.descriptor)
            || self.work_identity != ctx.control.work.identity()
            || self.reservation.budget() != ctx.library.memory_budget()
        {
            return Err(invalid("retained map lane belongs to another descriptor/invocation"));
        }
        if ctx.control.previous != Some(site) || ctx.control.completed || ctx.consumed
            || self.last_owned_visit == Some(ctx.control.visits)
        {
            return Err(invalid("retained map operation outside fresh generated visit"));
        }
        Ok(())
    }

    fn region(&self, index: usize) -> Result<&'plan NativeMapOwnerRegion, Error> {
        self.descriptor.map_owner_words.as_ref().and_then(|layout| layout.regions().get(index))
            .ok_or_else(|| invalid("retained map region is outside descriptor"))
    }

    fn geometry(&self, ctx: &mut Context<'_, '_>, site: usize)
        -> Result<Option<PublicationGeometry>, Error>
    {
        let Some(mut geometry) = ctx.retained_caller_geometry(site)? else { return Ok(None); };
        ctx.control.work.charge(Phase::Setup, amount(self.slots.len(), 128, 128)?)?;
        geometry.live_nodes = add(geometry.live_nodes, self.reservation.capacity_bytes())?;
        for slot in &self.slots {
            if let Some(binding) = &slot.binding {
                geometry.live_nodes = add(geometry.live_nodes, binding.retained_nodes().map_err(storage_error)?)?;
            }
        }
        // Every table/key/lane reservation shares this exact account. Snapshot
        // nodes come from the base view; private table nodes are added above.
        geometry.live_bytes = ctx.library.memory_budget().used_bytes();
        Ok(Some(geometry))
    }

    pub(super) fn begin(
        &mut self, ctx: &mut Context<'_, '_>, site: usize,
    ) -> Result<Option<InitialWords>, Error> {
        self.guard(ctx, site)?;
        let index = self.descriptor.map_owner_words.as_ref().and_then(|layout| layout.initializer(site))
            .ok_or_else(|| invalid("retained map entry outside initializer"))?;
        let region = self.region(index)?;
        if ctx.control.current != Some(region.iteration()) || self.slots[index].binding.is_some() {
            return Err(invalid("retained map entry overlaps another live owner"));
        }
        let Some(geometry) = self.geometry(ctx, site)? else {
            self.last_owned_visit = Some(ctx.control.visits);
            return Ok(None);
        };
        let source = ctx.arena.copy_current(region.tail().seed().get(), &mut ctx.control.work)?;
        let imported = Binding::import(&self.descriptor.plan, region, &self.descriptor.schemas,
            &source, ctx.library.clone(), geometry, ctx.limits);
        match imported {
            Ok((binding, words)) => {
                self.slots[index].binding = Some(binding);
                self.last_owned_visit = Some(ctx.control.visits);
                Ok(Some(words))
            }
            // Admission failure can preserve canonical execution. Its spent
            // prefix remains on the same ledger; no allowance is renewed.
            Err(StorageError::Memory(_) | StorageError::Work(_) | StorageError::Limit { .. }
                | StorageError::Allocation) => {
                self.last_owned_visit = Some(ctx.control.visits);
                Ok(None)
            }
            Err(error) => Err(storage_error(error)),
        }
    }

    pub(super) fn call(
        &mut self, ctx: &mut Context<'_, '_>, site: usize, words: &[u64],
    ) -> Result<u64, Error> {
        self.guard(ctx, site)?;
        let layout = self.descriptor.map_owner_words.as_ref()
            .ok_or_else(|| invalid("retained map call has no descriptor layout"))?;
        ctx.control.work.charge(Phase::Setup, amount(layout.regions().len(), 128, 128)?)?;
        let (index, region) = layout.regions().iter().enumerate().find(|(_, region)| {
            region.reads().contains(&(site as u32)) || region.writes().contains(&(site as u32))
        }).ok_or_else(|| invalid("retained map call is not selected"))?;
        if ctx.control.current != Some(region.iteration()) {
            return Err(invalid("retained map call is outside owning loop"));
        }
        let Action::LibraryCall { operands, .. } = self.descriptor.plan.blocks()[site].action() else {
            return Err(invalid("retained map call lacks exact library action"));
        };
        if words.len() != operands.len() || words.len() < 2 || words[1] != 0 {
            return Err(invalid("retained map operand lane differs"));
        }
        let write = region.writes().contains(&(site as u32));
        let key = ctx.arena.copy_current(operands[1].get(), &mut ctx.control.work)?;
        let geometry = self.geometry(ctx, site)?.ok_or_else(|| invalid("retained map call lost caller geometry"))?;
        let binding = self.slots[index].binding.as_mut()
            .ok_or_else(|| invalid("retained map call has no active owner"))?;
        let result = if write {
            if words.len() != 3 { return Err(invalid("retained map write arity")); }
            binding.insert(site, words[0], &key, words[2], geometry, ctx.limits)
        } else {
            binding.read(site, words[0], &key, geometry, ctx.limits)
        }.map_err(storage_error)?;
        self.last_owned_visit = Some(ctx.control.visits);
        Ok(result)
    }

    pub(super) fn finish(
        &mut self, ctx: &mut Context<'_, '_>, site: usize, token: u64,
    ) -> Result<Value, Error> {
        self.guard(ctx, site)?;
        let index = self.descriptor.map_owner_words.as_ref().and_then(|layout| layout.exit(site))
            .ok_or_else(|| invalid("retained map publication outside selected exit"))?;
        if ctx.control.current != self.region(index)?.tail().parent() {
            return Err(invalid("retained map publication outside parent scope"));
        }
        let geometry = self.geometry(ctx, site)?.ok_or_else(|| invalid("retained map publication lost caller geometry"))?;
        let value = self.slots[index].binding.as_mut()
            .ok_or_else(|| invalid("retained map publication lacks owner"))?
            .finish(site, token, &self.descriptor.schemas, geometry, ctx.limits).map_err(storage_error)?;
        drop(self.slots[index].binding.take());
        self.last_owned_visit = Some(ctx.control.visits);
        Ok(value)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_map_owner_runtime_tests.rs"]
mod tests;

impl Context<'_, '_> {
    pub(super) fn begin_retained_map(&mut self, site: usize) -> Result<Option<InitialWords>, Error> {
        // Check authority before lazy allocation, including refused-entry replay.
        if !core::ptr::eq(self.map_owners.descriptor, self.control.descriptor)
            || self.control.previous != u32::try_from(site).ok()
            || self.control.completed || self.consumed
            || self.map_owners.refused_visit == Some(self.control.visits)
            || self.map_owners.descriptor.map_owner_words.as_ref()
                .and_then(|layout| layout.initializer(site)).is_none()
        {
            return Err(invalid("retained map session entry outside fresh initializer"));
        }
        if self.map_owners.state.is_none() {
            let memory = self.library.memory_budget().clone();
            let limit = self.limits.max_temporary_bytes
                .ok_or_else(|| invalid("retained map session requires finite metadata cap"))?;
            let Some(state) = State::try_new(self.map_owners.descriptor,
                &memory, &mut self.control.work, limit)? else {
                self.map_owners.refused_visit = Some(self.control.visits);
                return Ok(None);
            };
            self.map_owners.state = Some(state);
        }
        let mut state = self.map_owners.state.take().unwrap();
        let result = state.begin(self, site);
        // Restore on both success and failure. No new ledger or permit is made.
        self.map_owners.state = Some(state);
        result
    }

    pub(super) fn call_retained_map(&mut self, site: usize, words: &[u64]) -> Result<u64, Error> {
        let mut state = self.map_owners.state.take()
            .ok_or_else(|| invalid("retained map session call before admitted entry"))?;
        let result = state.call(self, site, words);
        self.map_owners.state = Some(state);
        result
    }

    pub(super) fn finish_retained_map(&mut self, site: usize, token: u64) -> Result<Value, Error> {
        let mut state = self.map_owners.state.take()
            .ok_or_else(|| invalid("retained map session publication before admitted entry"))?;
        let result = state.finish(self, site, token);
        self.map_owners.state = Some(state);
        result
    }
}

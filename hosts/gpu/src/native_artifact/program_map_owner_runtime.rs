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
    pub(super) fn new(
        descriptor: &'plan NativeProgramDescriptor,
        memory: &ManagedMemoryBudget,
        work: &mut NativeWorkMeter,
        limit: u64,
    ) -> Result<Self, Error> {
        work.check_memory_budget(memory)?;
        let count = descriptor.map_owner_words.as_ref()
            .map_or(0, |layout| layout.regions().len());
        let bytes = amount(count, (2 * core::mem::size_of::<Slot<'plan>>()) as u64,
            (core::mem::size_of::<Self>() + 128) as u64)?;
        if bytes > limit {
            return Err(invalid("retained map owner lane exceeds its metadata cap"));
        }
        work.charge(Phase::Setup, amount(count, 512, 512)?)?;
        let reservation = memory.reserve_capacity(bytes)?;
        let mut slots = Vec::new();
        slots.try_reserve_exact(count).map_err(|_| invalid("retained map lane allocation"))?;
        if slots.capacity() != count {
            return Err(invalid("retained map lane capacity differs"));
        }
        slots.resize_with(count, || Slot { binding: None });
        Ok(Self {
            slots: slots.into_boxed_slice(), descriptor,
            work_identity: work.identity(), last_owned_visit: None, reservation,
        })
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

//! Invocation-bound retained tables for a checked generated caller. Transport
//! selection remains separate; these methods never evaluate a source body.
use super::{Context, NativeProgramExecutionError as Error, amount, invalid};
use crate::native_artifact::{
    NativeExecutionError, NativeExecutionPhase as Phase, NativeProgramDescriptor, NativeWorkMeter,
    owned_map_binding::{Binding, InitialWords, Key},
    owned_string_map::{Error as StorageError, PublicationGeometry},
};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, ManagedNodeReservation, Value};
use mech_engine::native_artifact::{NativeIterationAction as Action, NativeMapOwnerRegion};

#[path = "program_map_caller_geometry.rs"]
mod caller_geometry;
#[path = "program_map_closed_runtime.rs"]
mod closed;
#[path = "program_frozen_map_runtime.rs"]
pub(in crate::native_artifact::program_runtime) mod frozen;
#[path = "program_map_key_span.rs"]
mod spans;
use frozen::FrozenSlot;

pub(super) enum Publication {
    Canonical(Value),
    Frozen(u64),
}

/// Lazy owning-session lane. No table metadata is allocated merely because a
/// descriptor has a candidate; entry must request and admit it explicitly.
pub(super) struct Session<'plan> {
    descriptor: &'plan NativeProgramDescriptor,
    state: Option<State<'plan>>,
    refused_visit: Option<u64>,
    pub(super) closed_exit: Option<(u32, u32)>,
}
impl<'plan> Session<'plan> {
    pub(super) fn new(descriptor: &'plan NativeProgramDescriptor) -> Self {
        Self {
            descriptor,
            state: None,
            refused_visit: None,
            closed_exit: None,
        }
    }
    pub(super) fn active(&self, index: usize) -> bool {
        self.state
            .as_ref()
            .and_then(|state| state.slots.get(index))
            .is_some_and(|slot| slot.binding.is_some())
    }
}

struct Slot<'plan> {
    binding: Option<Binding<'plan>>,
    closed: Option<Box<closed::Owner<'plan>>>,
    frozen: Option<FrozenSlot>,
}
pub(super) struct State<'plan> {
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
        let count = descriptor
            .map_owner_words
            .as_ref()
            .map_or(0, |layout| layout.regions().len());
        let span_count = descriptor
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
        slots.resize_with(count, || Slot {
            binding: None,
            closed: None,
            frozen: None,
        });
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

    fn guard(&self, ctx: &Context<'_, '_>, site: usize) -> Result<(), Error> {
        let site = u32::try_from(site).map_err(|_| invalid("retained map site overflow"))?;
        if !core::ptr::eq(self.descriptor, ctx.control.descriptor)
            || self.work_identity != ctx.control.work.identity()
            || self.reservation.budget() != ctx.library.memory_budget()
        {
            return Err(invalid(
                "retained map lane belongs to another descriptor/invocation",
            ));
        }
        if ctx.control.previous != Some(site)
            || ctx.control.completed
            || ctx.consumed
            || self.last_owned_visit == Some(ctx.control.visits)
        {
            return Err(invalid(
                "retained map operation outside fresh generated visit",
            ));
        }
        Ok(())
    }

    fn region(&self, index: usize) -> Result<&'plan NativeMapOwnerRegion, Error> {
        self.descriptor
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.regions().get(index))
            .ok_or_else(|| invalid("retained map region is outside descriptor"))
    }

    fn geometry(
        &self,
        ctx: &mut Context<'_, '_>,
        site: usize,
    ) -> Result<Option<PublicationGeometry>, Error> {
        let Some(mut geometry) = self.caller_geometry(ctx, site)? else {
            return Ok(None);
        };
        ctx.control.work.charge(
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
        geometry.live_bytes = ctx.library.memory_budget().used_bytes();
        Ok(Some(geometry))
    }

    pub(super) fn begin(
        &mut self,
        ctx: &mut Context<'_, '_>,
        site: usize,
    ) -> Result<Option<InitialWords>, Error> {
        self.guard(ctx, site)?;
        let index = self
            .descriptor
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.initializer(site))
            .ok_or_else(|| invalid("retained map entry outside initializer"))?;
        let region = self.region(index)?;
        if ctx.control.current != Some(region.iteration())
            || self.slots[index].binding.is_some()
            || self.slots[index].frozen.is_some()
        {
            return Err(invalid("retained map entry overlaps another live owner"));
        }
        let Some(geometry) = self.geometry(ctx, site)? else {
            self.last_owned_visit = Some(ctx.control.visits);
            return Ok(None);
        };
        let source = ctx
            .arena
            .copy_current(region.tail().seed().get(), &mut ctx.control.work)?;
        let imported = Binding::import(
            &self.descriptor.plan,
            region,
            &self.descriptor.schemas,
            &source,
            ctx.library.clone(),
            geometry,
            ctx.limits,
        );
        match imported {
            Ok((binding, words)) => {
                let caller = self.capture_caller(ctx, index)?;
                self.active_tables = self
                    .active_tables
                    .checked_add(1)
                    .ok_or_else(|| invalid("retained map owner count overflow"))?;
                ctx.hash_table_regions = ctx
                    .hash_table_regions
                    .checked_add(1)
                    .ok_or_else(|| invalid("retained map table counter overflow"))?;
                self.slots[index].binding = Some(binding);
                self.caller = caller;
                self.last_owned_visit = Some(ctx.control.visits);
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
                self.last_owned_visit = Some(ctx.control.visits);
                Ok(None)
            }
            Err(error) => Err(storage_error(error)),
        }
    }

    pub(super) fn call(
        &mut self,
        ctx: &mut Context<'_, '_>,
        site: usize,
        words: &[u64],
    ) -> Result<u64, Error> {
        self.guard(ctx, site)?;
        let layout = self
            .descriptor
            .map_owner_words
            .as_ref()
            .ok_or_else(|| invalid("retained map call has no descriptor layout"))?;
        ctx.control
            .work
            .charge(Phase::Setup, amount(layout.regions().len(), 128, 128)?)?;
        let index = layout
            .call_region(site)
            .ok_or_else(|| invalid("retained map call is not selected"))?;
        let region = &layout.regions()[index];
        if ctx.control.current != Some(region.iteration()) {
            return Err(invalid("retained map call is outside owning loop"));
        }
        let Action::LibraryCall { operands, .. } = self.descriptor.plan.blocks()[site].action()
        else {
            return Err(invalid("retained map call lacks exact library action"));
        };
        if words.len() != operands.len() || words.len() < 2 || words[1] != 0 {
            return Err(invalid("retained map operand lane differs"));
        }
        ctx.control.work.charge(
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
            ctx.arena.get(*handle, &mut ctx.control.work)?;
            Key::Span {
                value: proof.output(),
                owner,
                bytes,
            }
        } else {
            canonical = ctx
                .arena
                .copy_current(operands[1].get(), &mut ctx.control.work)?;
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
            binding.insert(site, words[0], key, words[2], geometry, ctx.limits)
        } else {
            binding.read(site, words[0], key, geometry, ctx.limits)
        }
        .map_err(storage_error)?;
        if write {
            ctx.hash_table_updates = ctx
                .hash_table_updates
                .checked_add(1)
                .ok_or_else(|| invalid("retained map update counter overflow"))?;
        }
        self.last_owned_visit = Some(ctx.control.visits);
        Ok(result)
    }

    pub(super) fn finish(
        &mut self,
        ctx: &mut Context<'_, '_>,
        site: usize,
        token: u64,
    ) -> Result<Publication, Error> {
        self.guard(ctx, site)?;
        let index = self
            .descriptor
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.exit(site))
            .ok_or_else(|| invalid("retained map publication outside selected exit"))?;
        if ctx.control.current != self.region(index)?.tail().parent() {
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
                .frozen_maps
                .exits
                .get(site)
                .copied()
                .flatten()
                .and_then(|id| self.descriptor.frozen_maps.regions.get(id))
                .ok_or_else(|| invalid("frozen publication lacks complete-use proof"))?;
            if proof.owner != index || frozen.parent != proof.parent || frozen.token != 0 {
                return Err(invalid("frozen publication owner/phase differs"));
            }
            let schema = binding
                .frozen_exit_schema(site, token, &self.descriptor.schemas)
                .map_err(storage_error)?;
            frozen
                .map
                .admit_output(&self.descriptor.schemas, schema, ctx.limits)
                .map_err(storage_error)?;
            slot.frozen.as_mut().unwrap().token = token;
            ctx.frozen_map_regions = add(ctx.frozen_map_regions, 1)?;
            Publication::Frozen(token)
        } else {
            Publication::Canonical(
                binding
                    .finish(site, token, &self.descriptor.schemas, geometry, ctx.limits)
                    .map_err(storage_error)?,
            )
        };
        self.active_tables = self
            .active_tables
            .checked_sub(1)
            .ok_or_else(|| invalid("retained map owner count underflow"))?;
        drop(self.slots[index].binding.take());
        if self
            .caller
            .as_ref()
            .is_some_and(|caller| caller.iteration == self.region(index).unwrap().iteration())
        {
            self.caller = None;
        }
        let layout = self.descriptor.map_owner_words.as_ref().unwrap();
        ctx.control
            .work
            .charge(Phase::Setup, amount(layout.spans().len(), 128, 128)?)?;
        for (id, span) in layout.spans().iter().enumerate() {
            if span.region() == index {
                drop(self.spans[id].take());
            }
        }
        self.last_owned_visit = Some(ctx.control.visits);
        Ok(value)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_map_owner_runtime_tests.rs"]
pub(in crate::native_artifact::program_runtime) mod tests;

impl Context<'_, '_> {
    /// Retire private views at the SAME Tail latch/exit boundary as ordinary
    /// immutable locals. A later iteration can never read an earlier key.
    pub(super) fn retire_map_keys(&mut self, iteration: u32) -> Result<(), Error> {
        let Some(state) = self.map_owners.state.as_mut() else {
            return Ok(());
        };
        let layout = state.descriptor.map_owner_words.as_ref().unwrap();
        self.control
            .work
            .charge(Phase::Setup, amount(layout.spans().len(), 64, 64)?)?;
        for (id, proof) in layout.spans().iter().enumerate() {
            if layout.regions()[proof.region()].iteration() == iteration {
                drop(state.spans[id].take());
            }
        }
        Ok(())
    }
    pub(super) fn retained_native_span(&self, site: usize) -> bool {
        self.control
            .descriptor
            .map_owner_words
            .as_ref()
            .is_some_and(|layout| {
                layout
                    .span_site(site)
                    .is_some_and(|id| self.map_owners.active(layout.spans()[id].region()))
            })
    }
    pub(super) fn begin_retained_map(
        &mut self,
        site: usize,
    ) -> Result<Option<InitialWords>, Error> {
        // Check authority before lazy allocation, including refused-entry replay.
        if !core::ptr::eq(self.map_owners.descriptor, self.control.descriptor)
            || self.control.previous != u32::try_from(site).ok()
            || self.control.completed
            || self.consumed
            || self.map_owners.refused_visit == Some(self.control.visits)
            || self
                .map_owners
                .descriptor
                .map_owner_words
                .as_ref()
                .and_then(|layout| layout.initializer(site))
                .is_none()
        {
            return Err(invalid(
                "retained map session entry outside fresh initializer",
            ));
        }
        if self.map_owners.state.is_none() {
            let limit = self
                .limits
                .max_temporary_bytes
                .ok_or_else(|| invalid("retained map session requires finite metadata cap"))?;
            let memory = self
                .library
                .memory_budget()
                .with_limits(limit, self.limits.max_retained_nodes.unwrap_or(0));
            let Some(state) = State::try_new(
                self.map_owners.descriptor,
                &memory,
                &mut self.control.work,
                limit,
            )?
            else {
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
        let mut state = self
            .map_owners
            .state
            .take()
            .ok_or_else(|| invalid("retained map session call before admitted entry"))?;
        let result = state.call(self, site, words);
        self.map_owners.state = Some(state);
        result
    }

    pub(super) fn finish_retained_map(
        &mut self,
        site: usize,
        token: u64,
    ) -> Result<Publication, Error> {
        let mut state = self
            .map_owners
            .state
            .take()
            .ok_or_else(|| invalid("retained map session publication before admitted entry"))?;
        let result = state.finish(self, site, token);
        self.map_owners.state = Some(state);
        result
    }
}

impl Context<'_, '_> {
    pub(super) fn retained_native_action(&self, site: usize) -> bool {
        self.control
            .descriptor
            .map_owner_words
            .as_ref()
            .and_then(|layout| {
                crate::native_artifact::program_map_owner_layout::action_region(layout, site)
            })
            .is_some_and(|index| self.map_owners.active(index))
    }
    pub(super) fn retained_native_exit(&self, site: usize) -> bool {
        self.control
            .descriptor
            .map_owner_words
            .as_ref()
            .and_then(|layout| layout.exit(site))
            .is_some_and(|index| self.map_owners.active(index))
    }
    pub(super) fn has_retained_maps(&self) -> bool {
        self.map_owners
            .state
            .as_ref()
            .is_some_and(|state| state.active_tables != 0)
    }
    pub(super) fn retained_native_call(&self, site: usize) -> bool {
        self.control
            .descriptor
            .map_owner_words
            .as_ref()
            .and_then(|layout| {
                crate::native_artifact::program_map_owner_layout::call_region(layout, site)
            })
            .is_some_and(|index| self.map_owners.active(index))
    }
    pub(super) fn retained_map_helper(
        &mut self,
        site: usize,
        operation: u64,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        use crate::native_artifact::program_abi::{
            MAP_OWNER_BEGIN, MAP_OWNER_CALL, MAP_OWNER_END, MAP_OWNER_SPAN, MOVE,
        };
        if self.consumed {
            return Err(invalid("duplicate retained map helper"));
        }
        if operation == MAP_OWNER_BEGIN {
            if args != [0; 3] {
                return Err(invalid("malformed retained map entry"));
            }
            if let Some(words) = self.begin_retained_map(site)? {
                let layout = self.control.descriptor.map_owner_words.as_ref().unwrap();
                let index = layout.initializer(site).unwrap();
                let count = layout
                    .aggregates()
                    .slot(layout.regions()[index].tail().carrier())
                    .unwrap()
                    .1;
                if words.words().len() != count || count > self.argument_capacity {
                    return Err(invalid("retained map initializer word lane differs"));
                }
                self.control
                    .work
                    .charge(Phase::Setup, amount(count, 16, 128)?)?;
                for (index, &word) in words.words().iter().enumerate() {
                    self.write_borrowed_word(index, word)?;
                }
                self.consumed = true;
                Ok(Some(1))
            } else {
                self.helper(site as u64, MOVE, [0; 3])?;
                Ok(Some(0))
            }
        } else if operation == MAP_OWNER_SPAN {
            if !self.retained_native_span(site) {
                return Err(invalid("map key projection lacks active owner"));
            }
            let mut state = self.map_owners.state.take().unwrap();
            let result = state.project_key(self, site, args);
            self.map_owners.state = Some(state);
            result?;
            self.consumed = true;
            Ok(None)
        } else if operation == MAP_OWNER_CALL {
            let Action::LibraryCall { operands, .. } =
                self.control.descriptor.plan.blocks()[site].action()
            else {
                return Err(invalid("retained map helper outside library call"));
            };
            let count = operands.len();
            if args != [self.arguments as u64, count as u64, 0]
                || count > 3
                || count > self.argument_capacity
                || !self.retained_native_call(site)
            {
                return Err(invalid("retained map helper operand lane differs"));
            }
            let mut words = [0; 3];
            // SAFETY: exact admitted pointer and capacity checked above; native
            // execution is suspended and no borrowed lane escapes this copy.
            words[..count]
                .copy_from_slice(unsafe { core::slice::from_raw_parts(self.arguments, count) });
            let result = self.call_retained_map(site, &words[..count])?;
            self.consumed = true;
            Ok(Some(result))
        } else if operation == MAP_OWNER_END {
            if args[1..] != [0; 2] {
                return Err(invalid("malformed retained map publication"));
            }
            let Action::Scalar(mech_engine::native_artifact::NativeMatchAction::Copy {
                output,
                ..
            }) = self.control.descriptor.plan.blocks()[site].action()
            else {
                return Err(invalid("retained map exit lacks canonical destination"));
            };
            let output = *output;
            let token = match self.finish_retained_map(site, args[0])? {
                Publication::Canonical(value) => {
                    self.arena
                        .put(output.get(), &value, &mut self.control.work)?;
                    0
                }
                Publication::Frozen(token) => token,
            };
            self.consumed = true;
            Ok(Some(token))
        } else {
            Err(invalid("unknown retained map helper"))
        }
    }
}

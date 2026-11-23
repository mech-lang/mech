//! Virtual String matrices own their immutable key snapshot, independently of
//! both the original arena handle and the retained insertion owner's lifetime.
use super::*;
use mech_engine::native_artifact::{NativeBorrowedMapKeys, NativeBorrowedMapKeysChoice};

impl Session<'_> {
    pub(in crate::native_artifact::program::runtime) fn borrowed_keys(
        &self,
        source: u32,
    ) -> Option<&NativeBorrowedMapKeys> {
        let state = self.state.as_ref()?;
        state
            .descriptor
            .metadata()
            .matrix_map_keys
            .regions
            .iter()
            .enumerate()
            .find_map(|(id, p)| {
                (p.source.get() == source)
                    .then(|| state.matrix_keys[id].view.as_ref())
                    .flatten()
            })
    }
    pub(in crate::native_artifact::program::runtime) fn retire_borrowed_keys(
        &mut self,
        values: &[u64],
        work: &mut NativeWorkMeter,
    ) -> Result<(), Error> {
        let Some(state) = self.state.as_mut() else {
            return Ok(());
        };
        if state
            .descriptor
            .metadata()
            .borrowed_map_keys
            .regions
            .is_empty()
        {
            return Ok(());
        }
        work.charge(
            Phase::Setup,
            amount(values.len(), 16, amount(state.matrix_keys.len(), 64, 0)?)?,
        )?;
        for (id, p) in state
            .descriptor
            .metadata()
            .matrix_map_keys
            .regions
            .iter()
            .enumerate()
        {
            if values.contains(&(p.source.get() as u64)) && state.matrix_keys[id].view.is_some() {
                if state.matrix_keys[id].pending.is_some() {
                    return Err(invalid("retiring key view before its selected insertion"));
                }
                state.matrix_keys[id].view = None;
                state.matrix_keys[id].view_handle = None;
                state.caller = None;
            }
        }
        Ok(())
    }
}
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn borrowed_map_keys_call(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        use crate::native_artifact::program::protocol::GlobalHelper;
        if self.state.consumed
            || args != [self.arguments as u64, 1, 0]
            || self.argument_capacity < 1
        {
            return Err(invalid("borrowed map keys outside exact producer/lane"));
        }
        // SAFETY: exact invocation-owned input lane, suspended generated code.
        if unsafe { *self.arguments } != 0 {
            return Err(invalid("borrowed map keys has scalar source"));
        }
        let metadata = self.descriptor.metadata();
        let proof = metadata
            .borrowed_map_keys
            .producer(site)
            .ok_or_else(|| invalid("borrowed map keys lost complete uses"))?;
        self.state.control.work.charge(
            Phase::Setup,
            amount(metadata.borrowed_map_keys.regions.len(), 64, 128)?,
        )?;
        if self.has_uninventoried_private_owners() {
            return self.helper(site as u64, GlobalHelper::Library.raw(), args);
        }
        if self.state.map_owners.state.is_none() {
            let limit = self
                .limits
                .max_temporary_bytes
                .ok_or_else(|| invalid("key view requires finite metadata cap"))?;
            let memory = self
                .state
                .library
                .memory_budget()
                .with_limits(limit, self.limits.max_retained_nodes.unwrap_or(0));
            let Some(state) = State::try_new(
                self.descriptor,
                &memory,
                &mut self.state.control.work,
                limit,
            )?
            else {
                return self.helper(site as u64, GlobalHelper::Library.raw(), args);
            };
            self.state.map_owners.state = Some(state);
        }
        let (id, _) = metadata
            .matrix_map_keys
            .producer(proof.cells[0])
            .ok_or_else(|| invalid("key view lost cell origin"))?;
        {
            let state = self.state.map_owners.state.as_mut().unwrap();
            if state.matrix_keys[id].pending.is_some() {
                return Err(invalid("redefining keys with an outstanding insertion"));
            }
            state.matrix_keys[id].clear();
            state.matrix_keys[id].view = None;
            state.matrix_keys[id].view_handle = None;
            state.caller = None;
        }
        let source = self
            .state
            .arena
            .copy_current(proof.snapshot.get(), &mut self.state.control.work)?;
        let coexistence = self.selected_join_coexistence(None)?;
        let call = metadata.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("key view lost selected factory"))?;
        let element = metadata.calls[proof.cells[0]]
            .as_ref()
            .ok_or_else(|| invalid("key view lost element factory"))?;
        let choice = call
            .borrowed_map_keys_or_canonical(
                element,
                &source,
                self.limits,
                &self.state.library,
                coexistence,
            )
            .map_err(|error| Error::Library {
                origin: metadata.plan.blocks()[site].origin(),
                error,
            })?;
        match choice {
            NativeBorrowedMapKeysChoice::View(view) => {
                // A new virtual definition shadows any older canonical home.
                let handle = self.state.arena.define_virtual(
                    proof.output.get(),
                    view.schema(),
                    view.schema_key(),
                    &mut self.state.control.work,
                )?;
                let lane = &mut self.state.map_owners.state.as_mut().unwrap().matrix_keys[id];
                lane.view_handle = Some(handle);
                lane.view = Some(view);
            }
            NativeBorrowedMapKeysChoice::Canonical(value) => {
                self.state
                    .arena
                    .put(proof.output.get(), &value, &mut self.state.control.work)?;
            }
        }
        self.state.consumed = true;
        Ok(None)
    }

    pub(in crate::native_artifact::program::runtime) fn materialize_borrowed_keys(
        &mut self,
        source: u32,
    ) -> Result<(), Error> {
        let metadata = self.descriptor.metadata();
        let Some(proof) = metadata
            .borrowed_map_keys
            .regions
            .iter()
            .find(|p| p.output.get() == source)
        else {
            return Ok(());
        };
        let (id, _) = metadata
            .matrix_map_keys
            .producer(proof.cells[0])
            .ok_or_else(|| invalid("key materialization lost producer"))?;
        self.check_borrowed_keys_current(source)?;
        let view = match self.state.map_owners.state.as_mut() {
            Some(state) => {
                if state.matrix_keys[id].pending.is_some() {
                    return Err(invalid("materializing keys during pending insertion"));
                }
                state.caller = None;
                state.matrix_keys[id].view_handle = None;
                state.matrix_keys[id].view.take()
            }
            None => None,
        };
        let Some(view) = view else { return Ok(()) };
        let call = metadata.calls[proof.producer]
            .as_ref()
            .ok_or_else(|| invalid("key materialization lost selected source factory"))?;
        let value = call
            .materialize_borrowed_map_keys(view, &self.state.library)
            .map_err(|error| Error::Library {
                origin: metadata.plan.blocks()[proof.producer].origin(),
                error,
            })?;
        self.state
            .arena
            .put(source, &value, &mut self.state.control.work)?;
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime) fn check_borrowed_keys_current(
        &mut self,
        source: u32,
    ) -> Result<(), Error> {
        if let Some(state) = self.state.map_owners.state.as_ref() {
            if let Some((id, _)) = state
                .descriptor
                .metadata()
                .matrix_map_keys
                .regions
                .iter()
                .enumerate()
                .find(|(_, p)| p.source.get() == source)
            {
                if state.matrix_keys[id].view.is_some() {
                    let handle = state.matrix_keys[id]
                        .view_handle
                        .ok_or_else(|| invalid("key view has no logical generation"))?;
                    self.state
                        .arena
                        .check_virtual(handle, &mut self.state.control.work)?;
                }
            }
        }
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime) fn borrowed_keys_length_call(
        &mut self,
        site: usize,
        words: &[u64],
    ) -> Result<Option<u64>, Error> {
        let metadata = self.descriptor.metadata();
        let proof = metadata
            .borrowed_map_keys
            .length(site)
            .ok_or_else(|| invalid("key length lost complete use"))?;
        if words != [0] {
            return Err(invalid("borrowed key length has scalar input"));
        }
        if self
            .state
            .map_owners
            .borrowed_keys(proof.output.get())
            .is_none()
        {
            return self.matrix_length_call(site, words);
        }
        if self.has_uninventoried_private_owners() {
            self.materialize_borrowed_keys(proof.output.get())?;
            return self.matrix_length_call(site, words);
        }
        self.check_borrowed_keys_current(proof.output.get())?;
        let coexistence = self.selected_join_coexistence(None)?;
        let view = self
            .state
            .map_owners
            .borrowed_keys(proof.output.get())
            .ok_or_else(|| invalid("key length lost current view"))?;
        let call = metadata.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("key length lost selected factory"))?;
        call.borrowed_map_keys_length(view, self.limits, &self.state.library, coexistence)
            .map(Some)
            .map_err(|error| Error::Library {
                origin: metadata.plan.blocks()[site].origin(),
                error,
            })
    }
}

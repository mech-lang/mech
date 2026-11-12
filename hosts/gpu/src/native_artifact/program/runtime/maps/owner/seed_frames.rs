//! One original factory's owned aggregate, suspended only to its adjacent import.
use super::*;
use crate::native_artifact::NativeValueHandle;
use mech_engine::native_artifact::{NativeMapSeedChoice, NativeMapSeedFrame};

pub(in crate::native_artifact::program::runtime) struct Seed {
    pub frame: NativeMapSeedFrame,
    pub handle: NativeValueHandle,
    pub value: u32,
    pub owner: usize,
    pub producer: usize,
    pub initialize: usize,
    pub born: u64,
}
impl Session<'_> {
    pub(in crate::native_artifact::program::runtime) fn seed_nodes(
        &self,
        work: &mut NativeWorkMeter,
    ) -> Result<u64, Error> {
        let Some(state) = self.state.as_ref() else {
            return Ok(0);
        };
        if state.seeds.is_empty() {
            return Ok(0);
        }
        work.charge(Phase::Setup, amount(state.seeds.len(), 64, 64)?)?;
        let mut nodes = amount(
            state.seeds.len(),
            (2 * core::mem::size_of::<Option<Seed>>()) as u64,
            0,
        )?;
        for seed in state.seeds.iter().flatten() {
            nodes = add(nodes, seed.frame.metadata_demand().retained_nodes)?;
        }
        Ok(nodes)
    }
}
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn seed_frame_call(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        use crate::native_artifact::program::protocol::GlobalHelper;
        let metadata = self.descriptor.metadata();
        let proof = metadata
            .dictionary_seed_frames
            .producer(site)
            .ok_or_else(|| invalid("native seed outside complete-use proof"))?;
        let arity = proof.operands.len();
        if self.state.consumed
            || args != [self.arguments as u64, arity as u64, 0]
            || self.argument_capacity < arity
        {
            return Err(invalid("native seed outside exact original call/lane"));
        }
        if self.worker.is_some() || self.has_uninventoried_private_owners() {
            return self.helper(site as u64, GlobalHelper::Library.raw(), args);
        }
        self.state.control.work.charge(
            Phase::Setup,
            amount(metadata.dictionary_seed_frames.regions.len(), 64, 128)?,
        )?;
        if self.state.map_owners.state.is_none() {
            let limit = self
                .limits
                .max_temporary_bytes
                .ok_or_else(|| invalid("native seed requires finite metadata"))?;
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
        // SAFETY: exact invocation-owned pointer/count checked before reading;
        // generated execution is suspended for this synchronous helper.
        let words = unsafe { core::slice::from_raw_parts(self.arguments, arity) };
        for (index, &operand) in proof.operands.iter().enumerate() {
            let value = if metadata.plan.values()[operand.get() as usize]
                .scalar_kind()
                .is_some()
            {
                self.scalar(operand, words[index])?
            } else {
                if words[index] != 0 {
                    return Err(invalid("native seed managed input has bits"));
                }
                self.state
                    .arena
                    .copy_current(operand.get(), &mut self.state.control.work)?
            };
            self.state.call_inputs.values_mut()[index] = Some(value);
        }
        // While taken, the Map state inventories ALL its private tables,
        // views and metadata itself. Shared snapshot nodes remain distinct.
        let state = self.state.map_owners.state.take().unwrap();
        let geometry = state.geometry(&mut MapExecution::new(self), site);
        self.state.map_owners.state = Some(state);
        let Some(geometry) = geometry? else {
            self.state.call_inputs.values_mut()[..arity].fill(None);
            return self.helper(site as u64, GlobalHelper::Library.raw(), args);
        };
        let call = metadata.calls[site]
            .as_ref()
            .ok_or_else(|| invalid("native seed lost original factory"))?;
        let choice = call.map_seed_or_canonical(
            &self.state.call_inputs.values()[..arity],
            self.limits,
            &self.state.library,
            mech_core::ResourceDemand {
                turn_peak_bytes: geometry.live_bytes,
                retained_nodes: geometry.live_nodes,
                ..Default::default()
            },
        );
        self.state.call_inputs.values_mut()[..arity].fill(None);
        let choice = choice.map_err(|error| Error::Library {
            origin: metadata.plan.blocks()[site].origin(),
            error,
        })?;
        match choice {
            NativeMapSeedChoice::Canonical(value) => {
                self.state
                    .arena
                    .put(proof.seed.get(), &value, &mut self.state.control.work)?;
            }
            NativeMapSeedChoice::Frame(frame) => {
                let ty = metadata.types[proof.seed.get() as usize];
                if frame.schema() != ty.schema || frame.schema_key() != ty.key {
                    return Err(invalid("native seed type differs"));
                }
                let handle = self.state.arena.define_virtual(
                    proof.seed.get(),
                    ty.schema,
                    ty.key,
                    &mut self.state.control.work,
                )?;
                let id = metadata
                    .dictionary_seed_frames
                    .regions
                    .iter()
                    .position(|r| r.owner == proof.owner)
                    .ok_or_else(|| invalid("native seed lost bank"))?;
                let state = self.state.map_owners.state.as_mut().unwrap();
                if state.seeds[id].is_some() {
                    return Err(invalid("redefining unconsumed native seed"));
                }
                state.seeds[id] = Some(Seed {
                    frame,
                    handle,
                    value: proof.seed.get(),
                    owner: proof.owner,
                    producer: site,
                    initialize: proof.initialize as usize,
                    born: self.state.control.visits,
                });
            }
        }
        self.state.consumed = true;
        Ok(None)
    }
    pub(in crate::native_artifact::program::runtime) fn take_seed(
        &mut self,
        site: usize,
    ) -> Result<Option<Seed>, Error> {
        let metadata = self.descriptor.metadata();
        let Some(proof) = metadata.dictionary_seed_frames.initializer(site) else {
            return Ok(None);
        };
        let Some(state) = self.state.map_owners.state.as_mut() else {
            return Ok(None);
        };
        let id = metadata
            .dictionary_seed_frames
            .regions
            .iter()
            .position(|r| r.owner == proof.owner)
            .ok_or_else(|| invalid("native seed lost owner bank"))?;
        let Some(seed) = state.seeds[id].take() else {
            return Ok(None);
        };
        if seed.value != proof.seed.get()
            || seed.owner != proof.owner
            || seed.producer != proof.producer as usize
            || seed.initialize != site
            || seed.born >= self.state.control.visits
        {
            return Err(invalid("stale or foreign native seed source"));
        }
        self.state
            .arena
            .check_virtual(seed.handle, &mut self.state.control.work)?;
        seed.frame
            .check_account(&self.state.library)
            .map_err(|error| Error::Library {
                origin: metadata.plan.blocks()[seed.producer].origin(),
                error,
            })?;
        Ok(Some(seed))
    }
    pub(in crate::native_artifact::program::runtime) fn materialize_seed(
        &mut self,
        seed: Seed,
    ) -> Result<(), Error> {
        self.state
            .arena
            .check_virtual(seed.handle, &mut self.state.control.work)?;
        let metadata = self.descriptor.metadata();
        let call = metadata.calls[seed.producer]
            .as_ref()
            .ok_or_else(|| invalid("seed fallback lost original factory"))?;
        let value = call
            .materialize_map_seed(seed.frame, &self.state.library)
            .map_err(|error| Error::Library {
                origin: metadata.plan.blocks()[seed.producer].origin(),
                error,
            })?;
        self.state
            .arena
            .put(seed.value, &value, &mut self.state.control.work)?;
        Ok(())
    }
}

#[cfg(test)]
impl Session<'_> {
    pub(in crate::native_artifact::program::runtime) fn has_seed(&self, value: u32) -> bool {
        self.state
            .as_ref()
            .is_some_and(|s| s.seeds.iter().flatten().any(|seed| seed.value == value))
    }
}

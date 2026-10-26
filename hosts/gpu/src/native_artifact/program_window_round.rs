//! Short-lived owner loans for a compiler-proved loop INSIDE one host round.
//! No loop evaluation, provider access, renewed budget or transaction here.
use super::{Context, NativeProgramExecutionError as Error, invalid};
use mech_engine::native_artifact::NativeControlLoop;
#[path = "program_window_hashes.rs"]
pub(super) mod hashes;
#[path = "program_string_map_round.rs"]
mod string_map;

impl Context<'_, '_> {
    pub(super) fn prepare_window_region(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        if self.window_hashes.is_some() || self.window_region.is_some() || self.consumed {
            return Err(invalid("duplicate compiled window preparation"));
        }
        let descriptor = self.control.descriptor;
        // Pay both immutable region-table lookups (prepare and finish) once;
        // their cost must also remain bounded for programs with many regions.
        self.control.work.charge(
            super::NativeExecutionPhase::Setup,
            super::amount(descriptor.metadata().window_regions.len(), 128, 256)?,
        )?;
        let region = descriptor.metadata().window_regions
            .iter()
            .find(|region| {
                u64::from(region.collection()) == args[2]
                    && matches!(&descriptor.metadata().plan.iterations()[region.iteration() as usize],
                    NativeControlLoop::Collection(generator) if generator.header() as usize == site)
            })
            .ok_or_else(|| invalid("compiled window outside its exact immutable plan"))?;
        if args[0] > hashes::MAX_WIDTH {
            return Ok(Some(0));
        }
        let iteration = region.iteration() as usize;
        if self.control.current != Some(region.iteration())
            || self.control.progress[iteration * 2 + 1] != 1
            || args[1].checked_add(1) != Some(self.control.progress[iteration * 2])
        {
            return Err(invalid("compiled window outside its fresh source count"));
        }
        let mut state = self
            .collections
            .take()
            .ok_or_else(|| invalid("compiled window has no collection state"))?;
        let result = if region.query().is_some() {
            state.prepare_compiled_window(self, region, args[0], args[1])
        } else {
            state.prepare_compiled_histogram(self, region, args[0], args[1])
        };
        self.collections = Some(state);
        let Some((_words, accepted)) = result? else {
            // Same compiler/CFG handles representations not covered by this
            // machine-code variant. No resident or historical executor.
            return Ok(Some(0));
        };
        let source = self.call_inputs.values()[0]
            .as_ref()
            .ok_or_else(|| invalid("hash window lost its admitted source"))?;
        let mech_core::ValueData::String(text) = source.data() else {
            return Err(invalid("hash window source is not String"));
        };
        let memory = self.library.memory_budget();
        // Existing preparation includes all current source/query/scratch owners.
        // Its shared-account observation is deliberately conservative for the
        // component allowance and cannot grant additional global capacity.
        let temporary = self
            .limits
            .max_temporary_bytes
            .ok_or_else(|| invalid("hash window requires finite temporary bytes"))?
            .saturating_sub(memory.used_bytes());
        let retained = self
            .limits
            .max_retained_nodes
            .ok_or_else(|| invalid("hash window requires finite retained nodes"))?
            .saturating_sub(memory.retained_nodes());
        let hashes = hashes::WindowHashes::new(
            text.len(),
            args[0],
            accepted,
            region.collection(),
            descriptor.metadata().plan.collections()[region.collection() as usize].origin(),
            self.library.clone(),
            temporary,
            retained,
        )?;
        self.window_hashes = Some(Box::new(hashes));
        self.window_region = Some((region.collection(), accepted));
        let owner = self.window_hashes.as_mut().unwrap().as_mut() as *mut hashes::WindowHashes;
        self.write_window_arguments([
            text.as_ptr() as u64,
            text.len() as u64,
            owner as u64,
            hashes::update_batch as *const () as u64,
            args[0],
        ])?;
        Ok(Some(1))
    }

    pub(super) fn finish_window_region(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let (collection, accepted) = self
            .window_region
            .ok_or_else(|| invalid("compiled window finish without loans"))?;
        if args[1] != u64::from(collection) || args[2] != 0 || args[0] > accepted {
            return Err(invalid("compiled window count outside admitted bound"));
        }
        let descriptor = self.control.descriptor;
        let region = descriptor
            .metadata()
            .window_regions
            .iter()
            .find(|region| region.collection() == collection)
            .ok_or_else(|| invalid("compiled window lost its exact plan"))?;
        let NativeControlLoop::Collection(generator) =
            &descriptor.metadata().plan.iterations()[region.iteration() as usize]
        else {
            return Err(invalid("compiled window lost its generator"));
        };
        if generator.header() as usize != site || self.control.current != Some(region.iteration()) {
            return Err(invalid("compiled window finish outside its entry"));
        }
        let hashes = self
            .window_hashes
            .take()
            .ok_or_else(|| invalid("compiled window lost its hash table"))?;
        hashes.validate(collection, accepted, args[0])?;
        self.hash_table_regions = self
            .hash_table_regions
            .checked_add(1)
            .ok_or_else(|| invalid("hash table region witness overflow"))?;
        self.hash_table_updates = self
            .hash_table_updates
            .checked_add(hashes.completed_updates())
            .ok_or_else(|| invalid("hash table update witness overflow"))?;
        let counted = if region.query().is_some() {
            let query = self.call_inputs.values()[1]
                .as_ref()
                .ok_or_else(|| invalid("hash window lost its query"))?;
            let mech_core::ValueData::String(query) = query.data() else {
                return Err(invalid("hash window query is not String"));
            };
            hashes.count(query.as_bytes())?
        } else {
            accepted
        };
        let mut state = self
            .collections
            .take()
            .ok_or_else(|| invalid("compiled window lost its state"))?;
        if region.query().is_none() {
            state.publish_hash_window_bins(collection, &hashes)?;
        }
        // Actual hash-table owner retires before result materialization.
        drop(hashes);
        let result = if region.query().is_some() {
            state.finish_compiled_window(collection, accepted, counted)
        } else {
            state.finish_compiled_histogram(self, collection, accepted, args[0])
        };
        self.collections = Some(state);
        result?;
        // Only this closed, compiler-proved region may exhaust its own finite
        // generator in bulk. The next ordinary CFG edge is the SAME header's
        // exit. Do not replay/fabricate a billion old operation visits.
        self.control.progress[region.iteration() as usize * 2] = 0;
        self.call_inputs.values_mut()[..2].fill(None);
        self.window_region = None;
        Ok(None)
    }
}

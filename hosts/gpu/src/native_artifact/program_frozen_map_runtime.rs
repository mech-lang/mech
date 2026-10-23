//! Invocation-local immutable owners. Tokens never enter canonical Value data
//! and cannot escape the proved original immutable parent visit.
use super::{Context, Error, State, add, amount, invalid, storage_error};
use crate::native_artifact::{NativeExecutionPhase as Phase,
    program_abi::{FROZEN_MAP_COPY, FROZEN_MAP_READ}, program_packed_map::FrozenPackedMap};
use mech_core::{Value, ValueData, ResidentKernelError, snapshot::canonical_data_retained_footprint};
use mech_engine::native_artifact::{NativeCollectionInvocationError,
    NativeCollectionPrimitive as Primitive, NativeIterationAction as Action, NativeMatchAction as Scalar};

#[cfg(test)]
thread_local! { pub(in crate::native_artifact::program_runtime) static TEST_DISABLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

pub(super) struct FrozenSlot {
    pub map: FrozenPackedMap,
    pub token: u64,
    pub parent: Option<u32>,
    pub born_visit: u64,
}
fn cap(limit: Option<u64>) -> Result<u64, Error> {
    limit.ok_or_else(|| invalid("frozen read requires finite limits"))
}
fn check(required: u64, limit: Option<u64>, field: &'static str) -> Result<(), Error> {
    if required > cap(limit)? { Err(invalid(field)) } else { Ok(()) }
}
impl State<'_> {
    fn frozen_call(&mut self, ctx: &mut Context<'_, '_>, site: usize, operation: u64, args: [u64; 3])
        -> Result<u64, Error>
    {
        self.guard(ctx, site)?;
        if args[1..] != [0; 2] { return Err(invalid("malformed frozen-map argument lane")); }
        let layout = &self.descriptor.frozen_maps;
        let selected = match operation {
            FROZEN_MAP_COPY => &layout.copies,
            FROZEN_MAP_READ => &layout.reads,
            _ => return Err(invalid("unknown frozen-map helper")),
        };
        let id = selected.get(site).copied().flatten()
            .ok_or_else(|| invalid("frozen-map helper lacks exact selected source use"))?;
        let proof = &layout.regions[id];
        let frozen = self.slots[proof.owner].frozen.as_ref()
            .ok_or_else(|| invalid("frozen-map use before completed publication"))?;
        if args[0] == 0 || args[0] != frozen.token || frozen.parent != proof.parent
            || ctx.control.current != proof.parent || ctx.control.visits <= frozen.born_visit
        { return Err(invalid("stale or foreign frozen-map token/scope")); }
        ctx.control.work.charge(Phase::Setup, amount(proof.aliases.len(), 128, 512)?)?;
        let block = &self.descriptor.plan.blocks()[site];
        let result = match (operation, block.action()) {
            (FROZEN_MAP_COPY, Action::Scalar(Scalar::Copy { source, output })) => {
                if !proof.aliases.contains(&source.get()) || !proof.aliases.contains(&output.get()) {
                    return Err(invalid("frozen-map copy differs from exact alias inventory"));
                }
                ctx.control.work.charge(Phase::Setup, 256)?;
                args[0]
            }
            (FROZEN_MAP_READ, Action::LibraryCall { template, operands, output }) => {
                if operands.len() != 2 || !proof.aliases.contains(&operands[0].get()) {
                    return Err(invalid("frozen-map read differs from exact alias inventory"));
                }
                let query = ctx.arena.copy_current(operands[1].get(), &mut ctx.control.work)?;
                let ty = &self.descriptor.plan.values()[operands[1].get() as usize];
                if query.schema() != ty.schema() || query.schema_key() != ty.schema_key()
                    || query.memory_budget_retained_bytes(&ctx.library.memory_budget()).is_none()
                { return Err(invalid("unadmitted or wrong typed frozen-map query")); }
                let ValueData::String(text) = query.data() else { return Err(invalid("frozen query is not a String")); };
                let contains = match template.primitive() {
                    Primitive::MapContainsKey => true,
                    Primitive::MapKeyAccess => false,
                    _ => return Err(invalid("frozen helper is not a read-only map operation")),
                };
                let work = ctx.library.clone();
                // One finite original call scope covers the full byte/probe
                // work and typed output. No per-key or per-probe cap renewal.
                work.with_call_limits(cap(ctx.limits.max_compute_work)?, cap(ctx.limits.max_comparison_work)?, || {
                    let found = frozen.map.lookup(text).map_err(storage_error)?;
                    let word = if contains { u64::from(found.is_some()) } else {
                        found.ok_or(Error::Library { origin: block.origin(),
                            error: NativeCollectionInvocationError::Kernel(ResidentKernelError::InvalidInput) })?
                    };
                    let raw = if contains { ValueData::Bool(word != 0) } else { ValueData::U64(word) };
                    let out = &self.descriptor.plan.values()[output.get() as usize];
                    let footprint = canonical_data_retained_footprint(out.definition().body(), &raw)
                        .map_err(|_| invalid("frozen read output footprint differs"))?;
                    check(1, ctx.limits.max_output_elements, "frozen read output elements")?;
                    check(add(footprint.retained_bytes, size_of::<Value>() as u64)?,
                        ctx.limits.max_output_bytes, "frozen read output bytes")?;
                    // Private scalar construction clones no canonical map/key.
                    cap(ctx.limits.max_cloned_bytes)?;
                    let memory = work.memory_budget();
                    check(add(memory.used_bytes(), self.descriptor.scalar_bytes)?,
                        ctx.limits.max_temporary_bytes, "frozen read temporary bytes")?;
                    check(add(memory.retained_nodes(), 4)?, ctx.limits.max_retained_nodes,
                        "frozen read retained nodes")?;
                    work.charge(add(footprint.encoded_bytes, 256)?, 0)
                        .map_err(|error| storage_error(super::StorageError::Work(error)))?;
                    let value = ctx.scalar(*output, word)?;
                    ctx.arena.put(output.get(), &value, &mut ctx.control.work)?;
                    Ok(word)
                }).map_err(|error| storage_error(super::StorageError::Work(error)))??
            }
            _ => return Err(invalid("frozen-map helper differs from original action")),
        };
        self.last_owned_visit = Some(ctx.control.visits);
        if operation == FROZEN_MAP_READ { ctx.frozen_map_reads = add(ctx.frozen_map_reads, 1)?; }
        Ok(result)
    }
}
impl Context<'_, '_> {
    pub(in crate::native_artifact::program_runtime) fn frozen_native_action(&self, site: usize) -> bool {
        let proof = &self.control.descriptor.frozen_maps;
        proof.copies.get(site).copied().flatten().or_else(|| proof.reads.get(site).copied().flatten())
            .and_then(|id| self.map_owners.state.as_ref()?.slots.get(proof.regions[id].owner)?.frozen.as_ref())
            .is_some_and(|owner| owner.token != 0)
    }
    pub(in crate::native_artifact::program_runtime) fn frozen_map_helper(&mut self, site: usize,
        operation: u64, args: [u64; 3]) -> Result<Option<u64>, Error>
    {
        let mut state = self.map_owners.state.take()
            .ok_or_else(|| invalid("frozen-map helper before admitted session"))?;
        let result = state.frozen_call(self, site, operation, args);
        self.map_owners.state = Some(state);
        let word = result?;
        self.consumed = true;
        Ok(Some(word))
    }
    pub(in crate::native_artifact::program_runtime) fn retire_frozen_maps(&mut self, iteration: u32)
        -> Result<(), Error>
    {
        let Some(state) = self.map_owners.state.as_mut() else { return Ok(()); };
        self.control.work.charge(Phase::Setup, amount(state.descriptor.frozen_maps.regions.len(), 64, 64)?)?;
        for proof in &state.descriptor.frozen_maps.regions {
            if proof.parent == Some(iteration) {
                drop(state.slots[proof.owner].frozen.take());
            }
        }
        Ok(())
    }
}

//! One admitted typed constructor for a same-plan pure collection region.
//! Range preparation, lexical entry, ordinary finish and publication stay put.
use super::{CollectionExecution, Error, SourceBacking, State, add, charge, data, invalid};

fn fits(required: u64, allowance: Option<u64>) -> bool {
    allowance.is_some_and(|limit| required <= limit)
}

impl State {
    pub(in crate::native_artifact::program::runtime) fn fill_uniform_words(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        site: usize,
        word: u64,
        count: u64,
    ) -> Result<bool, Error> {
        let descriptor = ctx.descriptor();
        let regions = &descriptor.metadata().uniform_word_collections;
        // Fund bounded proof/builder/source lookups before inspecting them.
        let entries = regions
            .len()
            .checked_add(self.sources.len())
            .and_then(|n| n.checked_add(self.builders.len()))
            .ok_or_else(|| invalid("uniform collection metadata overflow"))?;
        charge(&mut ctx.work(), super::super::amount(entries, 128, 256)?)?;
        let region = regions
            .iter()
            .find(|region| region.header() as usize == site)
            .ok_or_else(|| invalid("uniform fill outside its exact same-plan header"))?;
        let id = region.collection();
        let iteration = region.iteration() as usize;
        let source = self
            .sources
            .get(iteration)
            .and_then(Option::as_ref)
            .ok_or_else(|| invalid("uniform fill has no current selected range"))?;
        if !matches!(source.backing, SourceBacking::U64Range { start: 0 })
            || count.checked_add(1) != Some(source.count)
            || ctx.progress().get(iteration * 2) != Some(&source.count)
            || ctx.progress().get(iteration * 2 + 1) != Some(&1)
            || source.index_offset != 0
            || source.element.is_some()
        {
            return Err(invalid("uniform fill outside its fresh inclusive range"));
        }
        let width = match region.kind() {
            mech_engine::native_artifact::NativeScalarKind::U8 if word <= u8::MAX as u64 => 1,
            mech_engine::native_artifact::NativeScalarKind::U16 if word <= u16::MAX as u64 => 2,
            mech_engine::native_artifact::NativeScalarKind::U32 if word <= u32::MAX as u64 => 4,
            mech_engine::native_artifact::NativeScalarKind::U64 => 8,
            _ => {
                return Err(invalid(
                    "uniform word differs from its proved unsigned kind",
                ));
            }
        };
        let output = &descriptor.metadata().plan.values()[descriptor.metadata().plan.collections()
            [id as usize]
            .output()
            .get() as usize];
        let node_count = data::output_nodes(output.definition().body(), count)?;
        let retained = self.retained_nodes()?;
        let old_live = self.live_bytes;
        let builder = self.builder(id)?;
        if builder.values.len() != 0
            || builder.values.capacity() != 0
            || builder.reservation.capacity_bytes() != 0
            || builder.payload_bytes != 0
            || builder.values.packed_width() != Some(width)
            || builder.join.is_some()
            || builder.query.is_some()
            || builder.histogram.is_some()
        {
            return Err(invalid("uniform fill requires its fresh packed builder"));
        }
        // Pure eligibility probes only. If any full construction/publication
        // bound cannot fit, retain the original per-element CFG and its error
        // precedence. No failed kernel or allocator is caught and retried.
        let geometry = (|| {
            let host_count = usize::try_from(count).ok()?;
            let bytes = count.checked_mul(width)?;
            let envelope = descriptor
                .metadata()
                .scalar_bytes
                .checked_add(bytes.checked_mul(4)?)?
                .checked_add(4096)?;
            let construct = bytes.checked_mul(16)?.checked_add(128)?;
            let allocation = bytes.checked_mul(16)?.checked_add(128)?;
            let finish = envelope.checked_mul(16)?.checked_add(256)?;
            let total_work = construct.checked_add(allocation)?.checked_add(finish)?;
            let component_compute = builder
                .compute
                .checked_add(construct)?
                .checked_add(finish)?;
            let cloned = builder.cloned_bytes.checked_add(bytes)?;
            let temporary = old_live.checked_add(bytes)?.checked_add(envelope)?;
            let output_bytes = builder
                .values
                .output_bytes(
                    bytes,
                    count,
                    host_count,
                    output.definition().dimension_parameters().len(),
                )
                .ok()?;
            Some((
                host_count,
                bytes,
                envelope,
                construct,
                allocation,
                total_work,
                component_compute,
                cloned,
                temporary,
                output_bytes,
            ))
        })();
        let Some((
            host_count,
            bytes,
            envelope,
            construct,
            allocation,
            total_work,
            compute,
            cloned,
            temporary,
            output_bytes,
        )) = geometry
        else {
            return Ok(false);
        };
        let limits = ctx.limits();
        if !fits(count, limits.max_output_elements)
            || !fits(cloned, limits.max_cloned_bytes)
            || !fits(compute, limits.max_compute_work)
            || !fits(temporary, limits.max_temporary_bytes)
            || !fits(output_bytes, limits.max_output_bytes)
            || !fits(add(node_count, retained)?, limits.max_retained_nodes)
        {
            return Ok(false);
        }
        let remaining = {
            let meter = ctx.work();
            meter.limit().saturating_sub(meter.used())
        };
        let Some((library_remaining, _)) = ctx.library().unscoped_remaining() else {
            return Ok(false);
        };
        if total_work > remaining || total_work > library_remaining {
            return Ok(false);
        }
        let memory = ctx.library().memory_budget();
        let probe_bytes = add(bytes, envelope)?;
        let probe = memory.reserve_capacity(probe_bytes);
        match probe {
            Ok(reservation) => drop(reservation),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
        let probe = memory.reserve_nodes(add(node_count, 1)?);
        match probe {
            Ok(reservation) => drop(reservation),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
        self.account(ctx, id, bytes, construct)?;
        charge(&mut ctx.work(), allocation)?;
        let builder = self.builder(id)?;
        let _replacement = memory.reserve_nodes(u64::from(host_count != 0))?;
        builder.reservation.resize_capacity(bytes)?;
        builder.values.reserve(host_count)?;
        builder
            .values
            .fill_unsigned(host_count, region.kind(), word)?;
        builder.payload_bytes = bytes;
        self.live_bytes = add(old_live, bytes)?;
        Ok(true)
    }
}

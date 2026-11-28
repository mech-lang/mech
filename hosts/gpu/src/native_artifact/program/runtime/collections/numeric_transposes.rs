//! Publish a solely transposed row builder in the exact selected column shape.
//! Eligibility never grants partial construction or relaxed source resources.
use super::*;
use crate::native_artifact::program::prepare::numeric_maps::Region;
fn fits(n: u64, cap: Option<u64>) -> bool {
    cap.is_some_and(|cap| n <= cap)
}
impl State {
    pub(in crate::native_artifact::program::runtime) fn finish_numeric_transpose(
        &mut self,
        ctx: &mut CollectionExecution<'_, '_, '_, '_>,
        region: &Region,
        prior: mech_core::ResourceDemand,
    ) -> Result<Option<Value>, Error> {
        let descriptor = ctx.descriptor();
        let metadata = descriptor.metadata();
        let plan = &metadata.plan;
        let projection = region
            .transpose
            .as_ref()
            .ok_or_else(|| invalid("numeric transpose lost proof"))?;
        let definition = &plan.collections()[region.collection as usize];
        if self.current != Some(region.collection)
            || ctx.ledger().current != definition.parent()
            || ctx.ledger().previous != Some(projection.finish)
            || self.native_map.is_some()
            || self.native_generator.is_some()
            || self.native_fill.is_some()
            || self.native_reduction.is_some()
            || self.has_utf8_read()
        {
            return Ok(None);
        }
        if projection
            .continuation
            .iter()
            .any(|&site| metadata.map_admission.group(site as usize).is_some())
        {
            return Ok(None);
        }
        let selected = metadata.calls[projection.transpose as usize]
            .as_ref()
            .ok_or_else(|| invalid("numeric transpose factory missing"))?;
        if !selected.supports_numeric_matrix_transpose() {
            return Ok(None);
        }
        let source = &plan.values()[region.output as usize];
        let target = &plan.values()[projection.output as usize];
        let retained = self.retained_nodes()?;
        let live_bytes = self.live_bytes;
        let builder = self.builder(region.collection)?;
        if builder.values.packed_width() != Some(u64::from(region.output_width))
            || builder.join.is_some()
            || builder.query.is_some()
            || builder.histogram.is_some()
            || builder.block_rows.rows().is_some()
            || builder.block_columns != 0
        {
            return Ok(None);
        }
        let count = builder.values.len() as u64;
        let Ok(output_bytes) = builder.values.output_bytes(
            builder.payload_bytes,
            count,
            builder.values.capacity(),
            source.definition().dimension_parameters().len(),
        ) else {
            return Ok(None);
        };
        let Some(super::numeric_transpose_budget::Funding {
            envelope,
            temporary,
            cloned,
            compute,
            setup,
            replay,
            output_bound,
            nodes,
            comparison,
        }) = super::numeric_transpose_budget::bound(
            descriptor,
            projection.transpose,
            &projection.continuation,
            super::numeric_transpose_budget::Row {
                count,
                width: region.output_width,
                capacity_bytes: builder.reservation.capacity_bytes(),
                finalization_cloned: builder
                    .values
                    .finalization_cloned_bytes(builder.payload_bytes),
                output_bytes,
            },
        )
        else {
            return Ok(None);
        };
        let Some(nodes) = prior
            .retained_nodes
            .checked_add(retained)
            .and_then(|n| n.checked_add(nodes))
        else {
            return Ok(None);
        };
        let total_temporary = prior
            .turn_peak_bytes
            .checked_add(live_bytes)
            .and_then(|n| n.checked_add(envelope))
            .and_then(|n| n.checked_add(temporary));
        let limits = ctx.limits();
        if !fits(count, limits.max_output_elements)
            || !fits(output_bound, limits.max_output_bytes)
            || total_temporary.is_none_or(|n| !fits(n, limits.max_temporary_bytes))
            || !fits(nodes, limits.max_retained_nodes)
            || !fits(comparison, limits.max_comparison_work)
            || builder
                .compute
                .checked_add(compute)
                .is_none_or(|n| !fits(n, limits.max_compute_work))
            || builder
                .cloned_bytes
                .checked_add(cloned)
                .is_none_or(|n| !fits(n, limits.max_cloned_bytes))
        {
            return Ok(None);
        }
        let Some(credit) = setup
            .checked_add(compute)
            .and_then(|n| n.checked_add(replay))
        else {
            return Ok(None);
        };
        if ctx.work().limit().saturating_sub(ctx.work().used()) < credit
            || ctx
                .library()
                .unscoped_remaining()
                .is_none_or(|(work, comparisons)| work < credit || comparisons < comparison)
        {
            return Ok(None);
        }
        let memory = ctx.library().memory_budget();
        let physical = add(envelope, temporary)?;
        match memory.reserve_capacity(physical) {
            Ok(probe) => drop(probe),
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let publication_nodes = data::output_nodes(target.definition().body(), count)?;
        let _publication_nodes = match memory.reserve_nodes(publication_nodes) {
            Ok(v) => v,
            Err(mech_core::MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        charge(&mut ctx.work(), setup)?;
        let _scratch = memory.reserve_capacity(temporary)?;
        if selected.template()
            != plan
                .prepared_library_template(projection.transpose as usize)
                .map_err(invalid)?
                .ok_or_else(|| invalid("numeric transpose template missing"))?
        {
            return Err(invalid("numeric transpose selected factory differs"));
        }
        for schema in [source.definition(), &selected.template().inputs()[0]] {
            if mech_core::shape_for_resolved_extents(schema, &[1, count]).is_err() {
                return Ok(None);
            }
        }
        if mech_core::shape_for_resolved_extents(selected.template().output(), &[count, 1]).is_err()
        {
            return Ok(None);
        }
        let Ok(shape) = mech_core::shape_for_resolved_extents(target.definition(), &[count, 1])
        else {
            return Ok(None);
        };
        // No constructor/call frontier has been consumed before full admission.
        // Failure after this point aborts atomically, with no cold allocator retry.
        self.account(ctx, region.collection, cloned, compute)?;
        ctx.library().charge(0, comparison).map_err(|error| {
            crate::native_artifact::NativeExecutionError::LibraryWork {
                phase: Phase::Completion,
                error,
            }
        })?;
        let mut reservation = memory.reserve_capacity(envelope)?;
        let builder = self.builder(region.collection)?;
        let value = core::mem::take(&mut builder.values)
            .finalize(
                target.schema(),
                shape,
                std::sync::Arc::clone(&metadata.schemas),
            )?
            .into_memory_budget(&mut reservation)?;
        let builder = self.builders[region.collection as usize].take().unwrap();
        self.live_bytes = self
            .live_bytes
            .checked_sub(builder.reservation.capacity_bytes())
            .ok_or_else(|| invalid("numeric transpose workspace accounting"))?;
        self.current = definition.parent_collection();
        Ok(Some(value))
    }
}

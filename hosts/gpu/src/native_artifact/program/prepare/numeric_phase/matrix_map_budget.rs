//! Complete original range/builder/transpose and native scratch/copy envelopes.
//! These prices are eligibility bounds, never owner or publication authority.
use super::super::reductions::FoldBudget;
use super::super::*;
use super::MatrixMap;
impl MatrixMap {
    pub fn budget(
        &self,
        plan: &Plan,
        calls: &[Option<NativePreparedLibraryCall>],
        visits: &[crate::native_artifact::iteration_abi::NativeIterationVisit],
    ) -> Option<FoldBudget> {
        let NativeControlLoop::Collection(child) =
            &plan.iterations()[self.region.iteration as usize]
        else {
            return None;
        };
        let mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) = child.source()
        else {
            return None;
        };
        let rows = match &self.region.rectangle {
            Some(rectangle) => rectangle.fixed()?[2],
            None => 1,
        };
        if cursor.operands().map(|v| v.get()) != [self.first, self.last]
            || self.count
                != super::super::reductions::constant(plan, self.last)?
                    .checked_sub(super::super::reductions::constant(plan, self.first)?)?
                    .checked_add(u64::from(self.inclusive))?
            || self.elements != self.count.checked_mul(rows)?
            || self.extents[0].checked_mul(self.extents[1])? != self.elements
        {
            return None;
        }
        let range = calls.get(cursor.prepare() as usize)?.as_ref()?;
        if range.template() != cursor.template() {
            return None;
        }
        let end = self.count.checked_sub(u64::from(self.inclusive))?;
        if range.selected_u64_range_cardinality(0, end).ok()? != self.count {
            return None;
        }
        let schema_bytes = range.schema_context_allocation_bound_bytes()?;
        let transpose = if let Some(t) = &self.region.transpose {
            Some(
                calls
                    .get(t.transpose as usize)?
                    .as_ref()
                    .filter(|call| call.supports_numeric_matrix_transpose())?,
            )
        } else {
            None
        };
        let mut binding_bytes = range
            .binding_metadata_bytes()
            .checked_add(transpose.map_or(0, |call| call.binding_metadata_bytes()))?;
        let mut binding_work = range
            .binding_work()
            .checked_add(transpose.map_or(0, |call| call.binding_work()))?;
        if let Some(rectangle) = &self.region.rectangle {
            let [start, end, rows] = rectangle.fixed()?;
            let NativeControlLoop::Collection(inner) =
                &plan.iterations()[rectangle.inner_iteration as usize]
            else {
                return None;
            };
            let mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) =
                inner.source()
            else {
                return None;
            };
            if super::super::reductions::constant(plan, cursor.operands()[0].get())? != start
                || super::super::reductions::constant(plan, cursor.operands()[1].get())? != end
                || cursor.prepare() != rectangle.range_prepare
            {
                return None;
            }
            let range = calls.get(rectangle.range_prepare as usize)?.as_ref()?;
            let transpose = calls.get(rectangle.transpose as usize)?.as_ref()?;
            if range.template() != cursor.template()
                || range.selected_u64_range_cardinality(start, end).ok()? != rows
                || !transpose.supports_numeric_matrix_transpose()
            {
                return None;
            }
            let input = plan.values()[plan.collections()[rectangle.inner_collection as usize]
                .output()
                .get() as usize]
                .definition();
            mech_core::shape_for_resolved_extents(range.template().output(), &[1, rows]).ok()?;
            mech_core::shape_for_resolved_extents(input, &[1, rows]).ok()?;
            mech_core::shape_for_resolved_extents(
                transpose.template().inputs().first()?,
                &[1, rows],
            )
            .ok()?;
            mech_core::shape_for_resolved_extents(transpose.template().output(), &[rows, 1])
                .ok()?;
            binding_bytes = binding_bytes
                .checked_add(range.binding_metadata_bytes())?
                .checked_add(transpose.binding_metadata_bytes())?;
            binding_work = binding_work
                .checked_add(range.binding_work())?
                .checked_add(transpose.binding_work())?;
        }
        let layout = super::super::super::numeric_maps::Layout {
            regions: vec![self.region.clone()],
            headers: Vec::new(),
        };
        let nested = super::super::super::numeric_maps::nested::Funding::derive(
            plan, &layout, calls, visits,
        )
        .at(&self.region)?;
        let mut source_work = 0u64;
        for site in self.source_sites() {
            source_work = source_work
                .checked_add(visits.get(site as usize)?.work)?
                .checked_add(super::super::super::tail::OVERHEAD)?;
            if let Some(template) = plan.prepared_library_template(site as usize).ok()? {
                if calls.get(site as usize)?.as_ref()?.template() != template {
                    return None;
                }
            }
        }
        source_work = source_work.checked_mul(2)?;
        let inventory = (plan.values().len() as u64)
            .checked_add(plan.blocks().len() as u64)?
            .checked_add(plan.iterations().len() as u64)?
            .checked_add(plan.collections().len() as u64)?;
        let mut reads = 0u64;
        let mut read_replay = 0u64;
        for read in &self.region.reads {
            let work = calls
                .get(read.site as usize)?
                .as_ref()?
                .scalar_matrix_element_native_work()?;
            reads = reads.checked_add(work)?;
            read_replay = read_replay
                .checked_add(work)?
                .checked_add(inventory.checked_mul(8192)?)?
                .checked_add(schema_bytes.checked_mul(128)?)?
                .checked_add(65536)?;
        }
        let native_step = nested.native_step.checked_add(
            (self.region.steps.len() as u64)
                .checked_mul(1024)?
                .checked_add((self.region.body.len() as u64).checked_mul(128)?)?
                .checked_add(8192)?,
        )?;
        let bank = self
            .elements
            .checked_mul(u64::from(self.region.output_width))?;
        // Fund all original draft banks, both materialized roots, selected
        // schema planning, and the simultaneous-update scratch bank. No row
        // constructor is treated as free merely because emission elides it.
        let cells = self.elements.checked_add(4)?.checked_mul(
            (core::mem::size_of::<mech_core::ValueDataDraft>() as u64)
                .checked_mul(16)?
                .checked_add(512)?,
        )?;
        let temporary = cells
            .checked_add(nested.temporary)?
            .checked_add(bank.checked_mul(16)?)?
            .checked_add(binding_bytes.checked_mul(128)?)?
            .checked_add(schema_bytes.checked_mul(32)?)?
            .checked_add(65536)?;
        let output = temporary
            .checked_add(nested.output)?
            .checked_add(bank.checked_mul(4)?)?
            .checked_add(4096)?;
        let setup = binding_work
            .checked_mul(128)?
            .checked_add(temporary.checked_mul(16)?)?
            .checked_add(4096)?;
        let per_element = source_work
            .checked_add(reads)?
            .checked_add(native_step)?
            .checked_add(nested.compute)?
            .checked_add(8192)?;
        let compute = self
            .elements
            .checked_add(1)?
            .checked_mul(per_element)?
            .checked_add(setup)?
            .checked_add(temporary.checked_mul(16)?)?;
        let probe = inventory
            .checked_mul(8192)?
            .checked_add(schema_bytes.checked_mul(128)?)?
            .checked_add(binding_work.checked_mul(1024)?)?
            .checked_add(temporary.checked_mul(128)?)?
            .checked_add(65536)?;
        let replay = self
            .elements
            .checked_add(1)?
            .checked_mul(
                source_work
                    .checked_add(read_replay)?
                    .checked_add(nested.replay)?
                    .checked_add(8192)?,
            )?
            .checked_add(probe)?;
        Some(FoldBudget {
            schema_bytes,
            compute,
            replay,
            temporary,
            output,
            nodes: self
                .elements
                .checked_mul(8)?
                .checked_add(nested.nodes)?
                .checked_add(binding_bytes)?
                .checked_add(256)?,
            elements: self.elements.max(nested.elements),
            cloned: cells
                .checked_add(self.elements.checked_mul(nested.cloned)?)?
                .checked_add(bank.checked_mul(16)?)?
                .checked_add(binding_bytes)?,
            comparison: output
                .checked_add(temporary)?
                .checked_add(self.elements.checked_mul(nested.comparison)?)?,
            setup,
            native_step,
        })
    }
}

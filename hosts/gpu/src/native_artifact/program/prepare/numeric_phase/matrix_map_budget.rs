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
        let binding_bytes = range
            .binding_metadata_bytes()
            .checked_add(transpose.map_or(0, |call| call.binding_metadata_bytes()))?;
        let binding_work = range
            .binding_work()
            .checked_add(transpose.map_or(0, |call| call.binding_work()))?;
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
        let native_step = (self.region.steps.len() as u64)
            .checked_mul(1024)?
            .checked_add((self.region.body.len() as u64).checked_mul(128)?)?
            .checked_add(8192)?;
        let bank = self
            .count
            .checked_mul(u64::from(self.region.output_width))?;
        // Fund all original draft banks, both materialized roots, selected
        // schema planning, and the simultaneous-update scratch bank. No row
        // constructor is treated as free merely because emission elides it.
        let cells = self.count.checked_add(4)?.checked_mul(
            (core::mem::size_of::<mech_core::ValueDataDraft>() as u64)
                .checked_mul(16)?
                .checked_add(512)?,
        )?;
        let temporary = cells
            .checked_add(bank.checked_mul(16)?)?
            .checked_add(binding_bytes.checked_mul(128)?)?
            .checked_add(schema_bytes.checked_mul(32)?)?
            .checked_add(65536)?;
        let output = temporary
            .checked_add(bank.checked_mul(4)?)?
            .checked_add(4096)?;
        let setup = binding_work
            .checked_mul(128)?
            .checked_add(temporary.checked_mul(16)?)?
            .checked_add(4096)?;
        let per_element = source_work
            .checked_add(reads)?
            .checked_add(native_step)?
            .checked_add(8192)?;
        let compute = self
            .count
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
            .count
            .checked_add(1)?
            .checked_mul(source_work.checked_add(read_replay)?.checked_add(8192)?)?
            .checked_add(probe)?;
        Some(FoldBudget {
            schema_bytes,
            compute,
            replay,
            temporary,
            output,
            nodes: self
                .count
                .checked_mul(8)?
                .checked_add(binding_bytes)?
                .checked_add(256)?,
            elements: self.count,
            cloned: cells
                .checked_add(bank.checked_mul(16)?)?
                .checked_add(binding_bytes)?,
            comparison: output.checked_add(temporary)?,
            setup,
            native_step,
        })
    }
}

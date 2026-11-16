//! Invocation-current immutable array loans for scalar-state native Tails.
//! The arena retains each exact owner until the synchronous closed loan ends;
//! the selected body cannot mutate owners, retire values, or call the host.
use super::{Error, Invocation, invalid};
use crate::native_artifact::NativeExecutionPhase;
use mech_core::{MemoryObjectOwner, ResourceDemand, first_call_memory_budget_violation};

impl Invocation<'_, '_, '_> {
    pub(super) fn begin_captured_numeric_views(&mut self, id: usize) -> Result<bool, Error> {
        let metadata = self.descriptor.metadata();
        let region = &metadata.closed_tails.regions[id];
        let proof = region
            .captures
            .as_ref()
            .ok_or_else(|| invalid("missing numeric capture proof"))?;
        if self.has_uninventoried_private_owners()
            || [
                self.limits.max_output_elements,
                self.limits.max_output_bytes,
                self.limits.max_temporary_bytes,
                self.limits.max_cloned_bytes,
                self.limits.max_retained_nodes,
                self.limits.max_compute_work,
                self.limits.max_comparison_work,
            ]
            .iter()
            .any(Option::is_none)
        {
            return Ok(false);
        }
        let memory = self.state.library.memory_budget();
        let mut setup = metadata
            .observation_schema_bytes
            .checked_mul(proof.captures.len() as u64 * 8)
            .and_then(|n| n.checked_add(proof.words() as u64 * 128 + 2048));
        for capture in &proof.captures {
            let Some(owner) = self
                .state
                .arena
                .native_entry_preview(capture.value, &self.state.control.work)?
            else {
                return Ok(false);
            };
            let Some(bytes) = owner.memory_budget_retained_bytes(&memory) else {
                return Ok(false);
            };
            setup = setup.and_then(|n| n.checked_add(bytes.checked_mul(8)?));
        }
        let Some(setup) = setup else {
            return Ok(false);
        };
        if self
            .state
            .library
            .unscoped_remaining()
            .is_none_or(|(remaining, _)| setup > remaining)
        {
            return Ok(false);
        }
        self.state
            .control
            .work
            .charge(NativeExecutionPhase::Setup, setup)?;
        let Some(temporary) = metadata
            .observation_schema_bytes
            .checked_mul(proof.captures.len() as u64)
            .and_then(|n| n.checked_add(proof.words() as u64 * 8 + 1024))
        else {
            return Ok(false);
        };
        let Ok(_temporary) = memory.reserve_capacity(temporary) else {
            return Ok(false);
        };
        let prior = self.selected_join_coexistence(None)?;
        for (index, capture) in proof.captures.iter().enumerate() {
            let Some(owner) = self
                .state
                .arena
                .native_entry_preview(capture.value, &self.state.control.work)?
            else {
                return Ok(false);
            };
            let call = metadata.calls[capture.witness]
                .as_ref()
                .ok_or_else(|| invalid("numeric capture lost selected factory"))?;
            let Some(bytes) = owner.memory_budget_retained_bytes(&memory) else {
                return Ok(false);
            };
            let Some(nodes) = owner.memory_budget_retained_nodes(&memory) else {
                return Ok(false);
            };
            // Reproduce the maintained selected call's logical scalar result
            // and current-owner coexistence bounds. Eight bytes covers length
            // as well as every unsigned read. Tight policies decline at entry
            // so an unselected read or earlier assertion retains its boundary.
            let Some(mut demand) =
                prior
                    .output_elements
                    .checked_add(1)
                    .map(|output_elements| ResourceDemand {
                        output_elements,
                        ..prior
                    })
            else {
                return Ok(false);
            };
            let Some(peak) = demand
                .turn_peak_bytes
                .checked_add(bytes)
                .and_then(|n| n.checked_add(16))
            else {
                return Ok(false);
            };
            let Some(retained) = demand
                .retained_nodes
                .checked_add(nodes)
                .and_then(|n| n.checked_add(2))
            else {
                return Ok(false);
            };
            demand.turn_peak_bytes = peak;
            demand.retained_nodes = retained;
            let Some(compute) = demand.work.compute.checked_add(region.compute) else {
                return Ok(false);
            };
            let Some(comparison) = demand.work.comparison.checked_add(region.comparison) else {
                return Ok(false);
            };
            demand.work.compute = compute;
            demand.work.comparison = comparison;
            let owner_id = MemoryObjectOwner::NodeOutput {
                node: call.template().origin().node(),
                port: 0,
            };
            if first_call_memory_budget_violation(owner_id, demand, 8, 0, self.limits).is_some() {
                return Ok(false);
            }
            let Some(view) = call.unsigned_matrix_native_view(owner, &memory) else {
                return Ok(false);
            };
            for (lane, word) in view.into_iter().enumerate() {
                self.write_borrowed_word(2 + index * 4 + lane, word)?;
            }
        }
        Ok(true)
    }
}

//! Original fold constructor bounds under the phase's existing owner account.
use super::{Error, Invocation, invalid};
use crate::native_artifact::program::prepare::readonly_numeric::Operation;

impl Invocation<'_, '_, '_> {
    pub(super) fn admit_numeric_reductions(&mut self, id: usize) -> Result<bool, Error> {
        let metadata = self.descriptor.metadata();
        let region = &metadata.closed_tails.regions[id];
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| invalid("fold entry lost numeric proof"))?;
        if !proof
            .actions
            .iter()
            .any(|(_, a)| matches!(a, Operation::Reduce { .. }))
        {
            return Ok(true);
        }
        let Some(prior) = self.retained_caller_inventory(region.tail.initialize() as usize)? else {
            return Ok(false);
        };
        let memory = self.state.library.memory_budget();
        let mut owner_bytes = 0u64;
        let mut owner_nodes = 0u64;
        for array in &proof.arrays {
            let Some(owner) = self.numeric_array_owner(array)? else {
                return Ok(false);
            };
            let Some(bytes) = owner.memory_budget_retained_bytes(&memory) else {
                return Ok(false);
            };
            let Some(nodes) = owner.memory_budget_retained_nodes(&memory) else {
                return Ok(false);
            };
            let Some(bytes) = owner_bytes.checked_add(bytes) else {
                return Ok(false);
            };
            let Some(nodes) = owner_nodes.checked_add(nodes) else {
                return Ok(false);
            };
            owner_bytes = bytes;
            owner_nodes = nodes;
        }
        let fits = |need: u64, cap: Option<u64>| cap.is_some_and(|cap| need <= cap);
        for (_, action) in &proof.actions {
            let Operation::Reduce { fold } = action else {
                continue;
            };
            let b = &fold.budget;
            if b.schema_bytes != metadata.observation_schema_bytes {
                return Err(invalid(
                    "fold schema accounting differs from its selected factories",
                ));
            }
            // All source banks, shape metadata and one first-change copy of
            // each root may coexist. No elided row is priced as zero storage.
            let bounds = (|| {
                let extra_bytes = b.temporary.checked_add(owner_bytes.checked_mul(4)?)?;
                let extra_nodes = b.nodes.checked_add(owner_nodes.checked_mul(4)?)?;
                Some((
                    prior.live_bytes.checked_add(extra_bytes)?,
                    prior.live_nodes.checked_add(extra_nodes)?,
                    b.cloned.checked_add(owner_bytes.checked_mul(8)?)?,
                    extra_bytes,
                    extra_nodes,
                ))
            })();
            let Some((temporary, nodes, cloned, physical, physical_nodes)) = bounds else {
                return Ok(false);
            };
            if !fits(b.elements, self.limits.max_output_elements)
                || !fits(b.output, self.limits.max_output_bytes)
                || !fits(temporary, self.limits.max_temporary_bytes)
                || !fits(nodes, self.limits.max_retained_nodes)
                || !fits(cloned, self.limits.max_cloned_bytes)
                || !fits(b.comparison, self.limits.max_comparison_work)
                || !fits(b.compute, self.limits.max_compute_work)
            {
                return Ok(false);
            }
            let Ok(bytes) = memory.reserve_capacity(physical) else {
                return Ok(false);
            };
            let Ok(nodes) = memory.reserve_nodes(physical_nodes) else {
                return Ok(false);
            };
            drop(nodes);
            drop(bytes);
        }
        Ok(true)
    }
}

//! Bounded geometry history for maintained shallow numeric calls.
//! Every slot owns its existing exact binding and plan cache, never Values or
//! permits. Current facts and admission still run after every geometry hit.
use super::*;
use crate::resident::budget::native_turn_plan_cache::NativeTurnPlanCache;
const SLOTS: usize = 64;
const HISTORY_NODE_WORK: u64 = 4096;

fn slot_demand(
    binding: &CachedBinding,
    work: &NativeLibraryWorkBudget,
) -> Result<ResourceDemand, InvokeError> {
    if !binding.work.shares_account(work) || binding._reservation.budget() != work.memory_budget() {
        return Err(InvokeError::InvalidInput);
    }
    let mut demand = ResourceDemand {
        turn_peak_bytes: binding
            ._reservation
            .capacity_bytes()
            .checked_add(binding.projection_index.bytes())
            .ok_or(InvokeError::InvalidInput)?,
        retained_nodes: 1u64
            .checked_add(if binding.call.is_some() {
                binding.binding_nodes
            } else {
                0
            })
            .and_then(|n| n.checked_add(binding.projection_index.nodes()))
            .ok_or(InvokeError::InvalidInput)?,
        ..ResourceDemand::default()
    };
    if let NativePlanCacheState::Cached(plans) = &binding.plans {
        let next = plans.retained_metadata_demand(work)?;
        demand.turn_peak_bytes = demand
            .turn_peak_bytes
            .checked_add(next.turn_peak_bytes)
            .ok_or(InvokeError::InvalidInput)?;
        demand.retained_nodes = demand
            .retained_nodes
            .checked_add(next.retained_nodes)
            .ok_or(InvokeError::InvalidInput)?;
    }
    Ok(demand)
}
fn add(total: &mut ResourceDemand, next: ResourceDemand) -> Result<(), InvokeError> {
    total.turn_peak_bytes = total
        .turn_peak_bytes
        .checked_add(next.turn_peak_bytes)
        .ok_or(InvokeError::InvalidInput)?;
    total.retained_nodes = total
        .retained_nodes
        .checked_add(next.retained_nodes)
        .ok_or(InvokeError::InvalidInput)?;
    Ok(())
}
impl NativePreparedLibraryCallCache {
    fn count(&self) -> Result<usize, InvokeError> {
        let n = self
            .binding
            .as_ref()
            .map(|b| {
                b.history_len
                    .checked_add(1)
                    .ok_or(InvokeError::InvalidInput)
            })
            .transpose()?
            .unwrap_or(0);
        if n > SLOTS {
            return Err(InvokeError::InvalidInput);
        }
        Ok(n)
    }
    pub(super) fn all_metadata_demand(
        &self,
        work: &NativeLibraryWorkBudget,
    ) -> Result<ResourceDemand, InvokeError> {
        let count = self.count()?;
        work.charge(
            256u64
                .checked_add(
                    (count.saturating_sub(1) as u64)
                        .checked_mul(HISTORY_NODE_WORK)
                        .ok_or(InvokeError::InvalidInput)?,
                )
                .ok_or(InvokeError::InvalidInput)?,
            0,
        )?;
        let mut demand = ResourceDemand::default();
        let mut next = self.binding.as_deref();
        let mut seen = 0;
        while let Some(binding) = next {
            if seen == count {
                return Err(InvokeError::InvalidInput);
            }
            add(&mut demand, slot_demand(binding, work)?)?;
            seen += 1;
            next = binding.previous.as_deref();
        }
        if seen != count {
            return Err(InvokeError::InvalidInput);
        }
        Ok(demand)
    }
    pub(super) fn promote_geometry(
        &mut self,
        prepared: &NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        output: &Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<(), InvokeError> {
        let count = self.count()?;
        if count <= 1 {
            return Ok(());
        }
        let each = prepared
            .lookup_work
            .checked_add(HISTORY_NODE_WORK)
            .ok_or(InvokeError::InvalidInput)?;
        work.charge(
            (count as u64)
                .checked_mul(each)
                .ok_or(InvokeError::InvalidInput)?,
            0,
        )?;
        let mut next = self.binding.as_deref();
        let mut hit = None;
        let mut seen = 0;
        while let Some(binding) = next {
            if seen == count
                || binding.projection_count.is_some()
                || !Arc::ptr_eq(&binding.identity, &prepared.cache_identity)
                || binding.limits != limits
                || !binding.work.shares_account(work)
                || binding._reservation.budget() != work.memory_budget()
            {
                return Err(InvokeError::InvalidInput);
            }
            if hit.is_none()
                && binding
                    .call
                    .as_ref()
                    .is_some_and(|call| prepared.geometry_matches(call, inputs, output))
            {
                hit = Some(seen);
            }
            seen += 1;
            next = binding.previous.as_deref();
        }
        if seen != count {
            return Err(InvokeError::InvalidInput);
        }
        if let Some(index) = hit.filter(|i| *i != 0) {
            let mut link = &mut self.binding;
            for _ in 0..index {
                link = &mut link.as_mut().unwrap().previous;
            }
            let mut selected = link.take().unwrap();
            *link = selected.previous.take();
            selected.previous = self.binding.take();
            selected.history_len = count - 1;
            self.binding = Some(selected);
        }
        Ok(())
    }
    pub(super) fn retain_for_new_geometry(
        &mut self,
        prepared: &NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        output: &Option<Value>,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<Option<Box<CachedBinding>>, InvokeError> {
        let count = self.count()?;
        if count == 0 {
            return Ok(None);
        }
        if self.binding.as_ref().unwrap().call.is_none() {
            return Ok(None);
        }
        if count == SLOTS {
            // The lookup already prepaid bounded list traversal and unlinking.
            let mut link = &mut self.binding;
            for _ in 1..count {
                link = &mut link.as_mut().unwrap().previous;
            }
            drop(link.take());
            self.binding.as_mut().unwrap().history_len = count - 2;
        }
        let held = self.all_metadata_demand(work)?;
        let owner = core::mem::size_of::<CachedBinding>() as u64;
        let call = self
            .binding
            .as_ref()
            .and_then(|b| b.call.as_ref())
            .ok_or(InvokeError::InvalidInput)?;
        // The prepared envelope bounds the maintained invocation envelope for
        // every geometry of this exact template. Include a FULL future plan
        // cache before retaining another binding; a later cold plan allocation
        // must not consume headroom reserved for the ordinary fallback.
        if call.invocation_metadata_bytes > prepared.binding_bytes {
            if let Some(binding) = self.binding.as_mut() {
                binding.previous = None;
                binding.history_len = 0;
            }
            return Ok(None);
        }
        let future = NativeTurnPlanCache::retained_bytes(&call.memory, prepared.binding_bytes);
        let future_nodes =
            NativeTurnPlanCache::retained_data_nodes(&call.memory).and_then(|n| n.checked_add(1));
        let fixed = prepared
            .binding_bytes
            .checked_mul(3)
            .and_then(|n| n.checked_add(owner))
            .and_then(|n| n.checked_add(future?))
            .and_then(|n| {
                n.checked_add(crate::resident::budget::payload::native_scope_metadata_bytes())
            });
        let extra_nodes = prepared
            .binding_nodes
            .checked_add(1)
            .and_then(|n| n.checked_add(future_nodes?));
        let fits = match (fixed, extra_nodes) {
            (Some(additional), Some(nodes)) => {
                let total = held.turn_peak_bytes.checked_add(additional);
                let total_nodes = held.retained_nodes.checked_add(nodes);
                match (total, total_nodes) {
                    (Some(total), Some(total_nodes)) => super::super::cache_capacity::admits(
                        &prepared.template,
                        &prepared.schemas,
                        inputs,
                        output,
                        limits,
                        work,
                        total,
                        total_nodes,
                        additional,
                        nodes,
                    )?,
                    _ => false,
                }
            }
            _ => false,
        };
        if fits {
            return Ok(self.binding.take());
        }
        // Keeping the ordinary one-slot replacement is always preferable to
        // retaining history without a current coexistence proof.
        if let Some(binding) = self.binding.as_mut() {
            binding.previous = None;
            binding.history_len = 0;
        }
        Ok(None)
    }
}

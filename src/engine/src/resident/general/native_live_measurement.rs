//! Same native live-facts checks, with optional fixed-count metadata admission.
use super::*;
use live::MeasurementCheck;
use mech_core::{ManagedMemoryBudget, SchemaBody, ValueData, snapshot::SequenceView};

pub(super) struct NativeMeasurementCheck<'a> {
    call: &'a NativeCollectionCall,
    cache: Option<&'a NativeTurnPlanCache>,
    memory: &'a ManagedMemoryBudget,
}

impl<'a> NativeMeasurementCheck<'a> {
    pub(super) fn new(
        call: &'a NativeCollectionCall,
        cache: Option<&'a NativeTurnPlanCache>,
        memory: &'a ManagedMemoryBudget,
    ) -> Self {
        Self {
            call,
            cache,
            memory,
        }
    }

    fn check_limits(
        &self,
        footprint: CurrentMemoryFootprint,
        mut observed: ResourceDemand,
    ) -> Result<(), MemoryPlanError> {
        observed.turn_peak_bytes = observed
            .turn_peak_bytes
            .checked_add(self.call.invocation_metadata_bytes)
            .and_then(|n| n.checked_add(self.cache.map_or(0, |cache| cache.bytes())))
            .ok_or(MemoryPlanError::TargetAddressOverflow)?;
        observed.retained_nodes = observed
            .retained_nodes
            .max(footprint.retained_nodes)
            .checked_add(self.cache.map_or(0, |cache| cache.index_nodes()))
            .ok_or(MemoryPlanError::TargetAddressOverflow)?;
        live::check_measurement_with_limits(footprint, observed, self.call.memory.target.limits)
    }
}

impl MeasurementCheck for NativeMeasurementCheck<'_> {
    fn check(
        &mut self,
        footprint: CurrentMemoryFootprint,
        observed: ResourceDemand,
    ) -> Result<(), MemoryPlanError> {
        budget::observe_control_work(observed).map_err(measurement_error)?;
        self.check_limits(footprint, observed)
    }

    fn try_metadata_scan(
        &mut self,
        value: &Value,
        body: &SchemaBody,
        footprint: CurrentMemoryFootprint,
        mut observed: ResourceDemand,
    ) -> Result<bool, MemoryPlanError> {
        let (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix)) = (body, value.data())
        else {
            return Ok(false);
        };
        if !matches!(element.as_ref(), SchemaBody::String) {
            return Ok(false);
        }
        let SequenceView::String(strings) = matrix.elements() else {
            return Ok(false);
        };
        let Some(nodes) = u64::try_from(strings.len())
            .ok()
            .and_then(|count| count.checked_add(2))
        else {
            return Ok(false);
        };
        let Some(owned_bytes) = value.memory_budget_retained_bytes(self.memory) else {
            return Ok(false);
        };
        // The actual admitted root covers the Matrix/Box<str> carriers and
        // every immutable String allocation. For packed Strings their encoded
        // 8-byte lengths fit within those carriers too. This bound ONLY proves
        // the visitor's sums cannot overflow; it never becomes a measured fact
        // or allocation/publication permit. No second payload/selector walk.
        if footprint.payload_bytes.checked_add(owned_bytes).is_none()
            || footprint.encoded_bytes.checked_add(owned_bytes).is_none()
        {
            return Ok(false);
        }
        let Some(retained_nodes) = footprint.retained_nodes.checked_add(nodes) else {
            return Ok(false);
        };
        let Some(compute) = observed.work.compute.checked_add(nodes) else {
            return Ok(false);
        };
        observed.work.compute = compute;
        let upper = CurrentMemoryFootprint {
            retained_nodes,
            ..footprint
        };
        // A packed String matrix contributes exactly N+2 one-node chunks.
        // Work and retained-node guards are monotone throughout this walk.
        // Preview BOTH existing authorities without changing their high-water
        // marks; a refusal leaves the original first-denied/paid prefix intact.
        if budget::preview_control_work(observed).is_err()
            || self.check_limits(upper, observed).is_err()
        {
            return Ok(false);
        }
        self.check(upper, observed)?;
        Ok(true)
    }
}

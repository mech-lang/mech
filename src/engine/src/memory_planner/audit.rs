use std::collections::BTreeMap;

use mech_core::{
    MemoryObjectId, MemoryPlanAuditMismatch, MemoryPlanAuditReport, MemoryPlanAuditStatus,
    MemoryPlanError, MemoryPlanObservation,
};

use super::ProgramMemoryPlan;

pub fn audit_program_memory_plan(
    plan: &ProgramMemoryPlan,
    observations: &[MemoryPlanObservation],
) -> Result<MemoryPlanAuditReport, MemoryPlanError> {
    let observed = observations
        .iter()
        .map(|observation| (observation.object, observation))
        .collect::<BTreeMap<_, _>>();
    let planned = plan
        .allocations
        .iter()
        .map(|allocation| (allocation.id, allocation))
        .collect::<BTreeMap<_, _>>();
    for observation in observations {
        if !planned.contains_key(&observation.object) {
            return Err(MemoryPlanError::ObservationUnexpected {
                object: observation.object,
            });
        }
    }
    let mut statuses = Vec::new();
    let mut mismatches = Vec::new();
    for (object, allocation) in planned {
        let Some(observation) = observed.get(&object) else {
            return Err(MemoryPlanError::ObservationMissing { object });
        };
        // Fixed Resident/device backing must cover the current extent exactly.
        // Only future capacity and variable payload/workspace facts are bounds.
        let exact_current = matches!(
            allocation.space,
            mech_core::MemorySpace::ResidentCpu | mech_core::MemorySpace::Device { .. }
        ) && !matches!(
            allocation.role,
            mech_core::AllocationRole::VariablePayload
                | mech_core::AllocationRole::ConstructionWorkspace
        ) && allocation.payload_block_capacity == 0;
        compare(
            &mut mismatches,
            object,
            "current_bytes",
            allocation.current_bytes,
            observation.current_bytes,
            exact_current,
        );
        compare(
            &mut mismatches,
            object,
            "capacity_bytes",
            allocation.capacity_bytes,
            observation.capacity_bytes,
            false,
        );
        let value = plan.values.iter().find(|value| value.object == object);
        if let Some(value) = value {
            compare(
                &mut mismatches,
                object,
                "payload_bytes",
                value.layout.payload.current_bytes,
                observation.payload_bytes,
                false,
            );
            compare(
                &mut mismatches,
                object,
                "retained_nodes",
                value.layout.payload.current_nodes,
                observation.retained_nodes,
                false,
            );
            compare(
                &mut mismatches,
                object,
                "logical_elements",
                value.layout.current_elements,
                observation.logical_elements,
                exact_current
                    && !matches!(
                        value.layout.storage,
                        mech_core::StorageLayoutClass::CanonicalSnapshot { .. }
                    ),
            );
        } else {
            compare(
                &mut mismatches,
                object,
                "payload_bytes",
                0,
                observation.payload_bytes,
                false,
            );
            compare(
                &mut mismatches,
                object,
                "retained_nodes",
                0,
                observation.retained_nodes,
                false,
            );
        }
        let current_exact = allocation.current_bytes == observation.current_bytes
            && value.map_or(true, |value| {
                value.layout.payload.current_bytes == observation.payload_bytes
                    && value.layout.payload.current_nodes == observation.retained_nodes
                    && value.layout.current_elements == observation.logical_elements
            });
        statuses.push((
            object,
            if current_exact && allocation.capacity_bytes == observation.capacity_bytes {
                MemoryPlanAuditStatus::Exact
            } else if observation.capacity_bytes < allocation.capacity_bytes {
                MemoryPlanAuditStatus::CapacityDeferredToR6
            } else {
                MemoryPlanAuditStatus::WithinPlannedCapacity
            },
        ));
    }
    statuses.sort_by_key(|(object, _)| *object);
    mismatches.sort_by_key(|mismatch| (mismatch.object, mismatch.field));
    Ok(MemoryPlanAuditReport {
        statuses: statuses.into_boxed_slice(),
        mismatches: mismatches.into_boxed_slice(),
    })
}

/// Public R5 audit entry point named by the architecture contract.
pub fn audit_memory_plan(
    plan: &ProgramMemoryPlan,
    observations: &[MemoryPlanObservation],
) -> Result<MemoryPlanAuditReport, MemoryPlanError> {
    audit_program_memory_plan(plan, observations)
}

fn compare(
    mismatches: &mut Vec<MemoryPlanAuditMismatch>,
    object: MemoryObjectId,
    field: &'static str,
    planned: u64,
    observed: u64,
    exact: bool,
) {
    if observed > planned || (exact && observed != planned) {
        mismatches.push(MemoryPlanAuditMismatch {
            object,
            field,
            planned,
            observed,
        });
    }
}

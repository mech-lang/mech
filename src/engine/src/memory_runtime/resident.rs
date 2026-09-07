use mech_core::{
    AccessMode, FunctionInvocation, ManagedCallAccessRequest, MemoryAccessMode, MemoryAccessRegion,
    MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult, PortMemoryPlan, PreparedCallAccess,
    RealizedMemoryPlan, RegionAccessPlan,
};

use crate::memory_planner::ProgramMemoryPlan;

use super::ManagedProgramMemory;

/// Resident activation uses the same R5-to-R6 realization authority as every
/// other CPU host. It may not construct a call-local synthetic memory plan.
pub fn realize_resident_memory(
    plan: &ProgramMemoryPlan,
) -> MemoryRuntimeResult<ManagedProgramMemory> {
    ManagedProgramMemory::realize(plan)
}

/// Resolves a function invocation's logical cells onto the current plan
/// revision and prepares its complete lease set once.
pub fn prepare_managed_function_call(
    domain: &MemoryDomain,
    realized: &RealizedMemoryPlan,
    invocation: &FunctionInvocation,
    call: &mech_core::CallMemoryPlan,
) -> MemoryRuntimeResult<PreparedCallAccess> {
    if invocation.input_count() != call.inputs.len() || call.outputs.len() != 1 {
        return Err(MemoryRuntimeError::InvalidLayout {
            object: None,
            size: invocation.input_count() as u64,
            alignment: 1,
            reason: "managed invocation and call-plan arity differ",
        });
    }
    let requirements = call
        .bound_call
        .operation_descriptor()
        .contract
        .memory_requirements(invocation.input_count())
        .map_err(|_| MemoryRuntimeError::InvalidLayout {
            object: None,
            size: invocation.input_count() as u64,
            alignment: 1,
            reason: "operation memory requirements cannot be derived",
        })?;
    let mut accesses = Vec::new();
    accesses
        .try_reserve_exact(call.inputs.len() + call.outputs.len())
        .map_err(|_| MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: (call.inputs.len() + call.outputs.len()) as u64,
            alignment: 1,
            space: mech_core::MemorySpace::Host,
        })?;
    for (index, (cell, port)) in invocation
        .input_cells()
        .iter()
        .zip(call.inputs.iter())
        .enumerate()
    {
        let policy = requirements
            .inputs
            .get(index)
            .ok_or(MemoryRuntimeError::InvalidLayout {
                object: Some(port.object),
                size: index as u64,
                alignment: 1,
                reason: "input memory policy is absent",
            })?;
        accesses.push(ManagedCallAccessRequest::for_logical_cell(
            cell.reactive_cell_id(),
            domain.plan_object_key(realized.revision(), port.object)?,
            access_mode(policy.access),
            access_region(port, false),
        ));
    }
    let output_cell = invocation.output_cell();
    for (index, port) in call.outputs.iter().enumerate() {
        let policy = requirements
            .outputs
            .get(index)
            .ok_or(MemoryRuntimeError::InvalidLayout {
                object: Some(port.object),
                size: index as u64,
                alignment: 1,
                reason: "output memory policy is absent",
            })?;
        accesses.push(ManagedCallAccessRequest::for_logical_cell(
            output_cell.reactive_cell_id(),
            domain.plan_object_key(realized.revision(), port.object)?,
            access_mode(policy.access),
            access_region(port, true),
        ));
    }
    domain.prepare_managed_call(realized, &accesses)
}

const fn access_mode(mode: AccessMode) -> MemoryAccessMode {
    match mode {
        AccessMode::Read => MemoryAccessMode::Read,
        AccessMode::Write => MemoryAccessMode::Write,
        AccessMode::ReadWrite | AccessMode::Consume => MemoryAccessMode::ExclusiveInPlace,
    }
}

fn access_region(port: &PortMemoryPlan, output: bool) -> MemoryAccessRegion {
    match &port.region {
        RegionAccessPlan::Contiguous {
            offset_bytes,
            length_bytes,
        } => MemoryAccessRegion::Contiguous {
            offset_bytes: *offset_bytes,
            length_bytes: *length_bytes,
        },
        RegionAccessPlan::Strided {
            offset_bytes,
            count,
            stride_bytes,
            element_bytes,
        } => MemoryAccessRegion::Strided {
            offset_bytes: *offset_bytes,
            count: *count,
            stride_bytes: *stride_bytes,
            element_bytes: *element_bytes,
        },
        RegionAccessPlan::Rectangle {
            base_offset_bytes,
            rows,
            columns,
            row_stride_bytes,
            column_stride_bytes,
        } => MemoryAccessRegion::Rectangle {
            offset_bytes: *base_offset_bytes,
            rows: *rows,
            columns: *columns,
            row_stride_bytes: *row_stride_bytes,
            column_stride_bytes: *column_stride_bytes,
            element_bytes: port.value.slot.bytes,
        },
        RegionAccessPlan::WholeValue if output => MemoryAccessRegion::Contiguous {
            offset_bytes: 0,
            length_bytes: port.value.current_address_span_bytes,
        },
        RegionAccessPlan::WholeValue
        | RegionAccessPlan::Gather { .. }
        | RegionAccessPlan::CollectionEntry { .. }
        | RegionAccessPlan::Deferred(_) => MemoryAccessRegion::WholeInitialized,
    }
}

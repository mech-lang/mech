use mech_core::{
    AccessMode, AliasDecision, FunctionInvocation, ManagedCallAccessRequest, MemoryAccessMode,
    MemoryAccessRegion, MemoryDomain, MemoryObjectId, MemoryRuntimeError, MemoryRuntimeResult,
    PortMemoryPlan, PreparedCallAccess, RealizedMemoryPlan, RegionAccessPlan,
    TransactionRequirement,
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
        accesses.push(ManagedCallAccessRequest::for_input_cell(
            cell.reactive_cell_id(),
            index,
            domain.plan_object_key(realized.revision(), port.object)?,
            input_access_mode(call, index, policy.access),
            access_region(port, false)?,
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
        let transaction =
            call.transactions
                .get(index)
                .copied()
                .ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(port.object),
                    size: index as u64,
                    alignment: 1,
                    reason: "output transaction requirement is absent",
                })?;
        let write_object = transaction_write_object(transaction, port.object)?;
        accesses.push(ManagedCallAccessRequest::for_output_cell(
            output_cell.reactive_cell_id(),
            index,
            domain.plan_object_key(realized.revision(), write_object)?,
            output_access_mode(transaction, policy.access),
            access_region(port, true)?,
        ));
    }
    domain.prepare_managed_call(realized, &accesses)
}

fn input_access_mode(
    call: &mech_core::CallMemoryPlan,
    input: usize,
    mode: AccessMode,
) -> MemoryAccessMode {
    match mode {
        AccessMode::Read => MemoryAccessMode::Read,
        AccessMode::Write => MemoryAccessMode::Write,
        AccessMode::Consume => MemoryAccessMode::Read,
        AccessMode::ReadWrite
            if call.aliases.iter().any(|alias| {
                matches!(alias, AliasDecision::InPlaceRequired { input: candidate }
                    if usize::from(*candidate) == input)
            }) =>
        {
            MemoryAccessMode::ExclusiveInPlace
        }
        AccessMode::ReadWrite => MemoryAccessMode::Read,
    }
}

const fn output_access_mode(
    transaction: TransactionRequirement,
    mode: AccessMode,
) -> MemoryAccessMode {
    match transaction {
        TransactionRequirement::UndoSnapshot { .. } => MemoryAccessMode::ExclusiveInPlace,
        TransactionRequirement::StageAndSwap { .. }
        | TransactionRequirement::DoubleBuffer { .. } => MemoryAccessMode::Write,
        TransactionRequirement::None => match mode {
            AccessMode::Read => MemoryAccessMode::Read,
            AccessMode::Write => MemoryAccessMode::Write,
            AccessMode::ReadWrite => MemoryAccessMode::ExclusiveInPlace,
            AccessMode::Consume => MemoryAccessMode::Read,
        },
    }
}

const fn transaction_write_object(
    transaction: TransactionRequirement,
    declared_output: MemoryObjectId,
) -> MemoryRuntimeResult<MemoryObjectId> {
    match transaction {
        TransactionRequirement::None => Ok(declared_output),
        TransactionRequirement::StageAndSwap { staged, .. }
        | TransactionRequirement::DoubleBuffer { next: staged, .. } => Ok(staged),
        TransactionRequirement::UndoSnapshot { target, .. } => Ok(target),
    }
}

fn access_region(port: &PortMemoryPlan, output: bool) -> MemoryRuntimeResult<MemoryAccessRegion> {
    Ok(match &port.region {
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
        RegionAccessPlan::WholeValue => MemoryAccessRegion::WholeInitialized,
        RegionAccessPlan::Gather { .. } => {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(port.object),
                size: port.value.current_address_span_bytes,
                alignment: port.value.slot.alignment,
                reason: "gather access requires a concrete bounded selector plan",
            });
        }
        RegionAccessPlan::CollectionEntry { .. } => {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(port.object),
                size: port.value.current_address_span_bytes,
                alignment: port.value.slot.alignment,
                reason: "collection-entry access requires a concrete key plan",
            });
        }
        RegionAccessPlan::Deferred(_) => {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(port.object),
                size: port.value.current_address_span_bytes,
                alignment: port.value.slot.alignment,
                reason: "deferred access region reached managed call acquisition",
            });
        }
    })
}

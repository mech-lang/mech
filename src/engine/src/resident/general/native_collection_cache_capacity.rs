//! Optional current-call cache capacity, never a payload or kernel permit.
use super::{
    NativeCollectionInvocationError as InvokeError, NativeProgramLibraryTemplate, headroom,
};
use crate::resident::budget::NativeLibraryWorkBudget;
use mech_core::{ManagedMemoryBudget, MemoryBudgetLimits, MemoryRuntimeError, SchemaTable, Value};

pub(super) fn metadata_work(inputs: usize) -> Option<u64> {
    let ports = u64::try_from(inputs.checked_add(1)?).ok()?;
    (core::mem::size_of::<Value>() as u64)
        .checked_mul(ports)?
        .checked_mul(8)?
        .checked_add(1024)
}

/// Metadata is charged before reading the fixed current input/shape tapes.
/// The returned bound grants only cache coexistence; all maintained call facts,
/// selector checks, work limits and atomic kernel publication still execute.
pub(super) fn admits(
    template: &NativeProgramLibraryTemplate,
    schemas: &SchemaTable,
    inputs: &[Option<Value>],
    output: &Option<Value>,
    limits: MemoryBudgetLimits,
    work: &NativeLibraryWorkBudget,
    fixed_bytes: u64,
    fixed_nodes: u64,
    additional_bytes: u64,
    additional_nodes: u64,
) -> Result<bool, InvokeError> {
    let metadata = metadata_work(inputs.len()).ok_or(InvokeError::InvalidInput)?;
    work.charge(metadata, 0)?;
    let memory = work.memory_budget();
    let Some(bound) = headroom::current_call_bound(template, schemas, inputs, output, &memory)
    else {
        return Ok(false);
    };
    let Some(peak) = bound.temporary_bytes.checked_add(fixed_bytes) else {
        return Ok(false);
    };
    let Some(nodes) = bound.retained_nodes.checked_add(fixed_nodes) else {
        return Ok(false);
    };
    if limits.max_temporary_bytes.is_none_or(|limit| peak > limit)
        || limits.max_retained_nodes.is_none_or(|limit| nodes > limit)
    {
        return Ok(false);
    }
    let Some(bytes) = bound.temporary_bytes.checked_add(additional_bytes) else {
        return Ok(false);
    };
    let Some(nodes) = bound.retained_nodes.checked_add(additional_nodes) else {
        return Ok(false);
    };
    capacity(&memory, bytes, nodes)
}

fn capacity(memory: &ManagedMemoryBudget, bytes: u64, nodes: u64) -> Result<bool, InvokeError> {
    let _bytes = match memory.reserve_capacity(bytes) {
        Ok(reservation) => reservation,
        Err(MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let _nodes = match memory.reserve_nodes(nodes) {
        Ok(reservation) => reservation,
        Err(MemoryRuntimeError::BudgetExceeded { .. }) => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    Ok(true)
}

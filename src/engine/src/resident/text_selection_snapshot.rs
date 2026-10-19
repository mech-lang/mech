//! Two physical carriers for the SAME maintained grapheme-selection kernel.
//! No selector interpretation or String slicing is duplicated here.
use super::*;
use mech_core::{Value, snapshot::ValueDataDraft};

pub(super) fn source<'a>(
    kernel: &BoundResidentKernel,
    input: ResidentValueRef<'a>,
) -> Result<&'a str, ResidentKernelError> {
    match input {
        ResidentValueRef::String([text]) => Ok(text),
        ResidentValueRef::Snapshot([Some(value)]) => {
            let expected = kernel
                .bound_call()
                .and_then(|call| call.inputs().first())
                .ok_or(ResidentKernelError::InvalidInput)?;
            if expected.schema().body() != &SchemaBody::String
                || value.schema_key() != expected.schema().key()
                || value.shape() != expected.shape()
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            let ValueData::String(text) = value.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            Ok(text)
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

pub(super) fn admit(
    kernel: &BoundResidentKernel,
    output: &ResidentValueMut<'_>,
    payload: usize,
    work: usize,
    scratch: usize,
) -> Result<(), ResidentKernelError> {
    match output {
        ResidentValueMut::String([target]) => super::admit_string_materialization(
            1,
            payload,
            payload,
            work,
            target.len().max(payload),
            1,
            scratch,
            0,
        ),
        ResidentValueMut::Snapshot([current]) => {
            admit_snapshot(kernel, current.as_ref(), payload, work, scratch)
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

fn admit_snapshot(
    kernel: &BoundResidentKernel,
    current: Option<&Value>,
    payload: usize,
    work: usize,
    scratch: usize,
) -> Result<(), ResidentKernelError> {
    use super::super::budget;
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let next = mech_core::snapshot::prospective_string_value_footprint(
        metadata.shape.parameter_values().len(),
        false,
        1,
        budget::checked_u64(payload)?,
    )
    .map_err(|_| ResidentKernelError::InvalidShape)?;
    let mut meter = budget::ResidentBudgetMeter::default();
    let comparison = if let Some(current) = current {
        let old = budget::published_canonical_footprint(&mut meter, current, schemas)?;
        budget::projected_language_equality_work(
            schemas,
            current,
            old,
            metadata.schema,
            metadata.shape.parameter_values().len(),
            next,
        )?
    } else {
        0
    };
    let measured = meter.estimate();
    let staging = payload
        .checked_add(scratch)
        .and_then(|bytes| {
            metadata
                .shape
                .parameter_values()
                .len()
                .checked_mul(3 * core::mem::size_of::<u64>())?
                .checked_add(bytes)
        })
        .and_then(|bytes| {
            bytes.checked_add(
                core::mem::size_of::<ValueDataDraft>() + core::mem::size_of::<String>(),
            )
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    let finalization = budget::checked_u64(payload)?
        .checked_add(1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    budget::PreparedKernel::new((), budget::resident_cost! {
        comparison_work: measured.comparison_work().checked_add(comparison).ok_or(ResidentKernelError::InvalidShape)?,
        compute_work: measured.compute_work().checked_add(budget::checked_u64(work)?)
            .and_then(|n| n.checked_add(comparison)).and_then(|n| n.checked_add(finalization)).ok_or(ResidentKernelError::InvalidShape)?,
        output_elements: 1,
        output_bytes: next.retained_bytes,
        temporary_bytes: next.retained_bytes.checked_add(budget::checked_u64(staging)?).ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: payload,
        retained_nodes: next.node_count.checked_add(2).ok_or(ResidentKernelError::InvalidShape)?,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    Ok(())
}

pub(super) fn publish(
    kernel: &BoundResidentKernel,
    output: ResidentValueMut<'_>,
    next: String,
) -> Result<bool, ResidentKernelError> {
    match output {
        ResidentValueMut::String([target]) => {
            let changed = *target != next;
            if changed {
                *target = next;
            }
            Ok(changed)
        }
        output @ ResidentValueMut::Snapshot([_]) => {
            let work = super::super::budget::checked_u64(next.len())?
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
            super::super::numeric::write_snapshot_data_with_work_budget(
                kernel,
                output,
                ValueDataDraft::String(next),
                Some(work),
            )
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

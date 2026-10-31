//! Aggregate selection keeps each component's independent dimension arena.
use super::*;

pub(super) fn component_supported(
    request: &ResidentKernelBindRequest<'_>,
    component: &SchemaBody,
    output: &SchemaBody,
) -> bool {
    component == output
        || (request.output.kind == ResidentValueKind::Snapshot
            && crate::resident::structural_values::carrier_component_matches(output, component))
}

pub(super) fn selected_component<'a>(
    body: &'a SchemaBody,
    data: &'a ValueData,
    ordinal: Option<usize>,
) -> Option<(&'a SchemaBody, &'a ValueData)> {
    let index = ordinal?;
    match (body, data) {
        (SchemaBody::Tuple(fields), ValueData::Tuple(values)) => {
            Some((fields.get(index)?, values.get(index)?))
        }
        (SchemaBody::Record(fields), ValueData::Record(values)) => {
            Some((&fields.get(index)?.schema, values.fields().get(index)?))
        }
        (SchemaBody::Map { value, .. }, ValueData::Map(values)) => {
            Some((value, values.entries().get(index)?.value()))
        }
        _ => None,
    }
}

fn fund_metadata(kernel: &BoundResidentKernel) -> Result<u64, ResidentKernelError> {
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut meter = super::super::budget::ResidentBudgetMeter::default();
    let schema_bytes = meter.precharge_schema_layout(schemas)?;
    // Source/selected/target closure, dimension witnesses and their growth
    // coexist. This visits schema metadata, never array elements.
    let bytes = schema_bytes
        .checked_mul(16)
        .and_then(|bytes| bytes.checked_add(4096))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let work = bytes
        .checked_mul(8)
        .and_then(|work| work.checked_add(meter.estimate().compute_work()))
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::super::budget::PreparedKernel::new(
        (),
        super::super::budget::resident_cost! {
            compute_work: work, temporary_bytes: bytes,
            ..super::super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(bytes)
}

pub(super) fn closed_source(
    kernel: &BoundResidentKernel,
    source: &mech_core::Value,
    schema: &mech_core::Schema,
) -> Result<(Option<SchemaBody>, u64), ResidentKernelError> {
    if schema.dimension_parameters().is_empty()
        || !matches!(
            schema.body(),
            SchemaBody::Tuple(_) | SchemaBody::Record(_) | SchemaBody::Map { .. }
        )
    {
        return Ok((None, 0));
    }
    let bytes = fund_metadata(kernel)?;
    let closed = schema
        .closed_body(source.shape())
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    Ok((Some(closed), bytes))
}

pub(super) fn current_shape(
    kernel: &BoundResidentKernel,
    component: Option<&SchemaBody>,
    metadata_funded: bool,
) -> Result<(Option<ShapeInstance>, u64), ResidentKernelError> {
    let Some(component) = component else {
        return Ok((None, 0));
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let output = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let target = schemas
        .get(output.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    if target.dimension_parameters().is_empty() && component == target.body() {
        return Ok((None, 0));
    }
    let bytes = if metadata_funded {
        0
    } else {
        fund_metadata(kernel)?
    };
    let shape =
        mech_core::shape_for_schema_components(target, &[(target.body(), component.clone())], None)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
    Ok((Some(shape), bytes))
}

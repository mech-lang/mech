//! Ordinary capacity-bearing sequence leaves. Canonical values remain immutable.
use super::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::intrinsics::sequence_buffers::{self as primitive, Operation};
use mech_core::*;
use std::sync::Arc;

#[derive(Debug)]
struct Plan {
    operation: Operation,
    width: usize,
    inputs: Box<[(SchemaId, SchemaKey)]>,
}
pub(super) fn bind_with_capacity(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, Operation::WithCapacity)
}
pub(super) fn bind_reserve(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, Operation::Reserve)
}
pub(super) fn bind_append(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, Operation::Append)
}
pub(super) fn bind_clear(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind(request, Operation::Clear)
}
fn bind(
    request: &ResidentKernelBindRequest<'_>,
    operation: Operation,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let expected = operation.contract();
    let [output] = contract.outputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let policies = expected
        .inputs
        .resolve(operation.arity())
        .map_err(|_| ResidentKernelBindError::UnsupportedContract)?;
    if request.inputs.len() != operation.arity()
        || contract.inputs.len() != operation.arity()
        || contract.interaction != expected.interaction
        || output.schema != request.output.schema_id
        || output.access != expected.outputs[0].access
        || output.delivery != expected.outputs[0].delivery
        || output.construction != expected.outputs[0].construction
        || output.alias != expected.outputs[0].alias
        || output.change_detection != expected.outputs[0].change_detection
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .zip(&policies)
            .any(|((port, layout), policy)| {
                port.schema != layout.schema_id
                    || port.access != policy.access
                    || port.delivery != policy.delivery
            })
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let bodies = request
        .inputs
        .iter()
        .chain([&request.output])
        .map(|layout| {
            let schema = request
                .schemas
                .get(layout.schema_id)
                .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
            if schema.key() != layout.schema_key
                || layout.kind != ResidentValueKind::Snapshot
                || layout.shape != ResidentShape::SCALAR
            {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            }
            Ok(schema.body().clone())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (output_body, input_bodies) = bodies.split_last().unwrap();
    if !primitive::schemas_supported(operation, input_bodies, output_body) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let width = primitive::width(primitive::element(output_body).unwrap()).unwrap();
    Ok(BoundResidentKernel::new(execute, Box::new([]))
        .with_retained_state(Arc::new(Plan {
            operation,
            width,
            inputs: request
                .inputs
                .iter()
                .map(|port| (port.schema_id, port.schema_key))
                .collect(),
        }))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone())
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}
fn input<'a>(
    plan: &Plan,
    inputs: &'a dyn ResidentKernelInputs,
    index: usize,
) -> Result<&'a Value, ResidentKernelError> {
    let ResidentValueRef::Snapshot([Some(value)]) =
        inputs.get(index).ok_or(ResidentKernelError::InvalidInput)?
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    if (value.schema(), value.schema_key()) != plan.inputs[index] {
        return Err(ResidentKernelError::InvalidInput);
    }
    Ok(value)
}
fn execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let _work_scope = budget::ControlWorkScopeGuard::enter();
    let plan = kernel
        .retained_state::<Plan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let source = (plan.operation != Operation::WithCapacity)
        .then(|| input(plan, inputs, 0))
        .transpose()?;
    let argument = if plan.operation.arity() == 2 {
        primitive::scalar_word(input(plan, inputs, 1)?.data())
            .map_err(|_| ResidentKernelError::InvalidInput)?
    } else {
        0
    };
    let geometry = primitive::geometry(plan.operation, source, plan.width, argument)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .shared_snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema_scratch = schema
        .clone_allocation_bound_bytes()
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(4096))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new((), budget::resident_cost! { compute_work: schema_scratch, temporary_bytes: schema_scratch, ..budget::KernelCostEstimate::default() }).admit()?.into_plan();
    let shape = shape_for_resolved_extents(schema, &[1, geometry.length as u64])
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let prospective = geometry
        .footprint(shape.parameter_values().len())
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let mut meter = ResidentBudgetMeter::default();
    let source_footprint = source
        .map(|source| budget::measure_canonical_value_footprint(&mut meter, source, schemas))
        .transpose()?;
    let prior = target
        .as_ref()
        .map(|prior| budget::published_canonical_footprint(&mut meter, prior, schemas))
        .transpose()?;
    let comparison = target
        .as_ref()
        .zip(prior)
        .map(|(prior, footprint)| {
            budget::projected_language_equality_work(
                schemas,
                prior,
                footprint,
                metadata.schema,
                shape.parameter_values().len(),
                prospective,
            )
        })
        .transpose()?
        .unwrap_or(0);
    let source_bytes = source_footprint.map_or(0, |f| f.retained_bytes);
    let prior_bytes = prior.map_or(0, |f| f.retained_bytes);
    let output_bytes = prospective.retained_bytes;
    let measured = meter.estimate();
    let cloned = if plan.operation == Operation::Clear {
        0
    } else {
        source.map_or(0, |source| match source.data() {
            ValueData::Matrix(matrix) => matrix.elements().len() as u64 * plan.width as u64,
            _ => 0,
        })
    };
    let temporary = output_bytes
        .checked_add(source_bytes)
        .and_then(|n| n.checked_add(prior_bytes))
        .and_then(|n| n.checked_add(schema_scratch))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let compute = temporary
        .checked_mul(8)
        .and_then(|n| n.checked_add(measured.compute_work()))
        .and_then(|n| n.checked_add(comparison))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let nodes = prospective
        .node_count
        .checked_add(source_footprint.map_or(0, |f| f.node_count))
        .and_then(|n| n.checked_add(prior.map_or(0, |f| f.node_count)))
        .and_then(|n| n.checked_add(8))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new((), budget::resident_cost! {
        compute_work: compute, comparison_work: measured.comparison_work().checked_add(comparison).ok_or(ResidentKernelError::InvalidShape)?,
        output_elements: geometry.length, output_bytes, temporary_bytes: temporary, cloned_bytes: cloned, retained_nodes: nodes,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let buffer = primitive::copy_buffer(plan.operation, source, geometry, argument)
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let next =
        Value::from_unsigned_matrix_buffer(metadata.schema, shape, buffer, Arc::clone(schemas))
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let changed = target
        .as_ref()
        .map(|prior| {
            prior
                .snapshot_eq(schemas, &next, schemas)
                .map(|same| !same)
                .map_err(|_| ResidentKernelError::InvalidOutput)
        })
        .transpose()?
        .unwrap_or(true);
    // This is the ordinary staged snapshot lane. Its enclosing payload scope
    // admits ownership before the invocation can publish the candidate.
    *target = Some(next);
    Ok(changed)
}

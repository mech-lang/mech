//! Budgeted resident implementations of ordinary text/lines and text/join.
use super::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::intrinsics::text_ops::{self as primitives, Operation};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, ImplementationMemoryClass, MResult,
    OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError,
    ResidentKernelInputs, ResidentKernelWorkAccounting, ResidentShape, ResidentSnapshotOutput,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResolvedOperationContract, SchemaBody,
    SchemaId, SchemaKey, ShapeInstance, ShapeRule, ValueData, ValueDataDraft, ValueDraft,
    snapshot::{
        SequenceView, SnapshotCanonicalizationBudget, SnapshotValidationContext,
        prospective_string_value_footprint,
    },
};
use std::sync::Arc;

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(
        ["text"],
        "lines",
        ImplementationMemoryClass::CanonicalFinalize,
        bind_lines,
    )?;
    builder.insert_resident_factory(
        ["text"],
        "join",
        ImplementationMemoryClass::CanonicalFinalize,
        bind_join,
    )
}

#[derive(Clone, Debug)]
struct TextPlan {
    input_schema: SchemaId,
    input_key: SchemaKey,
    kind: ResidentValueKind,
    dimensions: (usize, usize),
    fixed: bool,
}
fn dimensions(body: &SchemaBody, shape: &ShapeInstance) -> Option<(usize, usize)> {
    let SchemaBody::Matrix { dimensions, .. } = body else {
        return None;
    };
    let [rows, columns] = dimensions.as_ref() else {
        return None;
    };
    Some((
        usize::try_from(shape.resolve_dimension(rows).ok()?).ok()?,
        usize::try_from(shape.resolve_dimension(columns).ok()?).ok()?,
    ))
}
fn validate(
    request: &ResidentKernelBindRequest<'_>,
    operation: Operation,
) -> Result<(), ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [output] = contract.outputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if request.inputs.len() != operation.arity()
        || contract.inputs.len() != operation.arity()
        || contract.interaction != ExternalInteraction::Pure
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
        || output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.alias != AliasPolicy::NoAlias
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.change_detection
            != if operation == Operation::Lines {
                ChangeDetectionPolicy::KernelReported
            } else {
                ChangeDetectionPolicy::ExactScalar
            }
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(())
}
fn bind_lines(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate(request, Operation::Lines)?;
    let source = &request.inputs[0];
    let output = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if source.kind != ResidentValueKind::String
        || source.shape != ResidentShape::SCALAR
        || request
            .schemas
            .get(source.schema_id)
            .map(|schema| schema.body())
            != Some(&SchemaBody::String)
        || !primitives::is_string_matrix(output.body())
        || dimensions(output.body(), &request.output.shape_instance).map(|shape| shape.0) != Some(1)
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(execute_lines, Box::new([]))
        .with_snapshot_schemas(request.schemas.clone())
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}
fn bind_join(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate(request, Operation::Join)?;
    let [source, separator] = request.inputs else {
        unreachable!()
    };
    let schema = request
        .schemas
        .get(source.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let shape = dimensions(schema.body(), &source.shape_instance)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if !primitives::is_string_matrix(schema.body())
        || !matches!(
            source.kind,
            ResidentValueKind::String | ResidentValueKind::Snapshot
        )
        || (source.kind == ResidentValueKind::Snapshot && source.shape != ResidentShape::SCALAR)
        || (source.kind == ResidentValueKind::String
            && (source.shape.rows as usize, source.shape.columns as usize) != shape)
        || separator.kind != ResidentValueKind::String
        || separator.shape != ResidentShape::SCALAR
        || request
            .schemas
            .get(separator.schema_id)
            .map(|schema| schema.body())
            != Some(&SchemaBody::String)
        || request.output.kind != ResidentValueKind::String
        || request.output.shape != ResidentShape::SCALAR
        || request
            .schemas
            .get(request.output.schema_id)
            .map(|schema| schema.body())
            != Some(&SchemaBody::String)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(execute_join, Box::new([]))
        .with_retained_state(Arc::new(TextPlan {
            input_schema: source.schema_id,
            input_key: source.schema_key,
            kind: source.kind,
            dimensions: shape,
            fixed: source.activation_fixed_shape || schema.dimension_parameters().is_empty(),
        }))
        .with_snapshot_schemas(request.schemas.clone())
        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered))
}

fn copy_text(source: &str) -> Result<String, ResidentKernelError> {
    let mut result = String::new();
    result
        .try_reserve_exact(source.len())
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    result.push_str(source);
    Ok(result)
}
fn execute_lines(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([text])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
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
    let shape_scratch = schema
        .clone_allocation_bound_bytes()
        .and_then(|bytes| bytes.checked_mul(4))
        .and_then(|bytes| {
            bytes.checked_add((metadata.shape.parameter_values().len() as u64 + 2).checked_mul(64)?)
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    // Admit the complete delimiter scan and shape scratch before either runs.
    let scan = text
        .len()
        .checked_mul(4)
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {compute_work: scan, temporary_bytes: shape_scratch, ..budget::KernelCostEstimate::default()},
    )
    .admit()?
    .into_plan();
    let count = text.lines().count();
    let shape = mech_core::shape_for_resolved_extents(schema, &[1, budget::checked_u64(count)?])
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let prospective = prospective_string_value_footprint(
        shape.parameter_values().len(),
        true,
        budget::checked_u64(count)?,
        budget::checked_u64(text.len())?,
    )
    .map_err(|_| ResidentKernelError::InvalidShape)?;
    let mut meter = ResidentBudgetMeter::default();
    let comparison = if let Some(previous) = target.as_ref() {
        let previous_footprint =
            budget::published_canonical_footprint(&mut meter, previous, schemas)?;
        budget::projected_language_equality_work(
            schemas,
            previous,
            previous_footprint,
            metadata.schema,
            shape.parameter_values().len(),
            prospective,
        )?
    } else {
        0
    };
    let draft_bytes = count
        .checked_mul(core::mem::size_of::<ValueDataDraft>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let finalization_work = budget::checked_u64(text.len())?
        .checked_mul(8)
        .and_then(|work| work.checked_add(budget::checked_u64(count).ok()?.checked_mul(8)?))
        .and_then(|work| work.checked_add(32))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let measured = meter.estimate();
    PreparedKernel::new((), budget::resident_cost! {
        comparison_work: measured.comparison_work().checked_add(comparison).ok_or(ResidentKernelError::InvalidShape)?,
        compute_work: measured.compute_work().checked_add(finalization_work).and_then(|work| work.checked_add(budget::checked_u64(scan).ok()?)).and_then(|work| work.checked_add(comparison)).ok_or(ResidentKernelError::InvalidShape)?,
        output_elements: count, output_bytes: prospective.retained_bytes,
        temporary_bytes: prospective.retained_bytes.checked_add(budget::checked_u64(draft_bytes)?).and_then(|bytes| bytes.checked_add(shape_scratch)).ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: text.len().checked_mul(2).ok_or(ResidentKernelError::InvalidShape)?, retained_nodes: prospective.node_count.checked_mul(2).ok_or(ResidentKernelError::InvalidShape)?,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let mut lines = Vec::new();
    lines
        .try_reserve_exact(count)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    for line in text.lines() {
        lines.push(ValueDataDraft::String(copy_text(line)?));
    }
    let canonical = SnapshotCanonicalizationBudget::new(finalization_work);
    let next = ValueDraft {
        schema: metadata.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data: ValueDataDraft::Matrix(lines.into_boxed_slice()),
    }
    .finalize(
        &SnapshotValidationContext::with_shared_schemas(schemas)
            .with_canonicalization_budget(&canonical),
    )
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let changed = match target.as_ref() {
        Some(previous) => !previous
            .snapshot_eq(schemas, &next, schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?,
        None => true,
    };
    *target = Some(next);
    Ok(changed)
}

enum Texts<'a> {
    Dense(&'a [String], usize, usize),
    Canonical(&'a [Box<str>]),
}
impl Texts<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Dense(values, ..) => values.len(),
            Self::Canonical(values) => values.len(),
        }
    }
    fn get(&self, index: usize) -> &str {
        match self {
            Self::Dense(values, rows, columns) => {
                &values[(index % columns) * rows + index / columns]
            }
            Self::Canonical(values) => &values[index],
        }
    }
}
fn texts<'a>(
    kernel: &BoundResidentKernel,
    input: ResidentValueRef<'a>,
) -> Result<Texts<'a>, ResidentKernelError> {
    let plan = kernel
        .retained_state::<TextPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if input.kind() != plan.kind {
        return Err(ResidentKernelError::InvalidInput);
    }
    match input {
        ResidentValueRef::String(values)
            if plan.dimensions.0.checked_mul(plan.dimensions.1) == Some(values.len()) =>
        {
            Ok(Texts::Dense(values, plan.dimensions.0, plan.dimensions.1))
        }
        ResidentValueRef::Snapshot([Some(value)])
            if value.schema() == plan.input_schema && value.schema_key() == plan.input_key =>
        {
            let schema = kernel
                .snapshot_schemas()
                .and_then(|schemas| schemas.get(plan.input_schema))
                .ok_or(ResidentKernelError::InvalidInput)?;
            let actual = dimensions(schema.body(), value.shape())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let ValueData::Matrix(matrix) = value.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let SequenceView::String(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            if (plan.fixed && actual != plan.dimensions)
                || actual.0.checked_mul(actual.1) != Some(values.len())
            {
                return Err(ResidentKernelError::InvalidShape);
            }
            Ok(Texts::Canonical(values))
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}
fn execute_join(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let strings = texts(
        kernel,
        inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?,
    )?;
    let Some(ResidentValueRef::String([separator])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let scan = strings
        .len()
        .checked_mul(4)
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new(
        (),
        budget::resident_cost! {compute_work: scan, ..budget::KernelCostEstimate::default()},
    )
    .admit()?
    .into_plan();
    let payload = (0..strings.len())
        .try_fold(0usize, |bytes, index| {
            bytes
                .checked_add(strings.get(index).len())
                .ok_or(ResidentKernelError::InvalidShape)
        })?
        .checked_add(
            separator
                .len()
                .checked_mul(strings.len().saturating_sub(1))
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
        .ok_or(ResidentKernelError::InvalidShape)?;
    let comparison = target.len().max(payload);
    let compute = scan
        .checked_add(
            payload
                .checked_mul(4)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
        .and_then(|work| work.checked_add(comparison))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new((), budget::resident_cost! {
        compute_work: compute, comparison_work: comparison, output_elements: 1,
        output_bytes: payload.checked_add(core::mem::size_of::<String>()).ok_or(ResidentKernelError::InvalidShape)?,
        temporary_bytes: payload, cloned_bytes: payload, retained_nodes: 1,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let mut joined = String::new();
    joined
        .try_reserve_exact(payload)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    for index in 0..strings.len() {
        if index != 0 {
            joined.push_str(separator);
        }
        joined.push_str(strings.get(index));
    }
    let changed = *target != joined;
    if changed {
        *target = joined;
    }
    Ok(changed)
}

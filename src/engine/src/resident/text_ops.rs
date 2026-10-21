//! Budgeted resident implementations of ordinary text/lines and text/join.
use super::budget::{self, PreparedKernel, ResidentBudgetMeter};
use crate::intrinsics::text_ops::{self as primitives, Operation};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentPortLayout, ResidentShape, ResidentSnapshotOutput,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResolvedOperationContract, SchemaBody,
    SchemaId, SchemaKey, ShapeInstance, ShapeRule, ValueData, ValueDataDraft, ValueDraft,
    snapshot::{
        SequenceView, SnapshotCanonicalizationBudget, SnapshotValidationContext,
        StringMatrixSnapshotConstructor, prospective_string_value_footprint,
    },
};
use std::sync::Arc;

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    super::native_collection_semantics::register_text(builder)
}

#[derive(Clone, Debug)]
struct TextPlan {
    input_schema: SchemaId,
    input_key: SchemaKey,
    kind: ResidentValueKind,
    dimensions: (usize, usize),
    fixed: bool,
    separator: ScalarTextPlan,
    output_kind: ResidentValueKind,
}

/// Representation adapter only: both paths borrow the same String semantics.
/// Canonical values retain their exact schema identity and are never unpacked
/// or copied into a second dense String before the maintained leaf runs.
#[derive(Clone, Debug)]
pub(super) struct ScalarTextPlan {
    schema: SchemaId,
    key: SchemaKey,
    kind: ResidentValueKind,
}
impl ScalarTextPlan {
    pub(super) fn bind(
        request: &ResidentKernelBindRequest<'_>,
        layout: &ResidentPortLayout,
    ) -> Result<Self, ResidentKernelBindError> {
        let schema = request
            .schemas
            .get(layout.schema_id)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        if schema.body() != &SchemaBody::String
            || schema.key() != layout.schema_key
            || layout.shape != ResidentShape::SCALAR
            || (layout.kind == ResidentValueKind::Snapshot
                && !layout.shape_instance.parameter_values().is_empty())
            || !matches!(
                layout.kind,
                ResidentValueKind::String | ResidentValueKind::Snapshot
            )
        {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        Ok(Self {
            schema: layout.schema_id,
            key: layout.schema_key,
            kind: layout.kind,
        })
    }
    pub(super) fn text<'a>(
        &self,
        input: ResidentValueRef<'a>,
    ) -> Result<&'a str, ResidentKernelError> {
        if input.kind() != self.kind {
            return Err(ResidentKernelError::InvalidInput);
        }
        match input {
            ResidentValueRef::String([text]) => Ok(text),
            ResidentValueRef::Snapshot([Some(value)])
                if value.schema() == self.schema && value.schema_key() == self.key =>
            {
                let ValueData::String(text) = value.data() else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                Ok(text)
            }
            _ => Err(ResidentKernelError::InvalidInput),
        }
    }
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
        || request
            .inputs
            .iter()
            .chain(core::iter::once(&request.output))
            .any(|layout| {
                request
                    .schemas
                    .get(layout.schema_id)
                    .map(|schema| schema.key())
                    != Some(layout.schema_key)
            })
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
pub(super) fn bind_lines(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate(request, Operation::Lines)?;
    let source = &request.inputs[0];
    let source = ScalarTextPlan::bind(request, source)?;
    let output = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if !primitives::is_string_matrix(output.body())
        || dimensions(output.body(), &request.output.shape_instance).map(|shape| shape.0) != Some(1)
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(execute_lines, Box::new([]))
        .with_retained_state(Arc::new(source))
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
pub(super) fn bind_join(
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
    let separator = ScalarTextPlan::bind(request, separator)?;
    let target = ScalarTextPlan::bind(request, &request.output)?;
    if !primitives::is_string_matrix(schema.body())
        || !matches!(
            source.kind,
            ResidentValueKind::String | ResidentValueKind::Snapshot
        )
        || (source.kind == ResidentValueKind::Snapshot && source.shape != ResidentShape::SCALAR)
        || (source.kind == ResidentValueKind::String
            && (source.shape.rows as usize, source.shape.columns as usize) != shape)
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
            separator,
            output_kind: target.kind,
        }))
        .with_snapshot_schemas(request.schemas.clone())
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: Some(1),
            maximum_cardinality: Some(1),
        })
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

/// Shared geometry for the ordinary constructor and a compiler-private view.
/// The caller must admit the delimiter scan and declared-shape solver first.
pub(in crate::resident) fn line_shape(
    kernel: &BoundResidentKernel,
    text: &str,
) -> Result<(usize, ShapeInstance), ResidentKernelError> {
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let count = primitives::line_count(text);
    let shape = mech_core::shape_for_resolved_extents(schema, &[1, budget::checked_u64(count)?])
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    Ok((count, shape))
}
fn execute_lines(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let _work_scope = budget::ControlWorkScopeGuard::enter();
    let text = kernel
        .retained_state::<ScalarTextPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?
        .text(inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?)?;
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
    let (count, shape) = line_shape(kernel, text)?;
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
    let staging_bytes = count
        .checked_mul(core::mem::size_of::<Box<str>>())
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
        temporary_bytes: prospective.retained_bytes.checked_add(budget::checked_u64(staging_bytes)?).and_then(|bytes| bytes.checked_add(shape_scratch)).ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: text.len().checked_mul(2).ok_or(ResidentKernelError::InvalidShape)?, retained_nodes: prospective.node_count.checked_mul(2).ok_or(ResidentKernelError::InvalidShape)?,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let constructor =
        StringMatrixSnapshotConstructor::bind(metadata.schema, shape, Arc::clone(schemas))
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let mut lines = Vec::new();
    lines
        .try_reserve_exact(count)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    for line in text.lines() {
        lines.push(copy_text(line)?.into_boxed_str());
    }
    // Same canonical String sequence, built directly from its already-paid
    // UTF-8 owners instead of a large generic draft array and repacking pass.
    let next = constructor
        .construct(lines.into_boxed_slice())
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
/// Common append mechanics for ordinary TextJoin and a proved native sink.
/// The caller MUST admit/reserve complete capacity before either push. Ordinary
/// TextJoin already computes/reserves its whole payload; the selected native
/// wrapper checks each incremental prefix before entering these same mechanics.
pub(crate) fn append_joined(target: &mut String, separator: &str, has_previous: bool, value: &str) {
    if has_previous {
        target.push_str(separator);
    }
    target.push_str(value);
}
fn execute_join(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let _work_scope = budget::ControlWorkScopeGuard::enter();
    let strings = texts(
        kernel,
        inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?,
    )?;
    let plan = kernel
        .retained_state::<TextPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let separator = plan
        .separator
        .text(inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?)?;
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let previous = match &output {
        ResidentValueMut::String([target]) if plan.output_kind == ResidentValueKind::String => {
            Some(target.as_str())
        }
        ResidentValueMut::Snapshot([target]) if plan.output_kind == ResidentValueKind::Snapshot => {
            match target.as_ref() {
                Some(value)
                    if value.schema() == metadata.schema
                        && value.schema_key() == metadata.schema_key =>
                {
                    let ValueData::String(text) = value.data() else {
                        return Err(ResidentKernelError::InvalidOutput);
                    };
                    Some(text.as_ref())
                }
                Some(_) => return Err(ResidentKernelError::InvalidOutput),
                None => None,
            }
        }
        _ => return Err(ResidentKernelError::InvalidOutput),
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
    // String equality first compares lengths. Unequal lengths are decided
    // without traversing either payload; equal lengths may compare every byte.
    let comparison = previous.map_or(0, |target| {
        if target.len() == payload {
            payload.max(1)
        } else {
            1
        }
    });
    let canonical = plan.output_kind == ResidentValueKind::Snapshot;
    let finalization = if canonical {
        payload
            .checked_mul(8)
            .and_then(|n| n.checked_add(32))
            .ok_or(ResidentKernelError::InvalidShape)?
    } else {
        0
    };
    let footprint = if canonical {
        Some(
            prospective_string_value_footprint(
                metadata.shape.parameter_values().len(),
                false,
                1,
                budget::checked_u64(payload)?,
            )
            .map_err(|_| ResidentKernelError::InvalidShape)?,
        )
    } else {
        None
    };
    let output_bytes = match footprint {
        Some(footprint) => footprint.retained_bytes,
        None => budget::checked_u64(
            payload
                .checked_add(core::mem::size_of::<String>())
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?,
    };
    let temporary = if canonical {
        output_bytes
            .checked_add(
                budget::checked_u64(payload)?
                    .checked_mul(2)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )
            .and_then(|n| n.checked_add(core::mem::size_of::<ValueDataDraft>() as u64))
            .ok_or(ResidentKernelError::InvalidShape)?
    } else {
        budget::checked_u64(payload)?
    };
    let retained_nodes = match footprint {
        Some(footprint) => footprint
            .node_count
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or(ResidentKernelError::InvalidShape)?,
        None => 1,
    };
    let compute = scan
        .checked_add(
            payload
                .checked_mul(4)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
        .and_then(|work| work.checked_add(comparison))
        .and_then(|work| work.checked_add(finalization))
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedKernel::new((), budget::resident_cost! {
        compute_work: compute, comparison_work: comparison, output_elements: 1,
        output_bytes,
        temporary_bytes: temporary, cloned_bytes: if canonical { payload.checked_mul(2).ok_or(ResidentKernelError::InvalidShape)? } else { payload },
        retained_nodes,
        ..budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let mut joined = String::new();
    joined
        .try_reserve_exact(payload)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    for index in 0..strings.len() {
        append_joined(&mut joined, separator, index != 0, strings.get(index));
    }
    let changed = previous != Some(joined.as_str());
    match output {
        ResidentValueMut::String([target]) => {
            if changed {
                *target = joined;
            }
        }
        ResidentValueMut::Snapshot([target]) => {
            if changed {
                let schemas = kernel
                    .shared_snapshot_schemas()
                    .ok_or(ResidentKernelError::InvalidOutput)?;
                let canonical =
                    SnapshotCanonicalizationBudget::new(budget::checked_u64(finalization)?);
                let next = ValueDraft {
                    schema: metadata.schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::String(joined),
                }
                .finalize(
                    &SnapshotValidationContext::with_shared_schemas(schemas)
                        .with_canonicalization_budget(&canonical),
                )
                .map_err(|_| ResidentKernelError::InvalidOutput)?;
                *target = Some(next);
            }
        }
        _ => return Err(ResidentKernelError::InvalidOutput),
    }
    Ok(changed)
}

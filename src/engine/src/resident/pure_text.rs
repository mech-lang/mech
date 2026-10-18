use std::sync::Arc;

use mech_core::snapshot::{
    F64Bits, SnapshotCanonicalizationBudget, SnapshotValidationContext, SnapshotValueError,
};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FloatWidth, FunctionCatalogBuilder, ImplementationMemoryClass,
    IntegerWidth, MResult, OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest,
    ResidentKernelError, ResidentKernelExecutor, ResidentKernelInputs,
    ResidentKernelWorkAccounting, ResidentPortLayout, ResidentShape, ResidentSnapshotOutput,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResolvedOperationContract, SchemaBody,
    SchemaId, SchemaKey, ShapeInstance, ShapeRule, Value, ValueData, ValueDataDraft, ValueDraft,
};

use crate::intrinsics::pure_text as primitives;

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(
        ["number"],
        "parse-u64",
        ImplementationMemoryClass::CanonicalFinalize,
        bind_parse_u64,
    )?;
    super::native_collection_semantics::register_format_and_length(builder)
}

fn validate_full_write(
    request: &ResidentKernelBindRequest<'_>,
    arity: usize,
    shape: ShapeRule,
    change: ChangeDetectionPolicy,
) -> Result<(), ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [output] = contract.outputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if request.inputs.len() != arity
        || contract.inputs.len() != arity
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
        || output.construction != (OutputConstruction::FullWrite { shape })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != change
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(())
}

fn schema_body<'a>(
    request: &'a ResidentKernelBindRequest<'_>,
    layout: &ResidentPortLayout,
) -> Result<&'a SchemaBody, ResidentKernelBindError> {
    request
        .schemas
        .get(layout.schema_id)
        .map(|schema| schema.body())
        .ok_or(ResidentKernelBindError::UnsupportedLayout)
}

fn matrix_dimensions(body: &SchemaBody, shape: &ShapeInstance) -> Option<(usize, usize)> {
    let SchemaBody::Matrix { dimensions, .. } = body else {
        return None;
    };
    let [rows, columns] = dimensions.as_ref() else {
        return None;
    };
    let resolve = |dimension| {
        shape
            .resolve_dimension(dimension)
            .ok()
            .and_then(|value| usize::try_from(value).ok())
    };
    Some((resolve(rows)?, resolve(columns)?))
}

#[derive(Clone, Debug)]
struct InputPlan {
    schema: SchemaId,
    key: SchemaKey,
    body: SchemaBody,
    kind: ResidentValueKind,
    dimensions: Option<(usize, usize)>,
    fixed_shape: bool,
}

impl InputPlan {
    fn bind(
        request: &ResidentKernelBindRequest<'_>,
        layout: &ResidentPortLayout,
    ) -> Result<Self, ResidentKernelBindError> {
        let body = schema_body(request, layout)?;
        let dimensions = if matches!(body, SchemaBody::Matrix { .. }) {
            Some(
                matrix_dimensions(body, &layout.shape_instance)
                    .ok_or(ResidentKernelBindError::UnsupportedLayout)?,
            )
        } else {
            None
        };
        let element = match body {
            SchemaBody::Matrix { element, .. } => element.as_ref(),
            body => body,
        };
        let ordinary_kind = match element {
            SchemaBody::Bool => ResidentValueKind::Bool,
            SchemaBody::Index => ResidentValueKind::Index,
            SchemaBody::FloatingPoint(FloatWidth::W64) => ResidentValueKind::F64,
            SchemaBody::String => ResidentValueKind::String,
            _ => ResidentValueKind::Snapshot,
        };
        if layout.kind == ResidentValueKind::Snapshot {
            if layout.shape != ResidentShape::SCALAR
                || (dimensions.is_none() && ordinary_kind != ResidentValueKind::Snapshot)
            {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            }
        } else {
            let (rows, columns) = dimensions.unwrap_or((1, 1));
            if layout.kind != ordinary_kind
                || usize::try_from(layout.shape.rows).ok() != Some(rows)
                || usize::try_from(layout.shape.columns).ok() != Some(columns)
            {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            }
        }
        Ok(Self {
            schema: layout.schema_id,
            key: layout.schema_key,
            body: body.clone(),
            kind: layout.kind,
            dimensions,
            fixed_shape: layout.activation_fixed_shape
                || request
                    .schemas
                    .get(layout.schema_id)
                    .is_some_and(|schema| schema.dimension_parameters().is_empty()),
        })
    }

    fn snapshot<'a>(&self, input: ResidentValueRef<'a>) -> Result<&'a Value, ResidentKernelError> {
        let ResidentValueRef::Snapshot([Some(value)]) = input else {
            return Err(ResidentKernelError::InvalidInput);
        };
        if self.kind != ResidentValueKind::Snapshot
            || value.schema() != self.schema
            || value.schema_key() != self.key
        {
            return Err(ResidentKernelError::InvalidInput);
        }
        Ok(value)
    }

    fn geometry(&self, input: ResidentValueRef<'_>) -> Result<(usize, usize), ResidentKernelError> {
        if input.kind() != self.kind {
            return Err(ResidentKernelError::InvalidInput);
        }
        if self.kind == ResidentValueKind::Snapshot {
            let value = self.snapshot(input)?;
            if let Some(declared) = self.dimensions {
                let actual = matrix_dimensions(&self.body, value.shape())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let ValueData::Matrix(matrix) = value.data() else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                if (self.fixed_shape && actual != declared)
                    || actual.0.checked_mul(actual.1) != Some(matrix.elements().len())
                {
                    return Err(ResidentKernelError::InvalidShape);
                }
                return Ok(actual);
            }
            if matches!(value.data(), ValueData::Matrix(_)) {
                return Err(ResidentKernelError::InvalidInput);
            }
            return Ok((1, 1));
        }
        let dimensions = self.dimensions.unwrap_or((1, 1));
        if dimensions.0.checked_mul(dimensions.1) != Some(input.len()) {
            return Err(ResidentKernelError::InvalidShape);
        }
        Ok(dimensions)
    }
}

#[derive(Clone, Debug)]
struct PureTextPlan {
    source: InputPlan,
    precision: Option<InputPlan>,
    dense_output_geometry: Option<(usize, usize)>,
}

fn bound(
    request: &ResidentKernelBindRequest<'_>,
    executor: ResidentKernelExecutor,
    plan: PureTextPlan,
) -> BoundResidentKernel {
    // Scalar sources have no unadmitted recursive input validation scan.
    // Matrix-source execution keeps its previous policy until its complete
    // validation/geometry pass has its own metered contract.
    let accounting = if plan.source.dimensions.is_none() {
        ResidentKernelWorkAccounting::RuntimeMetered
    } else {
        ResidentKernelWorkAccounting::Unclassified
    };
    BoundResidentKernel::new(executor, Box::new([]))
        .with_retained_state(Arc::new(plan))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone())
        .with_work_accounting(accounting)
}

fn is_u64(body: &SchemaBody) -> bool {
    body == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
}

fn validate_u64_output(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<(), ResidentKernelBindError> {
    if !is_u64(schema_body(request, &request.output)?)
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(())
}

fn bind_parse_u64(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    validate_u64_output(request)?;
    let source = InputPlan::bind(request, &request.inputs[0])?;
    if source.body != SchemaBody::String {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        request,
        execute_parse_u64,
        PureTextPlan {
            source,
            precision: None,
            dense_output_geometry: None,
        },
    ))
}

pub(super) fn bind_length(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_full_write(
        request,
        1,
        ShapeRule::Declared,
        ChangeDetectionPolicy::ExactScalar,
    )?;
    validate_u64_output(request)?;
    let source = InputPlan::bind(request, &request.inputs[0])?;
    if source.body != SchemaBody::String && source.dimensions.is_none() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(bound(
        request,
        execute_length,
        PureTextPlan {
            source,
            precision: None,
            dense_output_geometry: None,
        },
    ))
}

pub(super) fn bind_format(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let Some(input) = request.inputs.first() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let source = InputPlan::bind(request, input)?;
    if !primitives::format_schema_supported(&source.body) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    validate_full_write(
        request,
        2,
        ShapeRule::SameAsInput { input: 0 },
        if source.dimensions.is_some() {
            ChangeDetectionPolicy::KernelReported
        } else {
            ChangeDetectionPolicy::ExactScalar
        },
    )?;
    let precision = InputPlan::bind(request, &request.inputs[1])?;
    if !is_u64(&precision.body) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let output_body = schema_body(request, &request.output)?;
    let dense_output_geometry = if let Some(dimensions) = source.dimensions {
        let SchemaBody::Matrix { element, .. } = output_body else {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        };
        if element.as_ref() != &SchemaBody::String
            || matrix_dimensions(output_body, &request.output.shape_instance) != Some(dimensions)
        {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        if request.output.kind == ResidentValueKind::Snapshot
            && request.output.shape == ResidentShape::SCALAR
        {
            None
        } else if request.output.kind == ResidentValueKind::String
            && usize::try_from(request.output.shape.rows).ok() == Some(dimensions.0)
            && usize::try_from(request.output.shape.columns).ok() == Some(dimensions.1)
        {
            Some(dimensions)
        } else {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
    } else {
        if output_body != &SchemaBody::String
            || request.output.kind != ResidentValueKind::String
            || request.output.shape != ResidentShape::SCALAR
        {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        Some((1, 1))
    };
    Ok(bound(
        request,
        execute_format,
        PureTextPlan {
            source,
            precision: Some(precision),
            dense_output_geometry,
        },
    ))
}

fn plan(kernel: &BoundResidentKernel) -> Result<&PureTextPlan, ResidentKernelError> {
    kernel
        .retained_state()
        .ok_or(ResidentKernelError::InvalidInput)
}

fn admit_work(work: usize, temporary_bytes: usize) -> Result<(), ResidentKernelError> {
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work: work,
            temporary_bytes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn finalize(
    kernel: &BoundResidentKernel,
    shape: &ShapeInstance,
    data: ValueDataDraft,
    work: u64,
) -> Result<Value, ResidentKernelError> {
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .shared_snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let budget = SnapshotCanonicalizationBudget::new(work);
    let context = SnapshotValidationContext::with_shared_schemas(schemas)
        .with_canonicalization_budget(&budget);
    let next = ValueDraft {
        schema: metadata.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&context)
    .map_err(|error| match error {
        SnapshotValueError::CanonicalizationWorkLimitExceededV1 { .. } => {
            ResidentKernelError::InvalidShape
        }
        _ => ResidentKernelError::InvalidOutput,
    })?;
    if next.schema_key() != metadata.schema_key {
        return Err(ResidentKernelError::InvalidOutput);
    }
    Ok(next)
}

fn publish_u64(
    kernel: &BoundResidentKernel,
    output: ResidentValueMut<'_>,
    value: u64,
    work: usize,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut meter = super::budget::ResidentBudgetMeter::default();
    let data = super::budget::measure_canonical_data_footprint(
        &mut meter,
        &SchemaBody::UnsignedInteger(IntegerWidth::W64),
        &ValueData::U64(value),
    )?;
    let footprint = super::budget::projected_canonical_value_footprint(
        data,
        metadata.shape.parameter_values().len(),
    )?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: 1,
            compute_work: work.checked_add(2).ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: 1,
            output_bytes: footprint.retained_bytes,
            temporary_bytes: footprint.retained_bytes,
            retained_nodes: footprint.node_count,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let changed = match target.as_ref() {
        Some(current) if current.schema_key() == metadata.schema_key => {
            let ValueData::U64(current) = current.data() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            *current != value
        }
        Some(_) => return Err(ResidentKernelError::InvalidOutput),
        None => true,
    };
    let next = finalize(kernel, &metadata.shape, ValueDataDraft::U64(value), 1)?;
    *target = Some(next);
    Ok(changed)
}

fn execute_parse_u64(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([text])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let work = text
        .len()
        .checked_mul(2)
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_work(work, 0)?;
    let value = primitives::parse_u64(text).map_err(|_| ResidentKernelError::InvalidInput)?;
    publish_u64(kernel, output, value, work)
}

fn execute_length(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let source = &plan(kernel)?.source;
    let input = inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?;
    let (value, work) = if source.body == SchemaBody::String {
        let ResidentValueRef::String([text]) = input else {
            return Err(ResidentKernelError::InvalidInput);
        };
        admit_work(text.len(), 0)?;
        (
            primitives::grapheme_length(text).map_err(|_| ResidentKernelError::InvalidInput)?,
            text.len(),
        )
    } else {
        let (rows, columns) = source.geometry(input)?;
        let count = rows
            .checked_mul(columns)
            .ok_or(ResidentKernelError::InvalidShape)?;
        (super::budget::checked_u64(count)?, 1)
    };
    publish_u64(kernel, output, value, work)
}

#[derive(Default)]
struct ByteCounter(usize);

impl core::fmt::Write for ByteCounter {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        self.0 = self.0.checked_add(text.len()).ok_or(core::fmt::Error)?;
        Ok(())
    }
}

fn numeric_at(
    source: &InputPlan,
    input: ResidentValueRef<'_>,
    dimensions: (usize, usize),
    ordinal: usize,
) -> Result<primitives::NumericTextValue, ResidentKernelError> {
    match input {
        ResidentValueRef::F64(values) => {
            let lane = if source.dimensions.is_some() {
                (ordinal % dimensions.1) * dimensions.0 + ordinal / dimensions.1
            } else {
                ordinal
            };
            let value = values.get(lane).ok_or(ResidentKernelError::InvalidInput)?;
            primitives::numeric_at(&ValueData::F64(F64Bits::from_f64(*value)), 0)
                .map_err(|_| ResidentKernelError::InvalidInput)
        }
        ResidentValueRef::Snapshot(_) => {
            primitives::numeric_at(source.snapshot(input)?.data(), ordinal)
                .map_err(|_| ResidentKernelError::InvalidInput)
        }
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn execute_format(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = plan(kernel)?;
    let input = inputs.get(0).ok_or(ResidentKernelError::InvalidInput)?;
    let dimensions = plan.source.geometry(input)?;
    let count = dimensions
        .0
        .checked_mul(dimensions.1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let precision_input = inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?;
    let precision_value = plan
        .precision
        .as_ref()
        .ok_or(ResidentKernelError::InvalidInput)?
        .snapshot(precision_input)?;
    let ValueData::U64(precision) = precision_value.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    primitives::validate_format_precision(&plan.source.body, *precision)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    if plan
        .dense_output_geometry
        .is_some_and(|expected| expected != dimensions)
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    match &output {
        ResidentValueMut::String(values)
            if plan.dense_output_geometry.is_some() && values.len() == count => {}
        ResidentValueMut::Snapshot([_]) if plan.dense_output_geometry.is_none() => {}
        _ => return Err(ResidentKernelError::InvalidOutput),
    }
    // At precision <= 12, even the largest finite f64 emits fewer than 512
    // bytes. Reserve both formatting passes before asking the shared writer
    // for exact lengths; integers retain their original signed/unsigned width.
    let format_work = count
        .checked_mul(1024)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let length_bytes = count
        .checked_mul(core::mem::size_of::<usize>())
        .and_then(|bytes| bytes.checked_add(core::mem::size_of::<Vec<usize>>()))
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_work(format_work, length_bytes)?;
    let canonical_ordinal = |ordinal: usize| {
        if plan.dense_output_geometry.is_some() && plan.source.dimensions.is_some() {
            (ordinal % dimensions.0) * dimensions.1 + ordinal / dimensions.0
        } else {
            ordinal
        }
    };
    let mut lengths = Vec::with_capacity(count);
    let mut payload = 0usize;
    for ordinal in 0..count {
        let value = numeric_at(&plan.source, input, dimensions, canonical_ordinal(ordinal))?;
        let mut counter = ByteCounter::default();
        primitives::write_number(&mut counter, value, *precision)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        payload = payload
            .checked_add(counter.0)
            .ok_or(ResidentKernelError::InvalidShape)?;
        lengths.push(counter.0);
    }
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut meter = super::budget::ResidentBudgetMeter::default();
    let (output_bytes, nodes, comparison_work) = match &output {
        ResidentValueMut::String(current) => {
            let current_payload = current.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(value.len())
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
            let bytes = count
                .checked_mul(core::mem::size_of::<String>())
                .and_then(|bytes| bytes.checked_add(payload))
                .ok_or(ResidentKernelError::InvalidShape)?;
            (
                super::budget::checked_u64(bytes)?,
                super::budget::checked_u64(count)?,
                super::budget::checked_u64(
                    current_payload
                        .checked_add(payload)
                        .and_then(|work| work.checked_add(count))
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )?,
            )
        }
        ResidentValueMut::Snapshot([current]) => {
            let next = mech_core::snapshot::prospective_string_value_footprint(
                metadata.shape.parameter_values().len(),
                true,
                super::budget::checked_u64(count)?,
                super::budget::checked_u64(payload)?,
            )
            .map_err(|_| ResidentKernelError::InvalidShape)?;
            let comparison = if let Some(current) = current {
                let old =
                    super::budget::published_canonical_footprint(&mut meter, current, schemas)?;
                super::budget::projected_language_equality_work(
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
            (next.retained_bytes, next.node_count, comparison)
        }
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    let shape_bytes = metadata
        .shape
        .parameter_values()
        .len()
        .checked_mul(core::mem::size_of::<u64>())
        .and_then(|bytes| bytes.checked_mul(3))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let staged_bytes = count
        .checked_mul(core::mem::size_of::<String>() + core::mem::size_of::<ValueDataDraft>())
        .and_then(|bytes| bytes.checked_add(length_bytes))
        .and_then(|bytes| bytes.checked_add(core::mem::size_of::<Vec<String>>()))
        .and_then(|bytes| bytes.checked_add(core::mem::size_of::<Vec<ValueDataDraft>>()))
        .and_then(|bytes| bytes.checked_add(shape_bytes))
        .and_then(|bytes| payload.checked_mul(2)?.checked_add(bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let finalization_work = count
        .checked_add(payload)
        .and_then(|work| work.checked_add(1))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let finalization_work = super::budget::checked_u64(finalization_work)?;
    let staged_nodes = super::budget::checked_u64(count)?
        .checked_mul(2)
        .and_then(|nodes| nodes.checked_add(4))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let measured = meter.estimate();
    super::budget::PreparedKernel::new((), super::budget::resident_cost! {
        comparison_work: measured.comparison_work().checked_add(comparison_work).ok_or(ResidentKernelError::InvalidShape)?,
        compute_work: super::budget::checked_u64(format_work)?.checked_add(comparison_work)
            .and_then(|work| work.checked_add(finalization_work))
            .and_then(|work| work.checked_add(measured.compute_work())).ok_or(ResidentKernelError::InvalidShape)?,
        output_elements: count,
        output_bytes,
        temporary_bytes: super::budget::checked_u64(staged_bytes)?.checked_add(output_bytes).ok_or(ResidentKernelError::InvalidShape)?,
        cloned_bytes: payload,
        retained_nodes: nodes.checked_add(staged_nodes).ok_or(ResidentKernelError::InvalidShape)?,
        ..super::budget::KernelCostEstimate::default()
    }).admit()?.into_plan();
    let mut candidate = Vec::with_capacity(count);
    for (ordinal, length) in lengths.into_iter().enumerate() {
        let value = numeric_at(&plan.source, input, dimensions, canonical_ordinal(ordinal))?;
        let mut text = String::with_capacity(length);
        primitives::write_number(&mut text, value, *precision)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        candidate.push(text);
    }
    match output {
        ResidentValueMut::String(current) => {
            let changed = current.iter().ne(candidate.iter());
            for (target, next) in current.iter_mut().zip(candidate) {
                *target = next;
            }
            Ok(changed)
        }
        ResidentValueMut::Snapshot([target]) => {
            let schema = schemas
                .get(metadata.schema)
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let rows = super::budget::checked_u64(dimensions.0)?;
            let columns = super::budget::checked_u64(dimensions.1)?;
            let shape = mech_core::shape_for_resolved_extents(schema, &[rows, columns])
                .map_err(|_| ResidentKernelError::InvalidShape)?;
            let data = ValueDataDraft::Matrix(
                candidate
                    .into_iter()
                    .map(ValueDataDraft::String)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            );
            let next = finalize(kernel, &shape, data, finalization_work)?;
            let changed = match target.as_ref() {
                Some(current) => !current
                    .snapshot_eq(schemas, &next, schemas)
                    .map_err(|_| ResidentKernelError::InvalidOutput)?,
                None => true,
            };
            *target = Some(next);
            Ok(changed)
        }
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Inputs<'a>(&'a [ResidentValueRef<'a>]);
    impl ResidentKernelInputs for Inputs<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index).copied()
        }
    }

    #[test]
    fn scalar_results_share_a_large_schema_arena_under_a_small_turn_budget() {
        use mech_core::{
            AllocationPlan, AllocationRole, ArenaPlacement, DeclaredOperationContract,
            DimensionExpr, MemoryArenaId, MemoryLifetime, MemoryObjectId, MemoryObjectOwner,
            MemoryPlanPoint, MemorySpace, NodeId, ResolvedInputPort, ResolvedOutputPort,
            SchemaDraft, SchemaTableBuilder, TargetMemoryProfile,
        };

        let mut builder = SchemaTableBuilder::new();
        let mut add = |body| {
            builder
                .insert(
                    SchemaDraft {
                        dimension_parameters: Box::new([]),
                        body,
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap()
        };
        let text = add(SchemaBody::String);
        let integer = add(SchemaBody::UnsignedInteger(IntegerWidth::W64));
        for columns in 0..512 {
            add(SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: Box::new([
                    DimensionExpr::Constant(1),
                    DimensionExpr::Constant(columns),
                ]),
            });
        }
        let build = builder.finish().unwrap();
        let text = build.resolve(text).unwrap();
        let integer = build.resolve(integer).unwrap();
        let (schemas, _) = build.into_parts();
        let arena_clone = schemas.clone_allocation_bound_bytes().unwrap();
        assert!(
            arena_clone > 4096,
            "the unrelated arena must exceed the turn allowance"
        );
        let layout = |schema_id, kind| {
            let schema = schemas.get(schema_id).unwrap();
            ResidentPortLayout {
                schema_id,
                schema_key: schema.key(),
                kind,
                shape: ResidentShape::SCALAR,
                shape_instance: schema.instantiate_shape(Box::new([])).unwrap(),
                activation_fixed_shape: true,
                resolved_selector: None,
            }
        };
        let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: Box::new([ResolvedInputPort {
                schema: text,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]),
            outputs: Box::new([ResolvedOutputPort {
                schema: integer,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::ExactScalar,
            }]),
            interaction: ExternalInteraction::Pure,
        });
        let kernel = bind_parse_u64(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[layout(text, ResidentValueKind::String)],
            output: layout(integer, ResidentValueKind::Snapshot),
        })
        .unwrap();
        assert_eq!(
            kernel.work_accounting(),
            ResidentKernelWorkAccounting::RuntimeMetered
        );
        let owner = kernel.shared_snapshot_schemas().unwrap().clone();
        assert!(Arc::ptr_eq(
            &owner,
            kernel.clone().shared_snapshot_schemas().unwrap()
        ));

        let node = NodeId::new(0);
        let mut limits = TargetMemoryProfile::current_resident_cpu().unwrap().limits;
        limits.max_output_bytes = Some(4096);
        limits.max_temporary_bytes = Some(4096);
        limits.max_cloned_bytes = Some(4096);
        let program = crate::memory_planner::ProgramMemoryPlan {
            allocations: Box::new([AllocationPlan {
                id: MemoryObjectId::new(0),
                owner: MemoryObjectOwner::TransactionStage { node, output: 0 },
                role: AllocationRole::TransactionStage,
                slot: None,
                space: MemorySpace::ResidentCpu,
                current_bytes: 0,
                capacity_bytes: 0,
                payload_block_capacity: 0,
                alignment: 1,
                lifetime: MemoryLifetime::Transaction {
                    first: MemoryPlanPoint::new(0),
                    last: MemoryPlanPoint::new(1),
                },
                placement: ArenaPlacement {
                    arena: MemoryArenaId::new(0),
                    offset: 0,
                },
                reuse_group: None,
            }]),
            budget_limits: limits,
            ..crate::memory_planner::ProgramMemoryPlan::default()
        };
        let turn = crate::memory_planner::plan_turn_memory(
            &program,
            node,
            &crate::memory_planner::TurnMemoryFacts::default(),
        )
        .unwrap();
        super::super::budget::with_resident_turn_plan(turn, || -> Result<(), ResidentKernelError> {
            // A real copy of the arena would exceed the admitted workspace.
            assert!(super::super::budget::PreparedKernel::new((),
                super::super::budget::resident_cost! {
                    temporary_bytes: arena_clone, cloned_bytes: arena_clone,
                    ..super::super::budget::KernelCostEstimate::default()
                }).admit().is_err());
            let mut output = [None];
            for text in ["19", "18446744073709551615", "0"] {
                let input = [text.to_owned()];
                kernel.execute(&Inputs(&[ResidentValueRef::String(&input)]),
                    ResidentValueMut::Snapshot(&mut output)).unwrap();
                let value = output[0].as_ref().unwrap();
                assert!(matches!(value.data(), ValueData::U64(actual) if *actual == text.parse::<u64>().unwrap()));
                assert!(Arc::ptr_eq(&owner, &value.schemas().unwrap()),
                    "each result must retain the existing owner without cloning the arena");
            }
            Ok(())
        }).unwrap();
    }
}

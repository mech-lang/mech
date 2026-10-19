//! Shape-changing conversion selects the shared core conversion plan. All
//! draft/key/schema staging and bounded sort work are admitted before copying.
use super::budget::{
    PreparedKernel, ResidentBudgetMeter, checked_cost_product as mul, checked_cost_sum as sum,
    checked_u64,
};
use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
use mech_core::*;
use std::sync::Arc;

pub(super) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(any(feature = "pure_text", feature = "map_ops"))]
    return super::native_collection_semantics::register_set_conversion(builder);
    #[cfg(not(any(feature = "pure_text", feature = "map_ops")))]
    builder.insert_resident_factory(
        ["convert"],
        "set",
        ImplementationMemoryClass::CanonicalSortUnique,
        bind,
    )
}

struct Plan {
    conversion: ConversionPlan,
    source: SchemaBody,
    source_key: SchemaKey,
    dense_shape: ResidentShape,
    metadata_bytes: u64,
}

pub(super) fn bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let ResolvedOperationContract::Declared(actual) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let expected = maintained_operation_contract("convert/set", 1, false).unwrap();
    let inputs = expected
        .inputs
        .resolve(1)
        .map_err(|_| ResidentKernelBindError::UnsupportedContract)?;
    if actual.inputs.len() != 1
        || actual.outputs.len() != 1
        || actual.interaction != expected.interaction
        || actual.inputs[0].schema != input.schema_id
        || actual.inputs[0].access != inputs[0].access
        || actual.inputs[0].delivery != inputs[0].delivery
        || actual.outputs[0].schema != request.output.schema_id
        || actual.outputs[0].access != expected.outputs[0].access
        || actual.outputs[0].delivery != expected.outputs[0].delivery
        || actual.outputs[0].construction != expected.outputs[0].construction
        || actual.outputs[0].alias != expected.outputs[0].alias
        || actual.outputs[0].change_detection != expected.outputs[0].change_detection
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let source = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let target = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if !matches!(source.body(), SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2)
        || !matches!(target.body(), SchemaBody::Set { .. })
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let source_type = ResolvedType::from_schema(source, &input.shape_instance)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let target_type = ResolvedType::from_schema(target, &request.output.shape_instance)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let conversion = plan_explicit_cast(&source_type, &target_type)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    if !matches!(conversion.step, ConversionStep::MatrixToSet(_)) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    // The temporary one-element schema/table builder derives from the target
    // element, not a clone of the entire application schema arena. Include
    // canonical byte buffers, descriptor normalization and vector growth.
    let metadata_bytes = target
        .body()
        .clone_allocation_bound_bytes()
        .and_then(|n| n.checked_mul(32))
        .and_then(|n| n.checked_add(4096))
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(BoundResidentKernel::new(execute, Box::new([]))
        .with_retained_state(Arc::new(Plan {
            conversion,
            source: source.body().clone(),
            source_key: source.key(),
            dense_shape: input.shape,
            metadata_bytes,
        }))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone()))
}

fn execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(input) = inputs.get(0).filter(|_| inputs.len() == 1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let plan = kernel
        .retained_state::<Plan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let mut meter = ResidentBudgetMeter::default();
    let (count, input_bytes, input_nodes, maximum_string) = match input {
        ResidentValueRef::Snapshot([Some(value)]) => {
            if value.schema_key() != plan.source_key {
                return Err(ResidentKernelError::InvalidInput);
            }
            let ValueData::Matrix(matrix) = value.data() else {
                return Err(ResidentKernelError::InvalidInput);
            };
            let footprint =
                super::budget::measure_canonical_value_footprint(&mut meter, value, schemas)?;
            meter.charge_compute_work(checked_u64(matrix.elements().len())?)?;
            let longest = match matrix.elements() {
                mech_core::snapshot::SequenceView::String(strings) => {
                    strings.iter().map(|s| s.len() as u64).max().unwrap_or(0)
                }
                _ => footprint.retained_bytes,
            };
            (
                matrix.elements().len(),
                footprint.retained_bytes,
                footprint.node_count,
                longest,
            )
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
        ResidentValueRef::String(values) => {
            meter.charge_compute_work(mul(&[checked_u64(values.len())?, 2])?)?;
            let bytes = values
                .iter()
                .try_fold(0u64, |n, value| n.checked_add(value.len() as u64))
                .ok_or(ResidentKernelError::InvalidShape)?;
            (
                values.len(),
                bytes,
                values.len() as u64,
                values.iter().map(|s| s.len() as u64).max().unwrap_or(0),
            )
        }
        values => (values.len(), mul(&[checked_u64(values.len())?, 16])?, 1, 0),
    };
    let n = checked_u64(count)?;
    let previous = target
        .as_ref()
        .map(|v| super::budget::measure_canonical_value_footprint(&mut meter, v, schemas))
        .transpose()?;
    let output_schema = schemas
        .get(
            kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?
                .schema,
        )
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let SchemaBody::Set { element, .. } = output_schema.body() else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let SchemaBody::Matrix {
        element: source_element,
        ..
    } = &plan.source
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let string_cast =
        element.as_ref() == &SchemaBody::String && source_element.as_ref() != &SchemaBody::String;
    // 768 bytes covers every maintained scalar Display conversion, including
    // both components of complex values and extreme subnormal spellings.
    let payload = if string_cast {
        mul(&[n, 768])?
    } else {
        input_bytes.max(mul(&[n, 32])?)
    };
    let key_work = match element.as_ref() {
        SchemaBody::String => {
            if string_cast {
                768
            } else {
                maximum_string.max(1)
            }
        }
        SchemaBody::Bool
        | SchemaBody::Index
        | SchemaBody::Id
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_) => 1,
        _ => payload.max(1),
    };
    let comparisons = mul(&[n, 3, u64::from(count.max(1).ilog2()) + 1, key_work])?;
    let staging = mul(&[
        n,
        (3 * core::mem::size_of::<ValueDataDraft>()
            + 2 * core::mem::size_of::<ValueData>()
            + core::mem::size_of::<mech_core::snapshot::CanonicalKeyValue>()) as u64,
    ])?;
    let previous_bytes = previous.map_or(0, |p| p.retained_bytes);
    let publication = sum(&[previous_bytes, payload])?;
    let measured = meter.estimate();
    let cost = super::budget::resident_cost! {
        comparison_work: sum(&[measured.comparison_work(), comparisons, publication])?,
        compute_work: sum(&[measured.compute_work(), comparisons, publication, mul(&[payload, 4])?, mul(&[plan.metadata_bytes, n.checked_add(8).ok_or(ResidentKernelError::InvalidShape)?])?, mul(&[n, 32])?])?,
        output_elements: count,
        output_bytes: sum(&[payload, mul(&[n, core::mem::size_of::<mech_core::snapshot::CanonicalKeyValue>() as u64])?])?,
        temporary_bytes: sum(&[input_bytes, previous_bytes, mul(&[payload, 5])?, staging, plan.metadata_bytes])?,
        cloned_bytes: sum(&[input_bytes, mul(&[payload, 3])?])?,
        retained_nodes: sum(&[mul(&[input_nodes, 4])?, previous.map_or(0, |p| p.node_count), mul(&[n, 4])?, 32])?,
        ..super::budget::KernelCostEstimate::default()
    };
    PreparedKernel::new((), cost).admit()?.into_plan();
    let draft = match input {
        ResidentValueRef::Snapshot([Some(value)]) => value
            .canonical_data_draft()
            .map_err(|_| ResidentKernelError::InvalidInput)?,
        ResidentValueRef::F64(values) => ValueDataDraft::Matrix(
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                Some(ValueDataDraft::F64(F64Bits::from_f64(*v)))
            })
            .ok_or(ResidentKernelError::InvalidShape)?,
        ),
        ResidentValueRef::Bool(values) => ValueDataDraft::Matrix(
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                (*v <= 1).then_some(ValueDataDraft::Bool(*v != 0))
            })
            .ok_or(ResidentKernelError::InvalidInput)?,
        ),
        ResidentValueRef::Index(values) => ValueDataDraft::Matrix(
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                Some(ValueDataDraft::Index(*v))
            })
            .ok_or(ResidentKernelError::InvalidShape)?,
        ),
        ResidentValueRef::String(values) => ValueDataDraft::Matrix(
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                Some(ValueDataDraft::String(v.clone()))
            })
            .ok_or(ResidentKernelError::InvalidShape)?,
        ),
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let data = execute_conversion_draft(draft, &plan.conversion.step)
        .map_err(|_| ResidentKernelError::Arithmetic)?;
    let shape = shape_for_value_data(output_schema, &data, &[], None)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let next = ValueDraft {
        schema: kernel.snapshot_output().unwrap().schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(
        kernel
            .shared_snapshot_schemas()
            .ok_or(ResidentKernelError::InvalidOutput)?,
    ))
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let changed = target
        .as_ref()
        .map(|old| old.snapshot_eq(schemas, &next, schemas).map(|same| !same))
        .transpose()
        .map_err(|_| ResidentKernelError::InvalidOutput)?
        .unwrap_or(true);
    *target = Some(next);
    Ok(changed)
}

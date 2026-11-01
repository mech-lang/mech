//! Shape-changing conversions use the shared schema-aware core executor.
//! Every nested sort and draft/key/shape staging phase is admitted before copying.
use super::budget::{PreparedKernel, ResidentBudgetMeter};
use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
use mech_core::*;
use std::sync::Arc;

#[path = "set_conversion_budget.rs"]
mod cost;
#[path = "set_conversion_stats.rs"]
mod stats;

#[path = "set_conversion_closed.rs"]
mod closed;
#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
pub(super) use closed::closed_scalar_setup_bytes;
pub(crate) use closed::fold_closed;

#[cfg(test)]
thread_local! {
    pub(super) static SOURCE_DRAFTS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

#[cfg(all(
    test,
    feature = "native-artifact",
    any(feature = "full_source", feature = "full_compiler"),
    any(feature = "pure_text", feature = "map_ops")
))]
#[path = "set_conversion_output_tests.rs"]
mod output_tests;

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
    source_schema: SchemaId,
    source_key: SchemaKey,
    source_shape: ShapeInstance,
    dense_shape: ResidentShape,
    metadata: ConversionSchemaStagingBound,
    phases: u64,
}

/// Symbolic semantic planning shared by the binder and native eligibility.
/// No live value, fabricated shape, binder or execution is needed here.
pub(crate) fn conversion_plan(
    source: &Schema,
    target: &Schema,
) -> Result<ConversionPlan, ResidentKernelBindError> {
    fn resolved(schema: &Schema) -> Result<ResolvedType, ResidentKernelBindError> {
        let parameters = schema
            .dimension_parameters()
            .iter()
            .enumerate()
            .map(|(index, p)| DimensionParameterDeclaration {
                id: DimensionParameterId::new(index as u32),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: p.lifetime(),
                lower_bound: p.lower_bound().clone(),
                upper_bound: p.upper_bound().cloned(),
            })
            .collect::<Vec<_>>();
        ResolvedType::from_schema_body(schema.body(), &parameters)
            .map_err(|_| ResidentKernelBindError::UnsupportedLayout)
    }
    let conversion = plan_explicit_cast(&resolved(source)?, &resolved(target)?)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    if !conversion.step.contains_matrix_to_set() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(conversion)
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
    let snapshot =
        input.kind == ResidentValueKind::Snapshot && input.shape == ResidentShape::SCALAR;
    let dense = matches!(source.body(), SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2)
        && matches!(
            input.kind,
            ResidentValueKind::F64
                | ResidentValueKind::Bool
                | ResidentValueKind::Index
                | ResidentValueKind::String
        );
    if (!snapshot && !dense)
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    source
        .instantiate_shape(
            input
                .shape_instance
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
        )
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    target
        .instantiate_shape(
            request
                .output
                .shape_instance
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
        )
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let conversion = conversion_plan(source, target)?;
    let metadata = conversion_schema_staging_bound(source, target)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let phases = conversion_materialization_phases(&conversion.step);
    Ok(BoundResidentKernel::new(execute, Box::new([]))
        .with_retained_state(Arc::new(Plan {
            conversion,
            source_schema: input.schema_id,
            source_key: source.key(),
            source_shape: input.shape_instance.clone(),
            dense_shape: input.shape,
            metadata,
            phases,
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
    let source_schema = schemas
        .get(plan.source_schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let output_schema = schemas
        .get(
            kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?
                .schema,
        )
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let mut meter = ResidentBudgetMeter::default();
    let prepared = cost::prepare(
        input,
        target.as_ref(),
        schemas,
        source_schema,
        output_schema,
        plan,
        &mut meter,
    )?;
    let canonicalization_work = PreparedKernel::new(prepared.canonicalization_work, prepared.cost)
        .admit()?
        .into_plan();
    let canonicalization =
        mech_core::snapshot::SnapshotCanonicalizationBudget::new(canonicalization_work);
    let context = SnapshotValidationContext::with_shared_schemas(
        kernel
            .shared_snapshot_schemas()
            .ok_or(ResidentKernelError::InvalidOutput)?,
    )
    .with_canonicalization_budget(&canonicalization);
    let next = materialize(
        plan,
        input,
        source_schema,
        kernel.snapshot_output().unwrap().schema,
        output_schema,
        &context,
    )?;
    let changed = target
        .as_ref()
        .map(|old| old.snapshot_eq(schemas, &next, schemas).map(|same| !same))
        .transpose()
        .map_err(|_| ResidentKernelError::InvalidOutput)?
        .unwrap_or(true);
    *target = Some(next);
    Ok(changed)
}

fn materialize(
    plan: &Plan,
    input: ResidentValueRef<'_>,
    source_schema: &Schema,
    output: SchemaId,
    output_schema: &Schema,
    context: &SnapshotValidationContext<'_>,
) -> Result<Value, ResidentKernelError> {
    // Current source geometry, not a copied parameter ordinal or flat length,
    // binds the real declared target. All allocations below are now admitted.
    let source_shape = match input {
        ResidentValueRef::Snapshot([Some(value)]) => {
            value
                .validate_against(context.schemas())
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            value.shape()
        }
        _ => &plan.source_shape,
    };
    let candidate_shape =
        conversion_output_shape(source_schema, source_shape, output_schema, &plan.conversion)
            .map_err(|_| ResidentKernelError::InvalidShape)?;
    #[cfg(test)]
    SOURCE_DRAFTS.with(|count| count.set(count.get().saturating_add(1)));
    let draft = input_draft(input, plan)?;
    let data = execute_conversion_draft_with_schema(
        draft,
        &plan.conversion,
        output_schema,
        &candidate_shape,
        context,
    )
    .map_err(|_| ResidentKernelError::Arithmetic)?;
    let shape = conversion_output_shape_for_data(output_schema, &candidate_shape, &data)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    ValueDraft {
        schema: output,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(context)
    .map_err(|_| ResidentKernelError::InvalidOutput)
}

fn input_draft(
    input: ResidentValueRef<'_>,
    plan: &Plan,
) -> Result<ValueDataDraft, ResidentKernelError> {
    let invalid_dense = if matches!(input, ResidentValueRef::Bool(_)) {
        ResidentKernelError::InvalidInput
    } else {
        ResidentKernelError::InvalidShape
    };
    let elements = match input {
        ResidentValueRef::Snapshot([Some(value)]) => {
            return value
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput);
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
        ResidentValueRef::F64(values) => {
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                Some(ValueDataDraft::F64(F64Bits::from_f64(*v)))
            })
        }
        ResidentValueRef::Bool(values) => {
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                (*v <= 1).then_some(ValueDataDraft::Bool(*v != 0))
            })
        }
        ResidentValueRef::Index(values) => {
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                Some(ValueDataDraft::Index(*v))
            })
        }
        ResidentValueRef::String(values) => {
            super::composite::canonical_matrix_elements(values, plan.dense_shape, |v| {
                Some(ValueDataDraft::String(v.clone()))
            })
        }
    }
    .ok_or(invalid_dense)?;
    Ok(ValueDataDraft::Matrix(elements))
}

use super::budget::{
    KernelCostEstimate, PreparedKernel, PreparedMutationPlan, PublishedOutputFootprint,
    ResidentBudgetMeter, checked_cost_product, checked_cost_sum, checked_product, checked_sum,
    checked_u64,
};
use mech_core::snapshot::{
    F64Bits, SnapshotValidationContext, Value, ValueDataDraft, ValueDraft,
    canonical_data_retained_footprint, canonical_snapshot_data_draft,
};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, CardinalitySpec, ChangeDetectionPolicy,
    DeliveryMode, ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, SchemaId, SchemaKey, SetValueRelation, ShapeInstance,
    ShapeRule, ValueData,
};
#[cfg(test)]
use std::cmp::Ordering;
use std::sync::Arc;

const MAX_POWERSET_INPUT_CARDINALITY: usize = 16;

fn cardinality_bounds(
    cardinality: &CardinalitySpec,
    shape: &ShapeInstance,
) -> Result<(Option<usize>, Option<usize>), ResidentKernelBindError> {
    let resolve = |dimension| {
        shape
            .resolve_dimension(dimension)
            .ok()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(ResidentKernelBindError::UnsupportedLayout)
    };
    match cardinality {
        CardinalitySpec::Exact(value) => {
            let value = resolve(value)?;
            Ok((Some(value), Some(value)))
        }
        CardinalitySpec::Dynamic { upper_bound } => {
            Ok((None, upper_bound.as_ref().map(resolve).transpose()?))
        }
    }
}

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(["set"], "union", bind_union)?;
    builder.insert_resident_factory(["set"], "cartesian-product", bind_cartesian_product)?;
    builder.insert_resident_factory(["set"], "difference", bind_difference)?;
    builder.insert_resident_factory(["set"], "disjoint", bind_disjoint)?;
    builder.insert_resident_factory(["set"], "equals", bind_equals)?;
    builder.insert_resident_factory(["set"], "intersection", bind_intersection)?;
    builder.insert_resident_factory(["set"], "not_equals", bind_not_equals)?;
    builder.insert_resident_factory(["set"], "powerset", bind_powerset)?;
    builder.insert_resident_factory(["set"], "proper-superset", bind_proper_superset)?;
    builder.insert_resident_factory(["set"], "proper_subset", bind_proper_subset)?;
    builder.insert_resident_factory(["set"], "size", bind_size)?;
    builder.insert_resident_factory(["set"], "subset", bind_subset)?;
    builder.insert_resident_factory(["set"], "superset", bind_superset)?;
    builder.insert_resident_factory(["set"], "symmetric-difference", bind_symmetric_difference)?;
    builder.insert_resident_factory(["set"], "element-of", bind_element_of)?;
    builder.insert_resident_factory(["set"], "not-element-of", bind_not_element_of)?;
    builder.insert_resident_factory(["set"], "insert", bind_insert)?;
    builder.insert_resident_factory(["set"], "remove", bind_remove)?;
    Ok(())
}

fn validate_unary_contract(
    request: &ResidentKernelBindRequest<'_>,
    change_detection: ChangeDetectionPolicy,
) -> Result<(), ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 1
        || request.inputs.len() != 1
        || contract.outputs.len() != 1
        || contract.inputs[0].schema != request.inputs[0].schema_id
        || contract.inputs[0].access != AccessMode::Read
        || contract.inputs[0].delivery != DeliveryMode::Signal
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != change_detection
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(())
}

fn validate_binary_contract(
    request: &ResidentKernelBindRequest<'_>,
    change_detection: ChangeDetectionPolicy,
) -> Result<(), ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || request.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != change_detection
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(())
}

fn is_set(schema: Option<&mech_core::Schema>) -> bool {
    matches!(
        schema.map(mech_core::Schema::body),
        Some(SchemaBody::Set { .. })
    )
}

fn set_element_schema_matches(
    schemas: &mech_core::SchemaTable,
    element: mech_core::SchemaId,
    set: mech_core::SchemaId,
) -> bool {
    schemas
        .get(set)
        .and_then(|schema| match schema.body() {
            SchemaBody::Set { element, .. } => Some(element.as_ref()),
            _ => None,
        })
        .is_some_and(|expected| {
            schemas
                .get(element)
                .is_some_and(|actual| actual.body() == expected)
        })
}

fn set_element_bodies_match(
    schemas: &mech_core::SchemaTable,
    left: mech_core::SchemaId,
    right: mech_core::SchemaId,
) -> bool {
    let element = |schema| {
        schemas.get(schema).and_then(|schema| match schema.body() {
            SchemaBody::Set { element, .. } => Some(element.as_ref()),
            _ => None,
        })
    };
    element(left)
        .zip(element(right))
        .is_some_and(|(left, right)| left == right)
}

fn set_element_body(
    schemas: &mech_core::SchemaTable,
    set: mech_core::SchemaId,
) -> Option<&SchemaBody> {
    schemas.get(set).and_then(|schema| match schema.body() {
        SchemaBody::Set { element, .. } => Some(element.as_ref()),
        _ => None,
    })
}

fn set_algebra_schemas_match(request: &ResidentKernelBindRequest<'_>) -> bool {
    set_element_bodies_match(
        request.schemas,
        request.inputs[0].schema_id,
        request.inputs[1].schema_id,
    ) && set_element_bodies_match(
        request.schemas,
        request.inputs[0].schema_id,
        request.output.schema_id,
    )
}

fn cartesian_product_schemas_match(request: &ResidentKernelBindRequest<'_>) -> bool {
    let Some(left) = set_element_body(request.schemas, request.inputs[0].schema_id) else {
        return false;
    };
    let Some(right) = set_element_body(request.schemas, request.inputs[1].schema_id) else {
        return false;
    };
    matches!(
        set_element_body(request.schemas, request.output.schema_id),
        Some(SchemaBody::Tuple(elements))
            if elements.as_ref() == [left.clone(), right.clone()]
    )
}

fn powerset_schemas_match(request: &ResidentKernelBindRequest<'_>) -> bool {
    let Some(input) = set_element_body(request.schemas, request.inputs[0].schema_id) else {
        return false;
    };
    matches!(
        set_element_body(request.schemas, request.output.schema_id),
        Some(SchemaBody::Set { element, .. }) if element.as_ref() == input
    )
}

#[derive(Clone)]
struct SetElementMetadata {
    schema: SchemaId,
    schema_key: SchemaKey,
    shape: ShapeInstance,
}

fn set_element_metadata(
    request: &ResidentKernelBindRequest<'_>,
    input: usize,
) -> Result<SetElementMetadata, ResidentKernelBindError> {
    let input = request
        .inputs
        .get(input)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(SetElementMetadata {
        schema: input.schema_id,
        schema_key: input.schema_key,
        shape: input.shape_instance.clone(),
    })
}

fn bind_union(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_set_algebra_output(request, set_union)
}

fn bind_cartesian_product(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary_set_output(
        request,
        set_cartesian_product,
        cartesian_product_schemas_match,
    )
}

fn bind_difference(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_set_algebra_output(request, set_difference)
}

fn bind_intersection(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_set_algebra_output(request, set_intersection)
}

fn bind_symmetric_difference(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_set_algebra_output(request, set_symmetric_difference)
}

fn bind_set_algebra_output(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary_set_output(request, executor, set_algebra_schemas_match)
}

fn bind_binary_set_output(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
    schemas_match: fn(&ResidentKernelBindRequest<'_>) -> bool,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_binary_contract(request, ChangeDetectionPolicy::AlwaysChanged)?;
    if request.inputs.iter().any(|input| {
        input.kind != ResidentValueKind::Snapshot || input.shape != ResidentShape::SCALAR
    }) || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || request
            .inputs
            .iter()
            .any(|input| !is_set(request.schemas.get(input.schema_id)))
        || !is_set(request.schemas.get(request.output.schema_id))
        || !schemas_match(request)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Set { cardinality, .. } = schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let shape = request.output.shape_instance.clone();
    let (exact_cardinality, maximum_cardinality) = cardinality_bounds(cardinality, &shape)?;
    Ok(BoundResidentKernel::new(executor, Box::new([]))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape,
            exact_cardinality,
            maximum_cardinality,
        })
        .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_powerset(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_unary_contract(request, ChangeDetectionPolicy::AlwaysChanged)?;
    if request.inputs[0].kind != ResidentValueKind::Snapshot
        || request.inputs[0].shape != ResidentShape::SCALAR
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || !is_set(request.schemas.get(request.inputs[0].schema_id))
        || !is_set(request.schemas.get(request.output.schema_id))
        || !powerset_schemas_match(request)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Set { cardinality, .. } = schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let shape = request.output.shape_instance.clone();
    let (exact_cardinality, maximum_cardinality) = cardinality_bounds(cardinality, &shape)?;
    Ok(BoundResidentKernel::new(set_powerset, Box::new([]))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape,
            exact_cardinality,
            maximum_cardinality,
        })
        .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_relation(
    request: &ResidentKernelBindRequest<'_>,
    relation: SetValueRelation,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_binary_contract(request, ChangeDetectionPolicy::ExactScalar)?;
    if request.inputs.iter().any(|input| {
        input.kind != ResidentValueKind::Snapshot
            || input.shape != ResidentShape::SCALAR
            || !is_set(request.schemas.get(input.schema_id))
    }) || request.output.kind != ResidentValueKind::Bool
        || request.output.shape != ResidentShape::SCALAR
        || !set_element_bodies_match(
            request.schemas,
            request.inputs[0].schema_id,
            request.inputs[1].schema_id,
        )
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(
        BoundResidentKernel::new(set_relation, vec![relation as u64].into_boxed_slice())
            .with_snapshot_schemas(request.schemas.clone()),
    )
}

fn bind_disjoint(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_relation(request, SetValueRelation::Disjoint)
}

fn bind_equals(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_relation(request, SetValueRelation::Equal)
}

fn bind_not_equals(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_relation(request, SetValueRelation::NotEqual)
}

fn bind_proper_subset(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_relation(request, SetValueRelation::ProperSubset)
}

fn bind_proper_superset(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_relation(request, SetValueRelation::ProperSuperset)
}

fn bind_subset(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_relation(request, SetValueRelation::Subset)
}

fn bind_superset(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_relation(request, SetValueRelation::Superset)
}

fn bind_size(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_unary_contract(request, ChangeDetectionPolicy::ExactScalar)?;
    if request.inputs[0].kind != ResidentValueKind::Snapshot
        || request.inputs[0].shape != ResidentShape::SCALAR
        || !is_set(request.schemas.get(request.inputs[0].schema_id))
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || !matches!(
            request
                .schemas
                .get(request.output.schema_id)
                .map(mech_core::Schema::body),
            Some(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64))
        )
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    Ok(BoundResidentKernel::new(set_size, Box::new([]))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape: request.output.shape_instance.clone(),
            exact_cardinality: None,
            maximum_cardinality: None,
        })
        .with_snapshot_schemas(request.schemas.clone()))
}

fn bind_element_of(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_membership(request, element_of, element_of_schema_mismatch)
}

fn bind_not_element_of(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_membership(request, not_element_of, not_element_of_schema_mismatch)
}

fn bind_membership(
    request: &ResidentKernelBindRequest<'_>,
    kernel: mech_core::ResidentKernelExecutor,
    mismatch_kernel: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_binary_contract(request, ChangeDetectionPolicy::ExactScalar)?;
    let element_schema_matches = set_element_schema_matches(
        request.schemas,
        request.inputs[0].schema_id,
        request.inputs[1].schema_id,
    );
    if request.inputs[0].shape != ResidentShape::SCALAR
        || request.inputs[1].kind != ResidentValueKind::Snapshot
        || request.inputs[1].shape != ResidentShape::SCALAR
        || !is_set(request.schemas.get(request.inputs[1].schema_id))
        || request.output.kind != ResidentValueKind::Bool
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if !element_schema_matches {
        return Ok(BoundResidentKernel::new(mismatch_kernel, Box::new([])));
    }
    Ok(BoundResidentKernel::new(kernel, Box::new([]))
        .with_snapshot_schemas(request.schemas.clone())
        .with_retained_state(Arc::new(set_element_metadata(request, 0)?)))
}

fn bind_mutation(
    request: &ResidentKernelBindRequest<'_>,
    kernel: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_binary_contract(request, ChangeDetectionPolicy::KernelReported)?;
    if request.inputs[0].kind != ResidentValueKind::Snapshot
        || request.inputs[0].shape != ResidentShape::SCALAR
        || request.inputs[1].shape != ResidentShape::SCALAR
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || !is_set(request.schemas.get(request.inputs[0].schema_id))
        || !is_set(request.schemas.get(request.output.schema_id))
        || !set_element_bodies_match(
            request.schemas,
            request.inputs[0].schema_id,
            request.output.schema_id,
        )
        || !set_element_schema_matches(
            request.schemas,
            request.inputs[1].schema_id,
            request.inputs[0].schema_id,
        )
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Set { cardinality, .. } = schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let shape = schema
        .instantiate_shape(Box::new([]))
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let (exact_cardinality, maximum_cardinality) = cardinality_bounds(cardinality, &shape)?;
    Ok(BoundResidentKernel::new(kernel, Box::new([]))
        .with_snapshot_output(ResidentSnapshotOutput {
            schema: request.output.schema_id,
            schema_key: request.output.schema_key,
            shape,
            exact_cardinality,
            maximum_cardinality,
        })
        .with_snapshot_schemas(request.schemas.clone())
        .with_retained_state(Arc::new(set_element_metadata(request, 1)?)))
}

fn bind_insert(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_mutation(request, insert)
}

fn bind_remove(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_mutation(request, remove)
}

fn snapshot_set_inputs<'a>(
    inputs: &'a dyn ResidentKernelInputs,
) -> Result<
    (
        &'a mech_core::snapshot::Value,
        &'a mech_core::snapshot::Value,
    ),
    ResidentKernelError,
> {
    let (
        Some(ResidentValueRef::Snapshot([Some(left)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok((left, right))
}

fn set_element_drafts(
    value: &mech_core::snapshot::Value,
) -> Result<Vec<ValueDataDraft>, ResidentKernelError> {
    let ValueDataDraft::Set(elements) = value
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidInput)?
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok(elements.into_vec())
}

fn set_cardinality(value: &Value) -> Result<usize, ResidentKernelError> {
    let ValueData::Set(set) = value.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok(set.elements().len())
}

fn value_retained_cost(
    kernel: &BoundResidentKernel,
    value: &Value,
) -> Result<(u64, u64), ResidentKernelError> {
    let footprint = value
        .retained_footprint(
            kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidInput)?,
        )
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    Ok((footprint.retained_bytes, footprint.node_count))
}

#[derive(Clone, Copy, Debug)]
struct SetCandidateCost {
    retained_bytes: u64,
    retained_nodes: u64,
    comparison_work: u64,
}

fn scalar_element_retained_cost(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    index: usize,
) -> Result<SetCandidateCost, ResidentKernelError> {
    let (retained_bytes, retained_nodes, comparison_work) = match inputs.get(index) {
        Some(ResidentValueRef::Bool([value])) if *value <= 1 => {
            (checked_u64(core::mem::size_of::<u8>())?, 1, 1)
        }
        Some(ResidentValueRef::Index([_])) => (checked_u64(core::mem::size_of::<u64>())?, 1, 1),
        Some(ResidentValueRef::F64([_])) => (checked_u64(core::mem::size_of::<f64>())?, 1, 1),
        Some(ResidentValueRef::String([value])) => (
            checked_cost_sum(&[
                checked_u64(core::mem::size_of::<String>())?,
                checked_u64(value.len())?,
            ])?,
            1,
            checked_u64(value.len())?
                .checked_add(8)
                .ok_or(ResidentKernelError::InvalidShape)?,
        ),
        Some(ResidentValueRef::Snapshot([Some(value)])) => value
            .retained_footprint(
                kernel
                    .snapshot_schemas()
                    .ok_or(ResidentKernelError::InvalidInput)?,
            )
            .map(|footprint| {
                (
                    footprint.retained_bytes,
                    footprint.node_count,
                    footprint.encoded_bytes.max(footprint.node_count).max(1),
                )
            })
            .map_err(|_| ResidentKernelError::InvalidInput)?,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    Ok(SetCandidateCost {
        retained_bytes,
        retained_nodes,
        comparison_work,
    })
}

fn admit_set_candidate_operation(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    candidate_index: usize,
    set: &Value,
    output_elements: Option<usize>,
    candidate_retained_in_output: bool,
) -> Result<(), ResidentKernelError> {
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let set_schema = set
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let SchemaBody::Set { element, .. } = set_schema.body() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ValueData::Set(set_value) = set.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let cardinality = set_value.elements().len();
    let candidate = scalar_element_retained_cost(kernel, inputs, candidate_index)?;
    let mut meter = ResidentBudgetMeter::default();
    meter.charge_temporary_bytes(candidate.retained_bytes)?;
    meter.charge_cloned_bytes(candidate.retained_bytes)?;
    meter.charge_retained_nodes(candidate.retained_nodes)?;
    meter.charge_comparison_work(candidate.comparison_work)?;
    let set_container_bytes = checked_cost_sum(&[
        checked_u64(core::mem::size_of::<Value>())?,
        checked_cost_product(&[
            checked_u64(cardinality)?,
            checked_u64(core::mem::size_of::<mech_core::snapshot::CanonicalKeyValue>())?,
        ])?,
    ])?;
    meter.charge_selector_bytes(set_container_bytes)?;
    meter.charge_retained_nodes(1)?;
    for key in set_value.elements() {
        let footprint = canonical_data_retained_footprint(element, key.data())
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        meter.charge_selector_bytes(footprint.retained_bytes)?;
        meter.charge_retained_nodes(footprint.node_count)?;
        meter.charge_comparison_work(
            candidate
                .comparison_work
                .checked_add(footprint.encoded_bytes.max(footprint.node_count).max(1))
                .ok_or(ResidentKernelError::InvalidShape)?,
        )?;
    }
    let mut cost = meter.estimate();
    let set_bytes = cost.selector_bytes;
    let set_nodes = cost
        .retained_nodes
        .checked_sub(candidate.retained_nodes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let Some(output_elements) = output_elements else {
        return PreparedMutationPlan::new(
            (),
            PublishedOutputFootprint {
                elements: 1,
                retained_bytes: checked_u64(core::mem::size_of::<u8>())?,
                retained_nodes: 1,
            },
            cost,
        )
        .admit()
        .map(|admitted| admitted.into_plan());
    };
    let output_clone_bytes = if candidate_retained_in_output {
        checked_cost_sum(&[set_bytes, candidate.retained_bytes])?
    } else {
        set_bytes
    };
    let output_nodes = if candidate_retained_in_output {
        checked_cost_sum(&[set_nodes, candidate.retained_nodes])?
    } else {
        set_nodes
    };
    let container_bytes = checked_cost_product(&[
        checked_u64(output_elements)?,
        checked_u64(core::mem::size_of::<ValueDataDraft>())?,
    ])?;
    cost.compute_work = cost
        .compute_work
        .checked_add(checked_u64(output_elements)?)
        .ok_or(ResidentKernelError::InvalidShape)?;
    cost.temporary_bytes = cost
        .temporary_bytes
        .checked_add(
            output_clone_bytes
                .checked_mul(2)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
        .ok_or(ResidentKernelError::InvalidShape)?;
    cost.cloned_bytes = cost
        .cloned_bytes
        .checked_add(output_clone_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    cost.container_bytes = cost
        .container_bytes
        .checked_add(container_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    PreparedMutationPlan::new(
        (),
        PublishedOutputFootprint {
            elements: checked_u64(output_elements)?,
            retained_bytes: checked_cost_sum(&[output_clone_bytes, container_bytes])?,
            retained_nodes: output_nodes,
        },
        cost,
    )
    .admit()?
    .into_plan();
    Ok(())
}

fn admit_set_read(comparison_work: usize) -> Result<(), ResidentKernelError> {
    super::budget::resident_cost! {
        comparison_work,
        compute_work: comparison_work,
        ..KernelCostEstimate::default()
    }
    .admit()
}

fn admit_set_materialization(
    output_elements: usize,
    comparison_work: usize,
    cloned_bytes: u64,
    retained_nodes: u64,
) -> Result<PreparedKernel<()>, ResidentKernelError> {
    let container_bytes = checked_cost_product(&[
        checked_u64(output_elements)?,
        checked_u64(std::mem::size_of::<usize>())?,
    ])?;
    let output_bytes = checked_cost_sum(&[cloned_bytes, container_bytes])?;
    Ok(PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work,
            compute_work: checked_sum(&[comparison_work, output_elements])?,
            output_elements,
            output_bytes,
            temporary_bytes: checked_cost_sum(&[output_bytes, output_bytes])?,
            cloned_bytes,
            retained_nodes,
            ..KernelCostEstimate::default()
        },
    ))
}

fn finalize_snapshot(
    kernel: &BoundResidentKernel,
    data: ValueDataDraft,
) -> Result<mech_core::snapshot::Value, ResidentKernelError> {
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    ValueDraft {
        schema: metadata.schema,
        shape_values: metadata
            .shape
            .parameter_values()
            .to_vec()
            .into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|_| ResidentKernelError::InvalidOutput)
}

fn write_full_snapshot(
    output: ResidentValueMut<'_>,
    next: mech_core::snapshot::Value,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    *target = Some(next);
    Ok(true)
}

fn write_changed_snapshot(
    kernel: &BoundResidentKernel,
    output: ResidentValueMut<'_>,
    next: mech_core::snapshot::Value,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let changed = match target.as_ref() {
        Some(current) => !current
            .language_eq(schemas, &next, schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?,
        None => true,
    };
    *target = Some(next);
    Ok(changed)
}

type SetMerge = fn(
    &Value,
    &mech_core::SchemaTable,
    &Value,
    &mech_core::SchemaTable,
) -> Result<Box<[ValueData]>, mech_core::snapshot::SnapshotValueError>;

fn merged_set_element_drafts(
    kernel: &BoundResidentKernel,
    left: &Value,
    right: &Value,
    maximum_output_elements: usize,
    merge: SetMerge,
) -> Result<Box<[ValueDataDraft]>, ResidentKernelError> {
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let left_count = set_cardinality(left)?;
    let right_count = set_cardinality(right)?;
    let (left_bytes, left_nodes) = value_retained_cost(kernel, left)?;
    let (right_bytes, right_nodes) = value_retained_cost(kernel, right)?;
    admit_set_materialization(
        maximum_output_elements,
        checked_sum(&[left_count, right_count])?,
        checked_cost_sum(&[left_bytes, right_bytes])?,
        checked_cost_sum(&[left_nodes, right_nodes])?,
    )?
    .admit()?
    .into_plan();
    let elements =
        merge(left, schemas, right, schemas).map_err(|_| ResidentKernelError::InvalidInput)?;
    left.set_element_data_drafts(schemas, &elements)
        .map_err(|_| ResidentKernelError::InvalidInput)
}

fn set_union(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let maximum = checked_sum(&[set_cardinality(left)?, set_cardinality(right)?])?;
    let elements =
        merged_set_element_drafts(kernel, left, right, maximum, Value::set_union_elements)?;
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements))?;
    write_full_snapshot(output, next)
}

fn set_intersection(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let maximum = set_cardinality(left)?.min(set_cardinality(right)?);
    let elements = merged_set_element_drafts(
        kernel,
        left,
        right,
        maximum,
        Value::set_intersection_elements,
    )?;
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements))?;
    write_full_snapshot(output, next)
}

fn set_difference(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let elements = merged_set_element_drafts(
        kernel,
        left,
        right,
        set_cardinality(left)?,
        Value::set_difference_elements,
    )?;
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements))?;
    write_full_snapshot(output, next)
}

fn set_symmetric_difference(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let maximum = checked_sum(&[set_cardinality(left)?, set_cardinality(right)?])?;
    let elements = merged_set_element_drafts(
        kernel,
        left,
        right,
        maximum,
        Value::set_symmetric_difference_elements,
    )?;
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements))?;
    write_full_snapshot(output, next)
}

fn set_cartesian_product(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let (left_retained_bytes, left_retained_nodes) = value_retained_cost(kernel, left)?;
    let (right_retained_bytes, right_retained_nodes) = value_retained_cost(kernel, right)?;
    let left_count = set_cardinality(left)?;
    let right_count = set_cardinality(right)?;
    let output_len = checked_product(&[left_count, right_count])?;
    let result_cloned_bytes = checked_cost_sum(&[
        checked_cost_product(&[left_retained_bytes, checked_u64(right_count)?])?,
        checked_cost_product(&[right_retained_bytes, checked_u64(left_count)?])?,
    ])?;
    // Both canonical element draft arrays are materialized before the nested
    // product loop, even when one side is empty and the result has no pairs.
    let input_staging_bytes = checked_cost_sum(&[left_retained_bytes, right_retained_bytes])?;
    let cloned_bytes = checked_cost_sum(&[result_cloned_bytes, input_staging_bytes])?;
    let result_cloned_nodes = checked_cost_sum(&[
        checked_cost_product(&[left_retained_nodes, checked_u64(right_count)?])?,
        checked_cost_product(&[right_retained_nodes, checked_u64(left_count)?])?,
    ])?;
    let input_staging_nodes = checked_cost_sum(&[left_retained_nodes, right_retained_nodes])?;
    // Every result adds its tuple node around recursively cloned member trees.
    let output_nodes = checked_cost_sum(&[result_cloned_nodes, checked_u64(output_len)?, 1])?;
    let container_bytes = checked_cost_product(&[
        checked_u64(output_len)?,
        2,
        checked_u64(std::mem::size_of::<usize>())?,
    ])?;
    let output_bytes = checked_cost_sum(&[result_cloned_bytes, container_bytes])?;
    PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work: output_len,
            output_elements: output_len,
            output_bytes,
            temporary_bytes: checked_cost_sum(&[output_bytes, input_staging_bytes])?,
            cloned_bytes,
            retained_nodes: checked_cost_sum(&[output_nodes, input_staging_nodes])?,
            ..KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let left = set_element_drafts(left)?;
    let right = set_element_drafts(right)?;
    let mut elements = Vec::with_capacity(output_len);
    for left in left {
        for right in &right {
            elements.push(ValueDataDraft::Tuple(
                vec![left.clone(), right.clone()].into_boxed_slice(),
            ));
        }
    }
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements.into_boxed_slice()))?;
    write_full_snapshot(output, next)
}

fn set_powerset(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let element_count = set_cardinality(input)?;
    if element_count > MAX_POWERSET_INPUT_CARDINALITY {
        return Err(ResidentKernelError::InvalidShape);
    }
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let subset_count = 1usize
        .checked_shl(element_count as u32)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let copies_per_element = if element_count == 0 {
        0
    } else {
        subset_count / 2
    };
    let member_copies = checked_product(&[element_count, copies_per_element])?;
    let output_elements = checked_sum(&[subset_count, member_copies])?;
    let footprint = input
        .retained_footprint(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let retained_bytes = footprint
        .retained_bytes
        .checked_mul(checked_u64(copies_per_element)?)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let cloned_nodes = footprint
        .node_count
        .checked_mul(checked_u64(copies_per_element)?)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_nodes = checked_cost_sum(&[cloned_nodes, checked_u64(subset_count)?, 1])?;
    let subset_headers = checked_cost_product(&[
        checked_u64(subset_count)?,
        checked_u64(std::mem::size_of::<usize>())?,
    ])?;
    let output_bytes = checked_cost_sum(&[retained_bytes, subset_headers])?;
    PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            compute_work: member_copies,
            output_elements,
            output_bytes,
            temporary_bytes: output_bytes,
            cloned_bytes: retained_bytes,
            retained_nodes: output_nodes,
            ..KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let elements = set_element_drafts(input)?;
    let mut subsets = vec![Vec::new()];
    for element in elements {
        let with_element = subsets
            .iter()
            .map(|subset| {
                let mut next = subset.clone();
                next.push(element.clone());
                next
            })
            .collect::<Vec<_>>();
        subsets.extend(with_element);
    }
    subsets.sort_by_key(Vec::len);
    let elements = subsets
        .into_iter()
        .map(|subset| ValueDataDraft::Set(subset.into_boxed_slice()))
        .collect::<Vec<_>>();
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements.into_boxed_slice()))?;
    write_full_snapshot(output, next)
}

fn set_relation(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let relation = match kernel.parameters().first().copied() {
        Some(value) if value == SetValueRelation::Disjoint as u64 => SetValueRelation::Disjoint,
        Some(value) if value == SetValueRelation::Equal as u64 => SetValueRelation::Equal,
        Some(value) if value == SetValueRelation::NotEqual as u64 => SetValueRelation::NotEqual,
        Some(value) if value == SetValueRelation::ProperSubset as u64 => {
            SetValueRelation::ProperSubset
        }
        Some(value) if value == SetValueRelation::ProperSuperset as u64 => {
            SetValueRelation::ProperSuperset
        }
        Some(value) if value == SetValueRelation::Subset as u64 => SetValueRelation::Subset,
        Some(value) if value == SetValueRelation::Superset as u64 => SetValueRelation::Superset,
        _ => return Err(ResidentKernelError::InvalidInput),
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    admit_set_read(checked_sum(&[
        set_cardinality(left)?,
        set_cardinality(right)?,
    ])?)?;
    let next = left
        .set_relation(schemas, right, schemas, relation)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::Bool([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let next = u8::from(next);
    let changed = *target != next;
    *target = next;
    Ok(changed)
}

fn set_size(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(input)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ValueData::Set(set) = input.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let size = u64::try_from(set.elements().len()).map_err(|_| ResidentKernelError::Arithmetic)?;
    let next = finalize_snapshot(kernel, ValueDataDraft::U64(size))?;
    write_changed_snapshot(kernel, output, next)
}

fn scalar_element_draft(
    inputs: &dyn ResidentKernelInputs,
    index: usize,
) -> Result<ValueDataDraft, ResidentKernelError> {
    match inputs.get(index) {
        Some(ResidentValueRef::Bool([value])) => Ok(ValueDataDraft::Bool(*value != 0)),
        Some(ResidentValueRef::Index([value])) => Ok(ValueDataDraft::Index(*value)),
        Some(ResidentValueRef::F64([value])) => Ok(ValueDataDraft::F64(F64Bits::from_f64(*value))),
        Some(ResidentValueRef::String([value])) => Ok(ValueDataDraft::String(value.clone())),
        Some(ResidentValueRef::Snapshot([Some(value)])) => value
            .canonical_data_draft()
            .map_err(|_| ResidentKernelError::InvalidInput),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn finalize_set_element(
    kernel: &BoundResidentKernel,
    data: ValueDataDraft,
) -> Result<Value, ResidentKernelError> {
    let metadata = kernel
        .retained_state::<SetElementMetadata>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let value = ValueDraft {
        schema: metadata.schema,
        shape_values: metadata
            .shape
            .parameter_values()
            .to_vec()
            .into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .map_err(|_| ResidentKernelError::InvalidInput)?;
    if value.schema_key() != metadata.schema_key {
        return Err(ResidentKernelError::InvalidInput);
    }
    Ok(value)
}

enum SetCandidateValue<'a> {
    Borrowed {
        value: &'a Value,
        schemas: Arc<mech_core::SchemaTable>,
    },
    Owned(Value),
}

impl SetCandidateValue<'_> {
    fn value(&self) -> &Value {
        match self {
            Self::Borrowed { value, .. } => value,
            Self::Owned(value) => value,
        }
    }

    fn schemas<'a>(&'a self, fallback: &'a mech_core::SchemaTable) -> &'a mech_core::SchemaTable {
        match self {
            Self::Borrowed { schemas, .. } => schemas,
            Self::Owned(_) => fallback,
        }
    }
}

fn set_candidate_value<'a>(
    kernel: &BoundResidentKernel,
    inputs: &'a dyn ResidentKernelInputs,
    index: usize,
) -> Result<SetCandidateValue<'a>, ResidentKernelError> {
    let metadata = kernel
        .retained_state::<SetElementMetadata>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    match inputs.get(index) {
        Some(ResidentValueRef::Snapshot([Some(value)])) => {
            let source_schemas = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
            let source_schema = value
                .validate_against(&source_schemas)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            let target_schema = schemas
                .get(metadata.schema)
                .ok_or(ResidentKernelError::InvalidInput)?;
            if source_schema.body() != target_schema.body() || value.shape() != &metadata.shape {
                return Err(ResidentKernelError::InvalidInput);
            }
            Ok(SetCandidateValue::Borrowed {
                value,
                schemas: source_schemas,
            })
        }
        _ => finalize_set_element(kernel, scalar_element_draft(inputs, index)?)
            .map(SetCandidateValue::Owned),
    }
}

fn canonical_set_element_drafts(
    kernel: &BoundResidentKernel,
    set: &Value,
    elements: Box<[ValueData]>,
) -> Result<Box<[ValueDataDraft]>, ResidentKernelError> {
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schema = set
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let SchemaBody::Set { element, .. } = schema.body() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    elements
        .iter()
        .map(|value| {
            canonical_snapshot_data_draft(element, value)
                .map_err(|_| ResidentKernelError::InvalidInput)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn write_membership(output: ResidentValueMut<'_>, next: bool) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Bool([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let next = u8::from(next);
    let changed = *target != next;
    *target = next;
    Ok(changed)
}

fn element_of(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    admit_set_candidate_operation(kernel, inputs, 0, set, None, false)?;
    let element = set_candidate_value(kernel, inputs, 0)?;
    let next = set
        .set_contains(schemas, element.value(), element.schemas(schemas))
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    write_membership(output, next)
}

fn element_of_schema_mismatch(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let ResidentValueMut::Bool(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let [target] = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let changed = *target != 0;
    *target = 0;
    Ok(changed)
}

fn not_element_of(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    admit_set_candidate_operation(kernel, inputs, 0, set, None, false)?;
    let element = set_candidate_value(kernel, inputs, 0)?;
    let next = set
        .set_contains(schemas, element.value(), element.schemas(schemas))
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    write_membership(output, !next)
}

fn not_element_of_schema_mismatch(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let ResidentValueMut::Bool([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let changed = *target != 1;
    *target = 1;
    Ok(changed)
}

fn insert(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let maximum_output_elements = set_cardinality(set)?
        .checked_add(1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    admit_set_candidate_operation(kernel, inputs, 1, set, Some(maximum_output_elements), true)?;
    let element = set_candidate_value(kernel, inputs, 1)?;
    let elements = set
        .set_elements_after_insert(schemas, element.value(), element.schemas(schemas))
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let elements = canonical_set_element_drafts(kernel, set, elements)?;
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements))?;
    write_changed_snapshot(kernel, output, next)
}

fn remove(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    admit_set_candidate_operation(kernel, inputs, 1, set, Some(set_cardinality(set)?), false)?;
    let element = set_candidate_value(kernel, inputs, 1)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let elements = set
        .set_elements_after_remove(schemas, element.value(), element.schemas(schemas))
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let elements = canonical_set_element_drafts(kernel, set, elements)?;
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements))?;
    write_changed_snapshot(kernel, output, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::snapshot::NamedValueDraft;
    use mech_core::{FloatWidth, SchemaField};

    struct Inputs<'a>(&'a [ResidentValueRef<'a>]);

    impl ResidentKernelInputs for Inputs<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index).copied()
        }
    }

    fn schema(body: SchemaBody) -> mech_core::Schema {
        mech_core::SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn resident_set_semantics_do_not_reintroduce_raw_draft_equality() {
        let production = include_str!("set.rs").split("#[cfg(test)]").next().unwrap();
        assert!(!production.contains(".contains("));
        assert!(!production.contains("ValueDataDraft::contains"));
    }

    #[test]
    fn membership_requires_the_exact_set_element_schema() {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let scalar = builder
            .insert(schema(SchemaBody::FloatingPoint(FloatWidth::W64)))
            .unwrap();
        let matrix = builder
            .insert(schema(SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(1),
                    mech_core::DimensionExpr::Constant(1),
                ]
                .into_boxed_slice(),
            }))
            .unwrap();
        let set = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                cardinality: mech_core::DimensionExpr::Constant(3).into(),
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar).unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let set = build.resolve(set).unwrap();
        let (schemas, _) = build.into_parts();

        assert!(set_element_schema_matches(&schemas, scalar, set));
        assert!(!set_element_schema_matches(&schemas, matrix, set));
    }

    #[test]
    fn set_algebra_binder_rejects_mismatched_element_schemas() {
        let dynamic = CardinalitySpec::Dynamic { upper_bound: None };
        let mut builder = mech_core::SchemaTableBuilder::new();
        let numbers = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
                cardinality: dynamic.clone(),
            }))
            .unwrap();
        let strings = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(SchemaBody::String),
                cardinality: dynamic,
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let numbers = build.resolve(numbers).unwrap();
        let strings = build.resolve(strings).unwrap();
        let (schemas, _) = build.into_parts();
        let port = |schema_id| mech_core::ResidentPortLayout {
            schema_id,
            schema_key: schemas.entry(schema_id).unwrap().key(),
            kind: ResidentValueKind::Snapshot,
            shape: ResidentShape::SCALAR,
            shape_instance: schemas
                .get(schema_id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        };
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: vec![numbers, strings]
                .into_iter()
                .map(|schema| mech_core::ResolvedInputPort {
                    schema,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema: numbers,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::AlwaysChanged,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        });
        assert!(matches!(
            bind_union(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[port(numbers), port(strings)],
                output: port(numbers),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
    }

    #[test]
    fn set_union_uses_output_cardinality_for_derived_results() {
        let element = SchemaBody::FloatingPoint(FloatWidth::W64);
        let mut builder = mech_core::SchemaTableBuilder::new();
        let input_handle = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(element.clone()),
                cardinality: mech_core::DimensionExpr::Constant(2).into(),
            }))
            .unwrap();
        let output_handle = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(element),
                cardinality: mech_core::DimensionExpr::Constant(3).into(),
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let input_schema = build.resolve(input_handle).unwrap();
        let output_schema = build.resolve(output_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let set = |values: &[f64]| {
            ValueDraft {
                schema: input_schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Set(
                    values
                        .iter()
                        .map(|value| ValueDataDraft::F64(F64Bits::from_f64(*value)))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap()
        };
        let left_slot = [Some(set(&[1.0, 2.0]))];
        let right_slot = [Some(set(&[2.0, 3.0]))];
        let inputs = [
            ResidentValueRef::Snapshot(&left_slot),
            ResidentValueRef::Snapshot(&right_slot),
        ];
        let output_shape = schemas
            .get(output_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let kernel = BoundResidentKernel::new(set_union, Box::new([]))
            .with_snapshot_output(ResidentSnapshotOutput {
                schema: output_schema,
                schema_key: schemas.entry(output_schema).unwrap().key(),
                shape: output_shape,
                exact_cardinality: Some(3),
                maximum_cardinality: Some(3),
            })
            .with_snapshot_schemas(schemas);
        let mut output = [None];

        assert_eq!(
            kernel.execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output)),
            Ok(true),
        );
        let ValueData::Set(output) = output[0].as_ref().unwrap().data() else {
            panic!("set/union must produce a set");
        };
        assert_eq!(output.elements().len(), 3);
    }

    fn nested_nan_element(outer_nan: u64, inner_nan: u64) -> ValueDataDraft {
        ValueDataDraft::Tuple(
            vec![
                ValueDataDraft::F64(F64Bits::from_bits(outer_nan)),
                ValueDataDraft::Record(
                    vec![NamedValueDraft {
                        name: "nested".to_owned(),
                        value: ValueDataDraft::Set(
                            vec![ValueDataDraft::F64(F64Bits::from_bits(inner_nan))]
                                .into_boxed_slice(),
                        ),
                    }]
                    .into_boxed_slice(),
                ),
            ]
            .into_boxed_slice(),
        )
    }

    #[test]
    fn snapshot_set_operations_use_recursive_canonical_nan_keys() {
        let inner_set = SchemaBody::Set {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        };
        let element_body = SchemaBody::Tuple(
            vec![
                SchemaBody::FloatingPoint(FloatWidth::W64),
                SchemaBody::Record(
                    vec![SchemaField {
                        name: "nested".to_owned(),
                        schema: inner_set,
                    }]
                    .into_boxed_slice(),
                ),
            ]
            .into_boxed_slice(),
        );
        let mut builder = mech_core::SchemaTableBuilder::new();
        let element_handle = builder.insert(schema(element_body.clone())).unwrap();
        let set_handle = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(element_body),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let element_schema = build.resolve(element_handle).unwrap();
        let set_schema = build.resolve(set_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let element_shape = schemas
            .get(element_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let set_shape = schemas
            .get(set_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let element_metadata = SetElementMetadata {
            schema: element_schema,
            schema_key: schemas.entry(element_schema).unwrap().key(),
            shape: element_shape,
        };
        let existing = ValueDraft {
            schema: element_schema,
            shape_values: Box::new([]),
            data: nested_nan_element(0x7ff0_0000_0000_0001, 0x7ff8_0000_0000_0002),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let candidate = ValueDraft {
            schema: element_schema,
            shape_values: Box::new([]),
            data: nested_nan_element(0xfff8_0000_0000_0042, 0xfff0_0000_0000_0043),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        assert!(
            !existing
                .snapshot_eq(&schemas, &candidate, &schemas)
                .unwrap()
        );
        assert_eq!(
            existing.key_cmp(&schemas, &candidate, &schemas).unwrap(),
            Ordering::Equal
        );
        let set = ValueDraft {
            schema: set_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Set(
                vec![existing.canonical_data_draft().unwrap()].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();

        let candidate_slot = [Some(candidate.clone())];
        let set_slot = [Some(set.clone())];
        let membership_inputs = [
            ResidentValueRef::Snapshot(&candidate_slot),
            ResidentValueRef::Snapshot(&set_slot),
        ];
        let membership_kernel = BoundResidentKernel::new(element_of, Box::new([]))
            .with_snapshot_schemas(schemas.clone())
            .with_retained_state(Arc::new(element_metadata.clone()));
        let mut member = [0_u8];
        assert_eq!(
            membership_kernel.execute(
                &Inputs(&membership_inputs),
                ResidentValueMut::Bool(&mut member),
            ),
            Ok(true),
        );
        assert_eq!(member, [1]);

        let non_membership_kernel = BoundResidentKernel::new(not_element_of, Box::new([]))
            .with_snapshot_schemas(schemas.clone())
            .with_retained_state(Arc::new(element_metadata.clone()));
        let mut non_member = [1_u8];
        assert_eq!(
            non_membership_kernel.execute(
                &Inputs(&membership_inputs),
                ResidentValueMut::Bool(&mut non_member),
            ),
            Ok(true),
        );
        assert_eq!(non_member, [0]);

        let mutation_kernel = |executor| {
            BoundResidentKernel::new(executor, Box::new([]))
                .with_snapshot_output(ResidentSnapshotOutput {
                    schema: set_schema,
                    schema_key: schemas.entry(set_schema).unwrap().key(),
                    shape: set_shape.clone(),
                    exact_cardinality: None,
                    maximum_cardinality: None,
                })
                .with_snapshot_schemas(schemas.clone())
                .with_retained_state(Arc::new(element_metadata.clone()))
        };
        let mutation_inputs = [
            ResidentValueRef::Snapshot(&set_slot),
            ResidentValueRef::Snapshot(&candidate_slot),
        ];
        let mut inserted = [None];
        assert_eq!(
            mutation_kernel(insert).execute(
                &Inputs(&mutation_inputs),
                ResidentValueMut::Snapshot(&mut inserted),
            ),
            Ok(true),
        );
        let ValueData::Set(inserted) = inserted[0].as_ref().unwrap().data() else {
            panic!("set/insert must produce a set");
        };
        assert_eq!(inserted.elements().len(), 1);

        let mut removed = [None];
        assert_eq!(
            mutation_kernel(remove).execute(
                &Inputs(&mutation_inputs),
                ResidentValueMut::Snapshot(&mut removed),
            ),
            Ok(true),
        );
        let ValueData::Set(removed) = removed[0].as_ref().unwrap().data() else {
            panic!("set/remove must produce a set");
        };
        assert!(removed.elements().is_empty());

        let candidate_set = ValueDraft {
            schema: set_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Set(
                vec![candidate.canonical_data_draft().unwrap()].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let candidate_set_slot = [Some(candidate_set)];
        let algebra_inputs = [
            ResidentValueRef::Snapshot(&set_slot),
            ResidentValueRef::Snapshot(&candidate_set_slot),
        ];
        let algebra_kernel = |executor| {
            BoundResidentKernel::new(executor, Box::new([]))
                .with_snapshot_output(ResidentSnapshotOutput {
                    schema: set_schema,
                    schema_key: schemas.entry(set_schema).unwrap().key(),
                    shape: set_shape.clone(),
                    exact_cardinality: None,
                    maximum_cardinality: None,
                })
                .with_snapshot_schemas(schemas.clone())
        };
        for (executor, expected_len) in [
            (set_union as mech_core::ResidentKernelExecutor, 1),
            (set_intersection as mech_core::ResidentKernelExecutor, 1),
            (set_difference as mech_core::ResidentKernelExecutor, 0),
            (
                set_symmetric_difference as mech_core::ResidentKernelExecutor,
                0,
            ),
        ] {
            let mut output = [None];
            assert_eq!(
                algebra_kernel(executor).execute(
                    &Inputs(&algebra_inputs),
                    ResidentValueMut::Snapshot(&mut output),
                ),
                Ok(true),
            );
            let ValueData::Set(output) = output[0].as_ref().unwrap().data() else {
                panic!("set algebra must produce a set");
            };
            assert_eq!(output.elements().len(), expected_len);
        }
    }

    #[test]
    fn set_expansions_reject_byte_amplification_before_allocating_outputs() {
        let dynamic = CardinalitySpec::Dynamic { upper_bound: None };
        let string_set_body = SchemaBody::Set {
            element: Box::new(SchemaBody::String),
            cardinality: dynamic.clone(),
        };
        let mut builder = mech_core::SchemaTableBuilder::new();
        let string_set_handle = builder.insert(schema(string_set_body.clone())).unwrap();
        let pair_set_handle = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(SchemaBody::Tuple(
                    vec![SchemaBody::String, SchemaBody::String].into_boxed_slice(),
                )),
                cardinality: dynamic.clone(),
            }))
            .unwrap();
        let powerset_handle = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(string_set_body),
                cardinality: dynamic,
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let string_set_schema = build.resolve(string_set_handle).unwrap();
        let pair_set_schema = build.resolve(pair_set_handle).unwrap();
        let powerset_schema = build.resolve(powerset_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let shape = |schema| {
            schemas
                .get(schema)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap()
        };
        let set = |count: usize, width: usize| {
            ValueDraft {
                schema: string_set_schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Set(
                    (0..count)
                        .map(|index| {
                            ValueDataDraft::String(format!("{index:04}-{}", "x".repeat(width)))
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap()
        };
        let output_kernel = |executor, schema| {
            BoundResidentKernel::new(executor, Box::new([]))
                .with_snapshot_output(ResidentSnapshotOutput {
                    schema,
                    schema_key: schemas.entry(schema).unwrap().key(),
                    shape: shape(schema),
                    exact_cardinality: None,
                    maximum_cardinality: None,
                })
                .with_snapshot_schemas(schemas.clone())
        };

        // Compact members keep byte amplification below the cap, but 65,536
        // pairs still materialize 196,608 tuple/member nodes.
        let left = [Some(set(256, 0))];
        let right = [Some(set(256, 0))];
        let cartesian_inputs = [
            ResidentValueRef::Snapshot(&left),
            ResidentValueRef::Snapshot(&right),
        ];
        let mut cartesian_output = [None];
        assert_eq!(
            output_kernel(set_cartesian_product, pair_set_schema).execute(
                &Inputs(&cartesian_inputs),
                ResidentValueMut::Snapshot(&mut cartesian_output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(cartesian_output[0].is_none());

        // An empty opposite side does not erase the cost of staging the input
        // element drafts that the implementation materializes unconditionally.
        let large = [Some(set(1, 16 * 1024 * 1024))];
        let empty = [Some(set(0, 0))];
        let empty_product_inputs = [
            ResidentValueRef::Snapshot(&large),
            ResidentValueRef::Snapshot(&empty),
        ];
        let mut empty_product_output = [None];
        assert_eq!(
            output_kernel(set_cartesian_product, pair_set_schema).execute(
                &Inputs(&empty_product_inputs),
                ResidentValueMut::Snapshot(&mut empty_product_output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(empty_product_output[0].is_none());

        // Sixteen short elements stay below the byte budget, but their
        // powerset contains 65,536 subsets plus 524,288 nested member copies.
        let input = [Some(set(16, 1))];
        let powerset_inputs = [ResidentValueRef::Snapshot(&input)];
        let mut powerset_output = [None];
        assert_eq!(
            output_kernel(set_powerset, powerset_schema).execute(
                &Inputs(&powerset_inputs),
                ResidentValueMut::Snapshot(&mut powerset_output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(powerset_output[0].is_none());
    }

    #[test]
    fn set_candidates_are_admitted_before_rebinding_or_comparison() {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let element_handle = builder.insert(schema(SchemaBody::String)).unwrap();
        let set_handle = builder
            .insert(schema(SchemaBody::Set {
                element: Box::new(SchemaBody::String),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let element_schema = build.resolve(element_handle).unwrap();
        let set_schema = build.resolve(set_handle).unwrap();
        let (schemas, _) = build.into_parts();
        let element_shape = schemas
            .get(element_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let set_shape = schemas
            .get(set_schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        let metadata = SetElementMetadata {
            schema: element_schema,
            schema_key: schemas.entry(element_schema).unwrap().key(),
            shape: element_shape,
        };
        let candidate = ValueDraft {
            schema: element_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::String(
                "x".repeat(super::super::budget::MAX_RESIDENT_CLONED_BYTES as usize + 1),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let empty_set = ValueDraft {
            schema: set_schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Set(Box::new([])),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let candidate_slot = [Some(candidate)];
        let set_slot = [Some(empty_set.clone())];
        let membership_inputs = [
            ResidentValueRef::Snapshot(&candidate_slot),
            ResidentValueRef::Snapshot(&set_slot),
        ];
        for (executor, initial) in [
            (element_of as mech_core::ResidentKernelExecutor, 1_u8),
            (not_element_of as mech_core::ResidentKernelExecutor, 0_u8),
        ] {
            let kernel = BoundResidentKernel::new(executor, Box::new([]))
                .with_snapshot_schemas(schemas.clone())
                .with_retained_state(Arc::new(metadata.clone()));
            let mut output = [initial];
            assert_eq!(
                kernel.execute(
                    &Inputs(&membership_inputs),
                    ResidentValueMut::Bool(&mut output),
                ),
                Err(ResidentKernelError::InvalidShape),
            );
            assert_eq!(output, [initial]);
        }

        let mutation_inputs = [
            ResidentValueRef::Snapshot(&set_slot),
            ResidentValueRef::Snapshot(&candidate_slot),
        ];
        for executor in [
            insert as mech_core::ResidentKernelExecutor,
            remove as mech_core::ResidentKernelExecutor,
        ] {
            let kernel = BoundResidentKernel::new(executor, Box::new([]))
                .with_snapshot_output(ResidentSnapshotOutput {
                    schema: set_schema,
                    schema_key: schemas.entry(set_schema).unwrap().key(),
                    shape: set_shape.clone(),
                    exact_cardinality: None,
                    maximum_cardinality: None,
                })
                .with_snapshot_schemas(schemas.clone())
                .with_retained_state(Arc::new(metadata.clone()));
            let mut output = [Some(empty_set.clone())];
            assert_eq!(
                kernel.execute(
                    &Inputs(&mutation_inputs),
                    ResidentValueMut::Snapshot(&mut output),
                ),
                Err(ResidentKernelError::InvalidShape),
            );
            let ValueData::Set(output) = output[0].as_ref().unwrap().data() else {
                unreachable!()
            };
            assert!(output.elements().is_empty());
        }
    }
}

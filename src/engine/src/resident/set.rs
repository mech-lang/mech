use mech_core::snapshot::{
    SnapshotValidationContext, ValueDataDraft, ValueDraft, build_f64_set_snapshot,
    build_f64_set_snapshot_after_remove, f64_set_snapshot_contains,
};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, CardinalitySpec, ChangeDetectionPolicy,
    DeliveryMode, ExternalInteraction, FloatWidth, FunctionCatalogBuilder, MResult,
    OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError,
    ResidentKernelInputs, ResidentShape, ResidentSnapshotOutput, ResidentValueKind,
    ResidentValueMut, ResidentValueRef, ResolvedOperationContract, SchemaBody, SetValueRelation,
    ShapeInstance, ShapeRule, ValueData,
};

const MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY: usize = 65_536;
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

fn is_f64_set(schema: Option<&mech_core::Schema>) -> bool {
    matches!(
        schema.map(|schema| schema.body()),
        Some(SchemaBody::Set { element, .. })
            if element.as_ref() == &SchemaBody::FloatingPoint(FloatWidth::W64)
    )
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

fn bind_union(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary_set_output(request, set_union)
}

fn bind_cartesian_product(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary_set_output(request, set_cartesian_product)
}

fn bind_difference(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary_set_output(request, set_difference)
}

fn bind_intersection(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary_set_output(request, set_intersection)
}

fn bind_symmetric_difference(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    bind_binary_set_output(request, set_symmetric_difference)
}

fn bind_binary_set_output(
    request: &ResidentKernelBindRequest<'_>,
    executor: mech_core::ResidentKernelExecutor,
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
    if request.inputs[0].kind != ResidentValueKind::F64
        || request.inputs[0].shape != ResidentShape::SCALAR
        || request.inputs[1].kind != ResidentValueKind::Snapshot
        || request.inputs[1].shape != ResidentShape::SCALAR
        || !is_f64_set(request.schemas.get(request.inputs[1].schema_id))
        || request.output.kind != ResidentValueKind::Bool
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if !element_schema_matches {
        return Ok(BoundResidentKernel::new(mismatch_kernel, Box::new([])));
    }
    Ok(BoundResidentKernel::new(kernel, Box::new([])))
}

fn bind_mutation(
    request: &ResidentKernelBindRequest<'_>,
    kernel: mech_core::ResidentKernelExecutor,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    validate_binary_contract(request, ChangeDetectionPolicy::KernelReported)?;
    if request.inputs[0].kind != ResidentValueKind::Snapshot
        || request.inputs[0].shape != ResidentShape::SCALAR
        || request.inputs[1].kind != ResidentValueKind::F64
        || request.inputs[1].shape != ResidentShape::SCALAR
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || !is_f64_set(request.schemas.get(request.inputs[0].schema_id))
        || !is_f64_set(request.schemas.get(request.output.schema_id))
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
        .with_snapshot_schemas(request.schemas.clone()))
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

fn set_union(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let mut elements = set_element_drafts(left)?;
    for element in set_element_drafts(right)? {
        if !elements.contains(&element) {
            elements.push(element);
        }
    }
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements.into_boxed_slice()))?;
    write_full_snapshot(output, next)
}

fn set_intersection(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let right = set_element_drafts(right)?;
    let elements = set_element_drafts(left)?
        .into_iter()
        .filter(|element| right.contains(element))
        .collect::<Vec<_>>();
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements.into_boxed_slice()))?;
    write_full_snapshot(output, next)
}

fn set_difference(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let right = set_element_drafts(right)?;
    let elements = set_element_drafts(left)?
        .into_iter()
        .filter(|element| !right.contains(element))
        .collect::<Vec<_>>();
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements.into_boxed_slice()))?;
    write_full_snapshot(output, next)
}

fn set_symmetric_difference(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let left = set_element_drafts(left)?;
    let right = set_element_drafts(right)?;
    let elements = left
        .iter()
        .filter(|element| !right.contains(element))
        .chain(right.iter().filter(|element| !left.contains(element)))
        .cloned()
        .collect::<Vec<_>>();
    let next = finalize_snapshot(kernel, ValueDataDraft::Set(elements.into_boxed_slice()))?;
    write_full_snapshot(output, next)
}

fn set_cartesian_product(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (left, right) = snapshot_set_inputs(inputs)?;
    let left = set_element_drafts(left)?;
    let right = set_element_drafts(right)?;
    let output_len = left
        .len()
        .checked_mul(right.len())
        .filter(|len| *len <= MAX_CARTESIAN_PRODUCT_OUTPUT_CARDINALITY)
        .ok_or(ResidentKernelError::InvalidShape)?;
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
    let elements = set_element_drafts(input)?;
    if elements.len() > MAX_POWERSET_INPUT_CARDINALITY {
        return Err(ResidentKernelError::InvalidShape);
    }
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

fn element_of(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::F64([element])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let next = f64_set_snapshot_contains(set, *element).ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::Bool([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let next = u8::from(next);
    let changed = *target != next;
    *target = next;
    Ok(changed)
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
    _: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::F64([element])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let next =
        !f64_set_snapshot_contains(set, *element).ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::Bool([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let next = u8::from(next);
    let changed = *target != next;
    *target = next;
    Ok(changed)
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

fn f64_set_values(value: &mech_core::snapshot::Value) -> Option<Vec<f64>> {
    let ValueData::Set(set) = value.data() else {
        return None;
    };
    set.elements()
        .iter()
        .map(|element| match element.data() {
            ValueData::F64(value) => Some(value.to_f64()),
            _ => None,
        })
        .collect()
}

fn snapshots_equal(left: &mech_core::snapshot::Value, right: &mech_core::snapshot::Value) -> bool {
    left.schema() == right.schema()
        && left.schema_key() == right.schema_key()
        && left.shape() == right.shape()
        && f64_set_values(left).is_some_and(|left_values| {
            f64_set_values(right).is_some_and(|right_values| {
                left_values.len() == right_values.len()
                    && left_values
                        .iter()
                        .zip(right_values)
                        .all(|(left, right)| left.to_bits() == right.to_bits())
            })
        })
}

fn write_snapshot(
    target: &mut Option<mech_core::snapshot::Value>,
    next: mech_core::snapshot::Value,
) -> bool {
    let changed = target
        .as_ref()
        .is_none_or(|current| !snapshots_equal(current, &next));
    *target = Some(next);
    changed
}

fn insert(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::F64([element])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let mut values = f64_set_values(set).ok_or(ResidentKernelError::InvalidInput)?;
    values.push(*element);
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let next = build_f64_set_snapshot(
        metadata.schema,
        metadata.schema_key,
        metadata.shape.clone(),
        schemas,
        metadata.exact_cardinality,
        metadata.maximum_cardinality,
        &values,
    )
    .ok_or(ResidentKernelError::InvalidOutput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    Ok(write_snapshot(target, next))
}

fn remove(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(set)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::F64([element])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let next = build_f64_set_snapshot_after_remove(
        metadata.schema,
        metadata.schema_key,
        metadata.shape.clone(),
        schemas,
        metadata.exact_cardinality,
        metadata.maximum_cardinality,
        set,
        *element,
    )
    .ok_or(ResidentKernelError::InvalidOutput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    Ok(write_snapshot(target, next))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(body: SchemaBody) -> mech_core::Schema {
        mech_core::SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap()
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
}

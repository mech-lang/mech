use mech_core::snapshot::{
    build_f64_set_snapshot, build_f64_set_snapshot_after_remove, f64_set_snapshot_contains,
};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FloatWidth, FunctionCatalogBuilder, MResult, OutputConstruction,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeRule, ValueData,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(["set"], "union", bind_union)?;
    builder.insert_resident_factory(["set"], "element-of", bind_element_of)?;
    builder.insert_resident_factory(["set"], "not-element-of", bind_not_element_of)?;
    builder.insert_resident_factory(["set"], "insert", bind_insert)?;
    builder.insert_resident_factory(["set"], "remove", bind_remove)?;

    // Frozen bytecode may still refer to the selected implementation identity.
    builder.insert_resident_factory(["runtime"], "SetUnionFxn", bind_union)?;
    builder.insert_resident_factory(["runtime"], "SetElementOfFxn", bind_element_of)?;
    builder.insert_resident_factory(["runtime"], "SetNotElementOfFxn", bind_not_element_of)?;
    builder.insert_resident_factory(["runtime"], "SetInsertFxn", bind_insert)?;
    builder.insert_resident_factory(["runtime"], "SetRemoveFxn", bind_remove)?;
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
    validate_binary_contract(request, ChangeDetectionPolicy::AlwaysChanged)?;
    if request.inputs.iter().any(|input| {
        input.kind != ResidentValueKind::Snapshot || input.shape != ResidentShape::SCALAR
    }) || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || request
            .inputs
            .iter()
            .any(|input| !is_f64_set(request.schemas.get(input.schema_id)))
        || !is_f64_set(request.schemas.get(request.output.schema_id))
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
    let cardinality = shape
        .resolve_dimension(cardinality)
        .ok()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(
        BoundResidentKernel::new(union, Box::new([])).with_snapshot_output(
            ResidentSnapshotOutput {
                schema: request.output.schema_id,
                schema_key: request.output.schema_key,
                shape,
                cardinality,
            },
        ),
    )
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
    let cardinality = shape
        .resolve_dimension(cardinality)
        .ok()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(
        BoundResidentKernel::new(kernel, Box::new([])).with_snapshot_output(
            ResidentSnapshotOutput {
                schema: request.output.schema_id,
                schema_key: request.output.schema_key,
                shape,
                cardinality,
            },
        ),
    )
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

fn union(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let (
        Some(ResidentValueRef::Snapshot([Some(left)])),
        Some(ResidentValueRef::Snapshot([Some(right)])),
    ) = (inputs.get(0), inputs.get(1))
    else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let values = [left, right]
        .into_iter()
        .flat_map(|value| match value.data() {
            ValueData::Set(set) => Some(set.elements()),
            _ => None,
        })
        .flatten()
        .map(|element| match element.data() {
            ValueData::F64(value) => Some(value.to_f64()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let next = build_f64_set_snapshot(
        metadata.schema,
        metadata.schema_key,
        metadata.shape.clone(),
        metadata.cardinality,
        &values,
    )
    .ok_or(ResidentKernelError::InvalidOutput)?;
    *target = Some(next);
    Ok(true)
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
    let next = build_f64_set_snapshot(
        metadata.schema,
        metadata.schema_key,
        metadata.shape.clone(),
        metadata.cardinality,
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
    let next = build_f64_set_snapshot_after_remove(
        metadata.schema,
        metadata.schema_key,
        metadata.shape.clone(),
        metadata.cardinality,
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
                cardinality: mech_core::DimensionExpr::Constant(3),
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

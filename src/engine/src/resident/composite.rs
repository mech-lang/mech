use mech_core::snapshot::{F64Bits, rebuild_composite_snapshot};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeRule, ValueData,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(["core"], "composite-pack", bind_composite_pack)?;
    Ok(())
}

fn port_matches_schema_body(
    request: &ResidentKernelBindRequest<'_>,
    input: &mech_core::ResidentPortLayout,
    expected: &SchemaBody,
) -> bool {
    request
        .schemas
        .get(input.schema_id)
        .is_some_and(|schema| schema.body() == expected)
}

fn composite_children_match_output_schema(request: &ResidentKernelBindRequest<'_>) -> bool {
    let Some(output) = request.schemas.get(request.output.schema_id) else {
        return false;
    };
    let children = &request.inputs[1..];
    match output.body() {
        SchemaBody::Tuple(elements) => {
            children.len() == elements.len()
                && children
                    .iter()
                    .zip(elements.iter())
                    .all(|(input, expected)| port_matches_schema_body(request, input, expected))
        }
        SchemaBody::Record(fields) => {
            children.len() == fields.len()
                && children
                    .iter()
                    .zip(fields.iter())
                    .all(|(input, field)| port_matches_schema_body(request, input, &field.schema))
        }
        _ => false,
    }
}

fn bind_composite_pack(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != request.inputs.len()
        || contract.inputs.is_empty()
        || contract.outputs.len() != 1
        || !matches!(
            request
                .schemas
                .get(request.output.schema_id)
                .map(|schema| schema.body()),
            Some(SchemaBody::Tuple { .. } | SchemaBody::Record { .. })
        )
        || contract
            .inputs
            .iter()
            .any(|input| input.access != AccessMode::Read || input.delivery != DeliveryMode::Signal)
        || contract
            .inputs
            .iter()
            .zip(request.inputs.iter())
            .any(|(contract, input)| contract.schema != input.schema_id)
        || request.inputs[0].schema_id != request.output.schema_id
        || !composite_children_match_output_schema(request)
        || request.inputs[0].kind != ResidentValueKind::Snapshot
        || request.inputs[0].shape != ResidentShape::SCALAR
        || request.inputs[1..].iter().any(|input| {
            input.shape != ResidentShape::SCALAR
                || !matches!(
                    input.kind,
                    ResidentValueKind::Bool
                        | ResidentValueKind::Index
                        | ResidentValueKind::F64
                        | ResidentValueKind::String
                        | ResidentValueKind::Snapshot
                )
        })
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
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
        || output.change_detection != ChangeDetectionPolicy::AlwaysChanged
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(composite_pack, Box::new([])))
}

fn composite_pack(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(template)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let children = (1..inputs.len())
        .map(|index| match inputs.get(index)? {
            ResidentValueRef::Bool([value]) => Some(ValueData::Bool(*value != 0)),
            ResidentValueRef::Index([value]) => Some(ValueData::Index(*value)),
            ResidentValueRef::F64([value]) => Some(ValueData::F64(F64Bits::from_f64(*value))),
            ResidentValueRef::String([value]) => {
                Some(ValueData::String(value.clone().into_boxed_str()))
            }
            ResidentValueRef::Snapshot([Some(value)]) => Some(value.data().clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(ResidentKernelError::InvalidInput)?
        .into_boxed_slice();
    let next =
        rebuild_composite_snapshot(template, children).ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    *target = Some(next);
    Ok(true)
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

    fn layout(
        schemas: &mech_core::SchemaTable,
        schema_id: mech_core::SchemaId,
        kind: ResidentValueKind,
    ) -> mech_core::ResidentPortLayout {
        mech_core::ResidentPortLayout {
            schema_id,
            schema_key: schemas.entry(schema_id).unwrap().key(),
            kind,
            shape: ResidentShape::SCALAR,
        }
    }

    #[test]
    fn composite_children_require_the_declared_field_schema() {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let scalar = builder
            .insert(schema(SchemaBody::FloatingPoint(
                mech_core::FloatWidth::W64,
            )))
            .unwrap();
        let matrix = builder
            .insert(schema(SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(1),
                    mech_core::DimensionExpr::Constant(1),
                ]
                .into_boxed_slice(),
            }))
            .unwrap();
        let tuple = builder
            .insert(schema(SchemaBody::Tuple(
                vec![SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)].into_boxed_slice(),
            )))
            .unwrap();
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar).unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let (schemas, _) = build.into_parts();
        let contract =
            ResolvedOperationContract::LegacyOpaque(mech_core::LegacyOpaqueOperationContract {
                input_schemas: Box::new([]),
                output_schemas: Box::new([]),
            });
        let output = layout(&schemas, tuple, ResidentValueKind::Snapshot);

        let good = [
            layout(&schemas, tuple, ResidentValueKind::Snapshot),
            layout(&schemas, scalar, ResidentValueKind::F64),
        ];
        assert!(composite_children_match_output_schema(
            &ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &good,
                output,
            }
        ));

        let bad = [
            layout(&schemas, tuple, ResidentValueKind::Snapshot),
            layout(&schemas, matrix, ResidentValueKind::F64),
        ];
        assert!(!composite_children_match_output_schema(
            &ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &bad,
                output,
            }
        ));
    }
}

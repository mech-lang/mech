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
        || request.inputs[0].schema_id != request.output.schema_id
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

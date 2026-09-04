use std::sync::Arc;

use mech_core::snapshot::{F64Bits, SnapshotValidationContext};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, ImplementationMemoryClass, MResult,
    OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError,
    ResidentKernelInputs, ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut,
    ResidentValueRef, ResolvedOperationContract, ResolvedType, SchemaBody, ShapeRule,
    ValueDataDraft, ValueDraft, execute_conversion_draft, plan_explicit_cast,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(
        ["convert"],
        "kind",
        ImplementationMemoryClass::CanonicalFinalize,
        bind_kind_conversion,
    )
}

#[derive(Debug)]
struct ResidentConversionPlan {
    source: SchemaBody,
    target: SchemaBody,
    conversion: mech_core::ConversionPlan,
}

fn bind_kind_conversion(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [input] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let [input_contract] = contract.inputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [output_contract] = contract.outputs.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || input_contract.schema != input.schema_id
        || input_contract.access != AccessMode::Read
        || input_contract.delivery != DeliveryMode::Signal
        || output_contract.schema != request.output.schema_id
        || output_contract.access != AccessMode::Write
        || output_contract.delivery != DeliveryMode::Signal
        || output_contract.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            })
        || output_contract.alias != AliasPolicy::NoAlias
        || output_contract.change_detection != ChangeDetectionPolicy::KernelReported
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let source_schema = request
        .schemas
        .get(input.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let target_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if !layout_matches_schema(input.kind, source_schema.body())
        || !layout_matches_schema(request.output.kind, target_schema.body())
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let source = ResolvedType::from_schema(source_schema, &input.shape_instance)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let target = ResolvedType::from_schema(target_schema, &request.output.shape_instance)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let conversion = plan_explicit_cast(&source, &target)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let plan = ResidentConversionPlan {
        source: source_schema.body().clone(),
        target: target_schema.body().clone(),
        conversion,
    };
    Ok(
        BoundResidentKernel::new(execute_kind_conversion, Box::new([]))
            .with_retained_state(Arc::new(plan))
            .with_snapshot_output(ResidentSnapshotOutput {
                schema: request.output.schema_id,
                schema_key: request.output.schema_key,
                shape: request.output.shape_instance.clone(),
                exact_cardinality: None,
                maximum_cardinality: None,
            })
            .with_snapshot_schemas(request.schemas.clone()),
    )
}

fn layout_matches_schema(kind: ResidentValueKind, schema: &SchemaBody) -> bool {
    let scalar = match schema {
        SchemaBody::Matrix { element, .. } => element.as_ref(),
        schema => schema,
    };
    match scalar {
        SchemaBody::Bool => kind == ResidentValueKind::Bool,
        SchemaBody::Index => kind == ResidentValueKind::Index,
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => kind == ResidentValueKind::F64,
        SchemaBody::String => kind == ResidentValueKind::String,
        _ => kind == ResidentValueKind::Snapshot,
    }
}

fn resident_input_draft(
    input: ResidentValueRef<'_>,
    schema: &SchemaBody,
) -> Result<ValueDataDraft, ResidentKernelError> {
    if let ResidentValueRef::Snapshot([Some(value)]) = input {
        return value
            .canonical_data_draft()
            .map_err(|_| ResidentKernelError::InvalidInput);
    }
    let values: Vec<ValueDataDraft> = match input {
        ResidentValueRef::Bool(values) => values
            .iter()
            .map(|value| ValueDataDraft::Bool(*value != 0))
            .collect(),
        ResidentValueRef::Index(values) => {
            values.iter().copied().map(ValueDataDraft::Index).collect()
        }
        ResidentValueRef::F64(values) => values
            .iter()
            .copied()
            .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
            .collect(),
        ResidentValueRef::String(values) => {
            values.iter().cloned().map(ValueDataDraft::String).collect()
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
    };
    match schema {
        SchemaBody::Matrix { .. } => Ok(ValueDataDraft::Matrix(values.into_boxed_slice())),
        _ => {
            let [value] = values
                .try_into()
                .map_err(|_| ResidentKernelError::InvalidShape)?;
            Ok(value)
        }
    }
}

fn converted_elements(
    draft: ValueDataDraft,
    schema: &SchemaBody,
) -> Result<Vec<ValueDataDraft>, ResidentKernelError> {
    match (schema, draft) {
        (SchemaBody::Matrix { .. }, ValueDataDraft::Matrix(elements)) => Ok(elements.into_vec()),
        (SchemaBody::Matrix { .. }, _) => Err(ResidentKernelError::InvalidShape),
        (_, value) => Ok(vec![value]),
    }
}

fn execute_kind_conversion(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(input) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let plan = kernel
        .retained_state::<ResidentConversionPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let source = resident_input_draft(input, &plan.source)?;
    let converted = execute_conversion_draft(source, &plan.conversion.step)
        .map_err(|_| ResidentKernelError::Arithmetic)?;
    match output {
        ResidentValueMut::Bool(target) => {
            let next = converted_elements(converted, &plan.target)?
                .into_iter()
                .map(|value| match value {
                    ValueDataDraft::Bool(value) => Ok(u8::from(value)),
                    _ => Err(ResidentKernelError::InvalidOutput),
                })
                .collect::<Result<Vec<_>, _>>()?;
            publish_slice(target, next)
        }
        ResidentValueMut::Index(target) => {
            let next = converted_elements(converted, &plan.target)?
                .into_iter()
                .map(|value| match value {
                    ValueDataDraft::Index(value) => Ok(value),
                    _ => Err(ResidentKernelError::InvalidOutput),
                })
                .collect::<Result<Vec<_>, _>>()?;
            publish_slice(target, next)
        }
        ResidentValueMut::F64(target) => {
            let next = converted_elements(converted, &plan.target)?
                .into_iter()
                .map(|value| match value {
                    ValueDataDraft::F64(value) => Ok(value.to_f64()),
                    _ => Err(ResidentKernelError::InvalidOutput),
                })
                .collect::<Result<Vec<_>, _>>()?;
            if target.len() != next.len() {
                return Err(ResidentKernelError::InvalidShape);
            }
            let changed = target
                .iter()
                .zip(&next)
                .any(|(current, next)| current.to_bits() != next.to_bits());
            target.copy_from_slice(&next);
            Ok(changed)
        }
        ResidentValueMut::String(target) => {
            let next = converted_elements(converted, &plan.target)?
                .into_iter()
                .map(|value| match value {
                    ValueDataDraft::String(value) => Ok(value),
                    _ => Err(ResidentKernelError::InvalidOutput),
                })
                .collect::<Result<Vec<_>, _>>()?;
            publish_slice(target, next)
        }
        ResidentValueMut::Snapshot([target]) => {
            let metadata = kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let schemas = kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let next = ValueDraft {
                schema: metadata.schema,
                shape_values: metadata
                    .shape
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
                data: converted,
            }
            .finalize(&SnapshotValidationContext::new(schemas))
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
            if next.schema_key() != metadata.schema_key {
                return Err(ResidentKernelError::InvalidOutput);
            }
            let changed = match target.as_ref() {
                Some(current) => !current
                    .language_eq(schemas, &next, schemas)
                    .map_err(|_| ResidentKernelError::InvalidOutput)?,
                None => true,
            };
            *target = Some(next);
            Ok(changed)
        }
        ResidentValueMut::Snapshot(_) => Err(ResidentKernelError::InvalidShape),
    }
}

fn publish_slice<T: PartialEq>(
    target: &mut [T],
    next: Vec<T>,
) -> Result<bool, ResidentKernelError> {
    if target.len() != next.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let changed = target != next;
    for (target, next) in target.iter_mut().zip(next) {
        *target = next;
    }
    Ok(changed)
}

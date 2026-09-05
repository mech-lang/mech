use std::sync::Arc;

use mech_core::snapshot::{F64Bits, SequenceView, SnapshotValidationContext};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, ImplementationMemoryClass, MResult,
    OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError,
    ResidentKernelInputs, ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut,
    ResidentValueRef, ResolvedOperationContract, ResolvedType, SchemaBody, ShapeRule,
    ValueDataDraft, ValueDraft, execute_conversion_draft, plan_explicit_cast,
};

#[derive(Default)]
struct DisplayByteCounter(usize);

impl core::fmt::Write for DisplayByteCounter {
    fn write_str(&mut self, value: &str) -> core::fmt::Result {
        self.0 = self.0.checked_add(value.len()).ok_or(core::fmt::Error)?;
        Ok(())
    }
}

fn displayed_bytes(value: impl core::fmt::Display) -> Result<usize, ResidentKernelError> {
    use core::fmt::Write;

    let mut counter = DisplayByteCounter::default();
    write!(&mut counter, "{value}").map_err(|_| ResidentKernelError::InvalidShape)?;
    Ok(counter.0)
}

fn projected_string_value_bytes(
    value: &mech_core::ValueData,
) -> Result<usize, ResidentKernelError> {
    use mech_core::ValueData;

    match value {
        ValueData::U8(value) => displayed_bytes(value),
        ValueData::U16(value) => displayed_bytes(value),
        ValueData::U32(value) => displayed_bytes(value),
        ValueData::U64(value) => displayed_bytes(value),
        ValueData::U128(value) => displayed_bytes(value),
        ValueData::I8(value) => displayed_bytes(value),
        ValueData::I16(value) => displayed_bytes(value),
        ValueData::I32(value) => displayed_bytes(value),
        ValueData::I64(value) => displayed_bytes(value),
        ValueData::I128(value) => displayed_bytes(value),
        ValueData::F32(value) => displayed_bytes(value.to_f32()),
        ValueData::F64(value) => displayed_bytes(value.to_f64()),
        ValueData::Complex32(value) => displayed_bytes(value.real().to_f32())?
            .checked_add(displayed_bytes(value.imaginary().to_f32())?)
            .and_then(|bytes| bytes.checked_add(2))
            .ok_or(ResidentKernelError::InvalidShape),
        ValueData::Complex64(value) => displayed_bytes(value.real().to_f64())?
            .checked_add(displayed_bytes(value.imaginary().to_f64())?)
            .and_then(|bytes| bytes.checked_add(2))
            .ok_or(ResidentKernelError::InvalidShape),
        ValueData::Rational64(value) => displayed_bytes(value.numerator())?
            .checked_add(displayed_bytes(value.denominator())?)
            .and_then(|bytes| bytes.checked_add(1))
            .ok_or(ResidentKernelError::InvalidShape),
        ValueData::Bool(value) => Ok(if *value { 4 } else { 5 }),
        ValueData::String(value) => Ok(value.len()),
        _ => Err(ResidentKernelError::InvalidInput),
    }
}

fn projected_display_sequence<T: core::fmt::Display>(
    values: &[T],
) -> Result<usize, ResidentKernelError> {
    values.iter().try_fold(0usize, |bytes, value| {
        bytes
            .checked_add(displayed_bytes(value)?)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

fn projected_string_payload(input: ResidentValueRef<'_>) -> Result<usize, ResidentKernelError> {
    match input {
        ResidentValueRef::Bool(values) => values.iter().try_fold(0usize, |bytes, value| {
            let next = match *value {
                0 => 5,
                1 => 4,
                _ => return Err(ResidentKernelError::InvalidInput),
            };
            bytes
                .checked_add(next)
                .ok_or(ResidentKernelError::InvalidShape)
        }),
        ResidentValueRef::Index(values) => projected_display_sequence(values),
        ResidentValueRef::F64(values) => projected_display_sequence(values),
        ResidentValueRef::String(values) => values.iter().try_fold(0usize, |bytes, value| {
            bytes
                .checked_add(value.len())
                .ok_or(ResidentKernelError::InvalidShape)
        }),
        ResidentValueRef::Snapshot([Some(value)]) => match value.data() {
            mech_core::ValueData::Matrix(matrix) => match matrix.elements() {
                SequenceView::U8(values) => projected_display_sequence(values),
                SequenceView::U16(values) => projected_display_sequence(values),
                SequenceView::U32(values) => projected_display_sequence(values),
                SequenceView::U64(values) => projected_display_sequence(values),
                SequenceView::U128(values) => projected_display_sequence(values),
                SequenceView::I8(values) => projected_display_sequence(values),
                SequenceView::I16(values) => projected_display_sequence(values),
                SequenceView::I32(values) => projected_display_sequence(values),
                SequenceView::I64(values) => projected_display_sequence(values),
                SequenceView::I128(values) => projected_display_sequence(values),
                SequenceView::F32(values) => values.iter().try_fold(0usize, |bytes, value| {
                    bytes
                        .checked_add(displayed_bytes(value.to_f32())?)
                        .ok_or(ResidentKernelError::InvalidShape)
                }),
                SequenceView::F64(values) => values.iter().try_fold(0usize, |bytes, value| {
                    bytes
                        .checked_add(displayed_bytes(value.to_f64())?)
                        .ok_or(ResidentKernelError::InvalidShape)
                }),
                SequenceView::Complex32(values) => {
                    values.iter().try_fold(0usize, |bytes, value| {
                        bytes
                            .checked_add(projected_string_value_bytes(
                                &mech_core::ValueData::Complex32(*value),
                            )?)
                            .ok_or(ResidentKernelError::InvalidShape)
                    })
                }
                SequenceView::Complex64(values) => {
                    values.iter().try_fold(0usize, |bytes, value| {
                        bytes
                            .checked_add(projected_string_value_bytes(
                                &mech_core::ValueData::Complex64(*value),
                            )?)
                            .ok_or(ResidentKernelError::InvalidShape)
                    })
                }
                SequenceView::Rational64(values) => {
                    values.iter().try_fold(0usize, |bytes, value| {
                        bytes
                            .checked_add(projected_string_value_bytes(
                                &mech_core::ValueData::Rational64(value.clone()),
                            )?)
                            .ok_or(ResidentKernelError::InvalidShape)
                    })
                }
                SequenceView::Bool(values) => values.iter().try_fold(0usize, |bytes, value| {
                    bytes
                        .checked_add(if *value { 4 } else { 5 })
                        .ok_or(ResidentKernelError::InvalidShape)
                }),
                SequenceView::String(values) => values.iter().try_fold(0usize, |bytes, value| {
                    bytes
                        .checked_add(value.len())
                        .ok_or(ResidentKernelError::InvalidShape)
                }),
                SequenceView::Values(values) => values.iter().try_fold(0usize, |bytes, value| {
                    bytes
                        .checked_add(projected_string_value_bytes(value)?)
                        .ok_or(ResidentKernelError::InvalidShape)
                }),
                SequenceView::Id(_) | SequenceView::Index(_) | SequenceView::Unit(_) => {
                    Err(ResidentKernelError::InvalidInput)
                }
            },
            value => projected_string_value_bytes(value),
        },
        ResidentValueRef::Snapshot(_) => Err(ResidentKernelError::InvalidInput),
    }
}

fn target_is_string(schema: &SchemaBody) -> bool {
    match schema {
        SchemaBody::String => true,
        SchemaBody::Matrix { element, .. } => element.as_ref() == &SchemaBody::String,
        _ => false,
    }
}

fn logical_input_len(input: ResidentValueRef<'_>) -> Result<usize, ResidentKernelError> {
    match input {
        ResidentValueRef::Snapshot([Some(value)]) => Ok(match value.data() {
            mech_core::ValueData::Matrix(matrix) => matrix.elements().len(),
            _ => 1,
        }),
        ResidentValueRef::Snapshot(_) => Err(ResidentKernelError::InvalidInput),
        input => Ok(input.len()),
    }
}

fn preflight_string_conversion(
    kernel: &BoundResidentKernel,
    input: ResidentValueRef<'_>,
    output: &ResidentValueMut<'_>,
    target_schema: &SchemaBody,
) -> Result<(), ResidentKernelError> {
    if !target_is_string(target_schema) {
        return Ok(());
    }
    let ResidentValueMut::String(current) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let output_len = logical_input_len(input)?;
    if output_len != current.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let output_len_u64 = super::budget::checked_u64(output_len)?;
    let mut meter = super::budget::ResidentBudgetMeter::default();
    let draft_container_bytes = super::budget::checked_u64(
        output_len
            .checked_mul(core::mem::size_of::<ValueDataDraft>())
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let (source_temporary_bytes, source_cloned_bytes, source_nodes) = match input {
        ResidentValueRef::Snapshot([Some(value)]) => {
            let schemas = kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidInput)?;
            let footprint =
                super::budget::measure_canonical_value_footprint(&mut meter, value, schemas)?;
            (
                footprint
                    .retained_bytes
                    .checked_add(draft_container_bytes)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                footprint.retained_bytes,
                footprint
                    .node_count
                    .checked_add(output_len_u64)
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
        ResidentValueRef::String(values) => {
            let payload = values.iter().try_fold(0u64, |bytes, value| {
                bytes
                    .checked_add(super::budget::checked_u64(value.len())?)
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
            (
                payload
                    .checked_add(draft_container_bytes)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                payload,
                super::budget::checked_u64(
                    output_len
                        .checked_add(1)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )?,
            )
        }
        input => (
            draft_container_bytes,
            0,
            super::budget::checked_u64(input.len())?,
        ),
    };
    let output_payload = super::budget::checked_u64(projected_string_payload(input)?)?;
    let current_payload = current.iter().try_fold(0u64, |bytes, value| {
        bytes
            .checked_add(super::budget::checked_u64(value.len())?)
            .ok_or(ResidentKernelError::InvalidShape)
    })?;
    // Comparing every current and converted String cannot inspect more than
    // both complete payloads. Use that deterministic fail-closed bound before
    // conversion formatting allocates its first String.
    let publication_work = current_payload
        .checked_add(output_payload)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_containers = super::budget::checked_u64(
        current
            .len()
            .checked_mul(core::mem::size_of::<String>())
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let output_bytes = output_containers
        .checked_add(output_payload)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_nodes = super::budget::checked_u64(
        output_len
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    let measured = meter.estimate();
    super::budget::PreparedMutationPlan::new(
        (),
        super::budget::PublishedOutputFootprint {
            elements: output_len_u64,
            retained_bytes: output_bytes,
            retained_nodes: output_nodes,
        },
        super::budget::MutationRetainedNodeFootprint {
            current_persistent: output_nodes
                .checked_add(source_nodes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            normalized_plan: 0,
            temporary_draft: source_nodes
                .checked_add(
                    output_nodes
                        .checked_mul(2)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )
                .ok_or(ResidentKernelError::InvalidShape)?,
        },
        super::budget::resident_cost! {
            comparison_work: measured
                .comparison_work()
                .checked_add(publication_work)
                .ok_or(ResidentKernelError::InvalidShape)?,
            compute_work: measured
                .compute_work()
                .checked_add(publication_work)
                .and_then(|work| work.checked_add(output_len_u64))
                .ok_or(ResidentKernelError::InvalidShape)?,
            temporary_bytes: source_temporary_bytes
                .checked_add(draft_container_bytes)
                .and_then(|bytes| bytes.checked_add(output_payload))
                .and_then(|bytes| bytes.checked_add(output_containers))
                .and_then(|bytes| bytes.checked_add(output_payload))
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes: source_cloned_bytes
                .checked_add(output_payload)
                .ok_or(ResidentKernelError::InvalidShape)?,
            ..super::budget::KernelCostEstimate::default()
        },
    )?
    .admit()?
    .into_plan();
    Ok(())
}

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
    preflight_string_conversion(kernel, input, &output, &plan.target)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{DimensionExpr, IntegerWidth, SchemaDraft, SchemaTableBuilder};

    #[test]
    fn snapshot_matrix_string_conversion_plans_logical_elements_before_drafting() {
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        };
        let schema = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: matrix_body,
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let pending = builder.insert(schema).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(pending).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::U64(1), ValueDataDraft::U64(2)].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let source = [Some(value)];
        let input = ResidentValueRef::Snapshot(&source);
        let mut current = ["old".to_owned(), "values".to_owned()];
        let output = ResidentValueMut::String(&mut current);
        let kernel = BoundResidentKernel::new(execute_kind_conversion, Box::new([]))
            .with_snapshot_schemas(schemas);
        let target = SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        };

        assert_eq!(
            preflight_string_conversion(&kernel, input, &output, &target),
            Ok(()),
        );
        assert_eq!(current, ["old", "values"]);
    }
}

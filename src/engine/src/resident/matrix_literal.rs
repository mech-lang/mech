use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FloatWidth, FunctionCatalogBuilder,
    ImplementationMemoryClass, MResult, OutputConstruction, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs, ResidentShape,
    ResidentSnapshotOutput, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeRule, ValueDataDraft, ValueDraft,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MatrixLiteralPlan {
    rows: usize,
    columns: usize,
    kind: ResidentValueKind,
}

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(
        ["matrix"],
        "literal",
        ImplementationMemoryClass::CanonicalFinalize,
        bind_matrix_literal,
    )?;
    Ok(())
}

fn resident_element_kind(element: &SchemaBody) -> Option<ResidentValueKind> {
    match element {
        SchemaBody::Bool => Some(ResidentValueKind::Bool),
        SchemaBody::Index => Some(ResidentValueKind::Index),
        SchemaBody::FloatingPoint(FloatWidth::W64) => Some(ResidentValueKind::F64),
        SchemaBody::String => Some(ResidentValueKind::String),
        SchemaBody::Atom(_)
        | SchemaBody::Dynamic
        | SchemaBody::Enum { .. }
        | SchemaBody::Option(_)
        | SchemaBody::Tuple(_)
        | SchemaBody::Record(_)
        | SchemaBody::Table { .. }
        | SchemaBody::Set { .. }
        | SchemaBody::Map { .. }
        | SchemaBody::ReifiedType
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(FloatWidth::W32)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::Id
        | SchemaBody::Matrix { .. } => Some(ResidentValueKind::Snapshot),
    }
}

fn bind_matrix_literal(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let Some(output_schema) = request.schemas.get(request.output.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = output_schema.body()
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let [rows, columns] = dimensions.as_ref() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let rows = request
        .output
        .shape_instance
        .resolve_dimension(rows)
        .ok()
        .and_then(|rows| usize::try_from(rows).ok())
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let columns = request
        .output
        .shape_instance
        .resolve_dimension(columns)
        .ok()
        .and_then(|columns| usize::try_from(columns).ok())
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let rows_u32 = u32::try_from(rows).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let columns_u32 =
        u32::try_from(columns).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let kind = resident_element_kind(element).ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_shape = if kind == ResidentValueKind::Snapshot {
        ResidentShape::SCALAR
    } else {
        ResidentShape {
            rows: rows_u32,
            columns: columns_u32,
        }
    };
    if request.inputs.len() != count
        || request.output.kind != kind
        || request.output.shape != output_shape
        || request.inputs.iter().any(|input| {
            input.kind != kind
                || input.shape != ResidentShape::SCALAR
                || request.schemas.get(input.schema_id).is_none_or(|schema| {
                    !schema.dimension_parameters().is_empty() || schema.body() != element.as_ref()
                })
        })
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != request.inputs.len()
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(contract, input)| {
                contract.schema != input.schema_id
                    || contract.access != AccessMode::Read
                    || contract.delivery != DeliveryMode::Signal
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
        || output.change_detection != ChangeDetectionPolicy::AlwaysChanged
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let kernel = BoundResidentKernel::new(matrix_literal, Box::new([])).with_retained_state(
        Arc::new(MatrixLiteralPlan {
            rows,
            columns,
            kind,
        }),
    );
    if kind == ResidentValueKind::Snapshot {
        Ok(kernel
            .with_snapshot_output(ResidentSnapshotOutput {
                schema: request.output.schema_id,
                schema_key: request.output.schema_key,
                shape: request.output.shape_instance.clone(),
                exact_cardinality: None,
                maximum_cardinality: None,
            })
            .with_snapshot_schemas(request.schemas.clone()))
    } else {
        Ok(kernel)
    }
}

fn target_index(source: usize, rows: usize, columns: usize) -> usize {
    let row = source / columns;
    let column = source % columns;
    column * rows + row
}

fn checked_cost_usize(value: u64) -> Result<usize, ResidentKernelError> {
    usize::try_from(value).map_err(|_| ResidentKernelError::InvalidShape)
}

fn admit_literal_stage(
    count: usize,
    output_bytes: usize,
    temporary_bytes: usize,
    container_bytes: usize,
    cloned_bytes: usize,
    retained_nodes: usize,
) -> Result<usize, ResidentKernelError> {
    super::budget::PreparedKernel::new(
        count,
        super::budget::resident_cost! {
            compute_work: count
                .checked_mul(2)
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: count,
            output_bytes,
            temporary_bytes,
            cloned_bytes,
            container_bytes,
            retained_nodes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()
    .map(super::budget::AdmittedKernel::into_plan)
}

fn matrix_literal(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = kernel
        .retained_state::<MatrixLiteralPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let count = plan
        .rows
        .checked_mul(plan.columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_len = if plan.kind == ResidentValueKind::Snapshot {
        1
    } else {
        count
    };
    if inputs.len() != count || output.kind() != plan.kind || output.len() != output_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    if count == 0 && plan.kind != ResidentValueKind::Snapshot {
        return Ok(true);
    }

    match output {
        ResidentValueMut::Bool(target) if plan.kind == ResidentValueKind::Bool => {
            for source in 0..count {
                let Some(ResidentValueRef::Bool([value])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                if *value > 1 {
                    return Err(ResidentKernelError::InvalidInput);
                }
            }
            let count = admit_literal_stage(count, count, 0, count, 0, count)?;
            let mut next = vec![0_u8; count];
            for source in 0..count {
                let Some(ResidentValueRef::Bool([value])) = inputs.get(source) else {
                    unreachable!("literal inputs were validated before admission")
                };
                next[target_index(source, plan.rows, plan.columns)] = *value;
            }
            target.copy_from_slice(&next);
        }
        ResidentValueMut::Index(target) if plan.kind == ResidentValueKind::Index => {
            for source in 0..count {
                let Some(ResidentValueRef::Index([value])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                let _ = value;
            }
            let bytes = count
                .checked_mul(core::mem::size_of::<u64>())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let count = admit_literal_stage(count, bytes, 0, bytes, 0, count)?;
            let mut next = vec![0_u64; count];
            for source in 0..count {
                let Some(ResidentValueRef::Index([value])) = inputs.get(source) else {
                    unreachable!("literal inputs were validated before admission")
                };
                next[target_index(source, plan.rows, plan.columns)] = *value;
            }
            target.copy_from_slice(&next);
        }
        ResidentValueMut::F64(target) if plan.kind == ResidentValueKind::F64 => {
            for source in 0..count {
                let Some(ResidentValueRef::F64([value])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                let _ = value;
            }
            let bytes = count
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let count = admit_literal_stage(count, bytes, 0, bytes, 0, count)?;
            let mut next = vec![0.0_f64; count];
            for source in 0..count {
                let Some(ResidentValueRef::F64([value])) = inputs.get(source) else {
                    unreachable!("literal inputs were validated before admission")
                };
                next[target_index(source, plan.rows, plan.columns)] = *value;
            }
            target.copy_from_slice(&next);
        }
        ResidentValueMut::String(target) if plan.kind == ResidentValueKind::String => {
            let mut payload_bytes = 0usize;
            for source in 0..count {
                let Some(ResidentValueRef::String([value])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                payload_bytes = payload_bytes
                    .checked_add(value.len())
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            let container_bytes = count
                .checked_mul(core::mem::size_of::<String>())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let output_bytes = payload_bytes
                .checked_add(container_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let count = admit_literal_stage(
                count,
                output_bytes,
                payload_bytes,
                container_bytes,
                payload_bytes,
                count,
            )?;
            let mut next = vec![String::new(); count];
            for source in 0..count {
                let Some(ResidentValueRef::String([value])) = inputs.get(source) else {
                    unreachable!("literal inputs were validated before admission")
                };
                next[target_index(source, plan.rows, plan.columns)] = value.clone();
            }
            for (target, value) in target.iter_mut().zip(next) {
                *target = value;
            }
        }
        ResidentValueMut::Snapshot([target]) if plan.kind == ResidentValueKind::Snapshot => {
            let schemas = kernel
                .snapshot_schemas()
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let metadata = kernel
                .snapshot_output()
                .ok_or(ResidentKernelError::InvalidOutput)?;
            let mut retained_bytes = 0usize;
            let mut input_nodes = 0u64;
            let mut finalization_work = 0u64;
            let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
            for source in 0..count {
                let Some(ResidentValueRef::Snapshot([Some(value)])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                let footprint = super::budget::measure_canonical_value_footprint(
                    &mut footprint_meter,
                    value,
                    schemas,
                )?;
                retained_bytes = retained_bytes
                    .checked_add(checked_cost_usize(footprint.retained_bytes)?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                input_nodes = input_nodes
                    .checked_add(footprint.node_count)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let schema = value
                    .validate_against(schemas)
                    .map_err(|_| ResidentKernelError::InvalidInput)?;
                finalization_work = finalization_work
                    .checked_add(super::budget::preflight_canonical_data_finalization(
                        &mut footprint_meter,
                        schema.body(),
                        value.data(),
                    )?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            if let Some(previous) = target.as_ref() {
                super::budget::measure_canonical_value_footprint(
                    &mut footprint_meter,
                    previous,
                    schemas,
                )?;
            }
            let container_bytes = count
                .checked_mul(core::mem::size_of::<ValueDataDraft>())
                .ok_or(ResidentKernelError::InvalidShape)?;
            let output_bytes = retained_bytes
                .checked_add(container_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let count_u64 = super::budget::checked_u64(count)?;
            let child_data_nodes = input_nodes
                .checked_sub(count_u64)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let draft_nodes = child_data_nodes
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let final_nodes = draft_nodes
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let measured = footprint_meter.estimate();
            let cost = super::budget::resident_cost! {
                comparison_work: measured.comparison_work,
                compute_work: measured.compute_work
                    .checked_add(count_u64.checked_mul(2).ok_or(ResidentKernelError::InvalidShape)?)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                output_elements: count,
                output_bytes,
                temporary_bytes: retained_bytes
                    .checked_mul(2)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                cloned_bytes: retained_bytes
                    .checked_mul(2)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                container_bytes,
                retained_nodes: super::budget::checked_cost_sum(&[
                    measured.retained_nodes,
                    draft_nodes,
                    final_nodes,
                ])?,
                ..super::budget::KernelCostEstimate::default()
            };
            let (count, canonicalization_work_limit) =
                super::budget::PreparedKernel::new((count, finalization_work), cost)
                    .admit()?
                    .into_plan();
            let mut elements = Vec::with_capacity(count);
            for source in 0..count {
                let Some(ResidentValueRef::Snapshot([Some(value)])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                elements.push(
                    value
                        .canonical_data_draft()
                        .map_err(|_| ResidentKernelError::InvalidInput)?,
                );
            }
            let budget = mech_core::snapshot::SnapshotCanonicalizationBudget::new(
                canonicalization_work_limit,
            );
            let next = ValueDraft {
                schema: metadata.schema,
                shape_values: metadata
                    .shape
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
                data: ValueDataDraft::Matrix(elements.into_boxed_slice()),
            }
            .finalize(
                &SnapshotValidationContext::new(schemas).with_canonicalization_budget(&budget),
            )
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
            if next.schema_key() != metadata.schema_key {
                return Err(ResidentKernelError::InvalidOutput);
            }
            *target = Some(next);
        }
        _ => return Err(ResidentKernelError::InvalidOutput),
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DeclaredOperationContract, ResidentPortLayout, ResolvedInputPort, ResolvedOutputPort,
        SchemaDraft, SchemaId, SchemaTable, SchemaTableBuilder, ValueDataDraft, ValueDraft,
        snapshot::{F64Bits, SnapshotValidationContext},
    };

    struct Inputs<'a>(Vec<ResidentValueRef<'a>>);

    impl ResidentKernelInputs for Inputs<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index).copied()
        }
    }

    fn schema(body: SchemaBody) -> mech_core::Schema {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap()
    }

    fn f64_schemas(rows: u64, columns: u64) -> (SchemaTable, SchemaId, SchemaId) {
        schemas_for(SchemaBody::FloatingPoint(FloatWidth::W64), rows, columns)
    }

    fn schemas_for(
        element_body: SchemaBody,
        rows: u64,
        columns: u64,
    ) -> (SchemaTable, SchemaId, SchemaId) {
        let mut builder = SchemaTableBuilder::new();
        let scalar = builder.insert(schema(element_body.clone())).unwrap();
        let matrix = builder
            .insert(schema(SchemaBody::Matrix {
                element: Box::new(element_body),
                dimensions: vec![
                    DimensionExpr::Constant(rows),
                    DimensionExpr::Constant(columns),
                ]
                .into_boxed_slice(),
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let scalar = build.resolve(scalar).unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let (schemas, _) = build.into_parts();
        (schemas, scalar, matrix)
    }

    fn layout(
        schemas: &SchemaTable,
        schema: SchemaId,
        kind: ResidentValueKind,
        shape: ResidentShape,
    ) -> ResidentPortLayout {
        ResidentPortLayout {
            schema_id: schema,
            schema_key: schemas.entry(schema).unwrap().key(),
            kind,
            shape,
            shape_instance: schemas
                .get(schema)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
        }
    }

    fn contract(input: SchemaId, output: SchemaId, count: usize) -> ResolvedOperationContract {
        ResolvedOperationContract::Declared(DeclaredOperationContract {
            inputs: (0..count)
                .map(|_| ResolvedInputPort {
                    schema: input,
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            outputs: vec![ResolvedOutputPort {
                schema: output,
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
        })
    }

    #[test]
    fn f64_literal_writes_logical_rows_into_column_major_storage() {
        let (schemas, scalar, matrix) = f64_schemas(2, 3);
        let contract = contract(scalar, matrix, 6);
        let inputs = vec![
            layout(
                &schemas,
                scalar,
                ResidentValueKind::F64,
                ResidentShape::SCALAR,
            );
            6
        ];
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &inputs,
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::F64,
                ResidentShape {
                    rows: 2,
                    columns: 3,
                },
            ),
        })
        .unwrap();
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let inputs = Inputs(
            values
                .iter()
                .map(|value| ResidentValueRef::F64(core::slice::from_ref(value)))
                .collect(),
        );
        let mut output = [0.0; 6];
        assert!(
            kernel
                .execute(&inputs, ResidentValueMut::F64(&mut output))
                .unwrap()
        );
        assert_eq!(output, [1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn scalar_resident_families_execute_one_by_one_literals() {
        let (schemas, scalar, matrix) = f64_schemas(1, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 1),
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                scalar,
                ResidentValueKind::F64,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::F64,
                ResidentShape {
                    rows: 1,
                    columns: 1,
                },
            ),
        })
        .unwrap();
        let source = [3.5];
        let mut output = [0.0];
        kernel
            .execute(
                &Inputs(vec![ResidentValueRef::F64(&source)]),
                ResidentValueMut::F64(&mut output),
            )
            .unwrap();
        assert_eq!(output, source);

        let (schemas, scalar, matrix) = schemas_for(SchemaBody::Bool, 1, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 1),
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                scalar,
                ResidentValueKind::Bool,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::Bool,
                ResidentShape {
                    rows: 1,
                    columns: 1,
                },
            ),
        })
        .unwrap();
        let source = [1];
        let mut output = [0];
        kernel
            .execute(
                &Inputs(vec![ResidentValueRef::Bool(&source)]),
                ResidentValueMut::Bool(&mut output),
            )
            .unwrap();
        assert_eq!(output, source);

        let (schemas, scalar, matrix) = schemas_for(SchemaBody::Index, 1, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 1),
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                scalar,
                ResidentValueKind::Index,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::Index,
                ResidentShape {
                    rows: 1,
                    columns: 1,
                },
            ),
        })
        .unwrap();
        let source = [42];
        let mut output = [0];
        kernel
            .execute(
                &Inputs(vec![ResidentValueRef::Index(&source)]),
                ResidentValueMut::Index(&mut output),
            )
            .unwrap();
        assert_eq!(output, source);

        let (schemas, scalar, matrix) = schemas_for(SchemaBody::String, 1, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 1),
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                scalar,
                ResidentValueKind::String,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::String,
                ResidentShape {
                    rows: 1,
                    columns: 1,
                },
            ),
        })
        .unwrap();
        let source = ["matrix".to_owned()];
        let mut output = [String::new()];
        kernel
            .execute(
                &Inputs(vec![ResidentValueRef::String(&source)]),
                ResidentValueMut::String(&mut output),
            )
            .unwrap();
        assert_eq!(output, source);
    }

    #[test]
    fn dense_literal_first_middle_and_last_failures_are_atomic() {
        let (schemas, scalar, matrix) = schemas_for(SchemaBody::Bool, 1, 3);
        let input_layout = layout(
            &schemas,
            scalar,
            ResidentValueKind::Bool,
            ResidentShape::SCALAR,
        );
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 3),
            schemas: &schemas,
            inputs: &[input_layout.clone(), input_layout.clone(), input_layout],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::Bool,
                ResidentShape {
                    rows: 1,
                    columns: 3,
                },
            ),
        })
        .unwrap();

        for invalid in 0..3 {
            let mut values = [[1_u8]; 3];
            values[invalid] = [2];
            let inputs = Inputs(
                values
                    .iter()
                    .map(|value| ResidentValueRef::Bool(value))
                    .collect(),
            );
            let mut output = [9_u8, 9, 9];
            assert_eq!(
                kernel.execute(&inputs, ResidentValueMut::Bool(&mut output)),
                Err(ResidentKernelError::InvalidInput)
            );
            assert_eq!(output, [9, 9, 9]);
        }
    }

    #[test]
    fn string_literal_rejects_clone_amplification_before_publication() {
        let (schemas, scalar, matrix) = schemas_for(SchemaBody::String, 1, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 1),
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                scalar,
                ResidentValueKind::String,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::String,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let source = ["x".repeat(16 * 1024 * 1024 + 1)];
        let mut output = ["unchanged".to_owned()];
        assert_eq!(
            kernel.execute(
                &Inputs(vec![ResidentValueRef::String(&source)]),
                ResidentValueMut::String(&mut output),
            ),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(output, ["unchanged"]);
    }

    #[test]
    fn snapshot_literal_preflights_nested_set_finalization_before_cloning() {
        let set_body = SchemaBody::Set {
            element: Box::new(SchemaBody::String),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        };
        let (schemas, scalar, matrix) = schemas_for(set_body, 1, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 1),
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                scalar,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let element = ValueDraft {
            schema: scalar,
            shape_values: Box::new([]),
            data: ValueDataDraft::Set(
                (0..48)
                    .map(|index| ValueDataDraft::String(format!("{}-{index:04}", "x".repeat(500))))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let source = [Some(element)];
        let mut output = [None];

        assert_eq!(
            kernel.execute(
                &Inputs(vec![ResidentValueRef::Snapshot(&source)]),
                ResidentValueMut::Snapshot(&mut output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(output[0].is_none());
    }

    #[test]
    fn binder_rejects_wrong_counts_and_shapes_but_accepts_snapshot_elements() {
        let (schemas, scalar, matrix) = f64_schemas(1, 1);
        let scalar_layout = layout(
            &schemas,
            scalar,
            ResidentValueKind::F64,
            ResidentShape::SCALAR,
        );
        let output_layout = layout(
            &schemas,
            matrix,
            ResidentValueKind::F64,
            ResidentShape {
                rows: 1,
                columns: 1,
            },
        );
        assert!(matches!(
            bind_matrix_literal(&ResidentKernelBindRequest {
                contract: &contract(scalar, matrix, 0),
                schemas: &schemas,
                inputs: &[],
                output: output_layout.clone(),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(matches!(
            bind_matrix_literal(&ResidentKernelBindRequest {
                contract: &contract(matrix, matrix, 1),
                schemas: &schemas,
                inputs: &[layout(
                    &schemas,
                    matrix,
                    ResidentValueKind::F64,
                    ResidentShape::SCALAR,
                )],
                output: output_layout.clone(),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
        assert!(matches!(
            bind_matrix_literal(&ResidentKernelBindRequest {
                contract: &contract(scalar, matrix, 1),
                schemas: &schemas,
                inputs: &[scalar_layout],
                output: layout(
                    &schemas,
                    matrix,
                    ResidentValueKind::F64,
                    ResidentShape {
                        rows: 1,
                        columns: 2,
                    },
                ),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));

        let nested = SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                .into_boxed_slice(),
        };
        let (schemas, scalar, matrix) = schemas_for(nested, 1, 1);
        assert!(
            bind_matrix_literal(&ResidentKernelBindRequest {
                contract: &contract(scalar, matrix, 1),
                schemas: &schemas,
                inputs: &[layout(
                    &schemas,
                    scalar,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                )],
                output: layout(
                    &schemas,
                    matrix,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            })
            .is_ok()
        );
    }

    #[test]
    fn zero_by_zero_literal_requires_empty_input_and_output() {
        let (schemas, scalar, matrix) = f64_schemas(0, 0);
        let contract = contract(scalar, matrix, 0);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::F64,
                ResidentShape {
                    rows: 0,
                    columns: 0,
                },
            ),
        })
        .unwrap();
        let mut output = [];
        assert!(
            kernel
                .execute(&Inputs(Vec::new()), ResidentValueMut::F64(&mut output))
                .unwrap()
        );
    }

    #[test]
    fn snapshot_elements_are_cloned_and_missing_values_are_rejected() {
        let mut builder = SchemaTableBuilder::new();
        let element = builder
            .insert(schema(SchemaBody::Tuple(
                vec![SchemaBody::FloatingPoint(FloatWidth::W64)].into_boxed_slice(),
            )))
            .unwrap();
        let matrix = builder
            .insert(schema(SchemaBody::Matrix {
                element: Box::new(SchemaBody::Tuple(
                    vec![SchemaBody::FloatingPoint(FloatWidth::W64)].into_boxed_slice(),
                )),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                    .into_boxed_slice(),
            }))
            .unwrap();
        let build = builder.finish().unwrap();
        let element = build.resolve(element).unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema: element,
            shape_values: Box::new([]),
            data: ValueDataDraft::Tuple(
                vec![ValueDataDraft::F64(F64Bits::from_f64(1.0))].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let contract = contract(element, matrix, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                element,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let source = [Some(value.clone())];
        let mut output = [None];
        assert!(
            kernel
                .execute(
                    &Inputs(vec![ResidentValueRef::Snapshot(&source)]),
                    ResidentValueMut::Snapshot(&mut output),
                )
                .unwrap()
        );
        let materialized = output[0].as_ref().unwrap();
        assert_eq!(materialized.schema(), matrix);
        assert!(matches!(
            materialized.data(),
            mech_core::ValueData::Matrix(elements)
                if elements.elements().len() == 1
        ));

        let missing = [None];
        assert_eq!(
            kernel.execute(
                &Inputs(vec![ResidentValueRef::Snapshot(&missing)]),
                ResidentValueMut::Snapshot(&mut output),
            ),
            Err(ResidentKernelError::InvalidInput)
        );
    }

    #[test]
    fn recursive_snapshot_literal_admits_every_live_node_population() {
        const TUPLE_ELEMENTS: usize = 22_000;
        let tuple_body = SchemaBody::Tuple(
            std::iter::repeat_n(SchemaBody::Bool, TUPLE_ELEMENTS)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let (schemas, element, matrix) = schemas_for(tuple_body, 1, 1);
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(element, matrix, 1),
            schemas: &schemas,
            inputs: &[layout(
                &schemas,
                element,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            )],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let tuple = || {
            ValueDataDraft::Tuple(
                std::iter::repeat_n(ValueDataDraft::Bool(true), TUPLE_ELEMENTS)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };
        let source = [Some(
            ValueDraft {
                schema: element,
                shape_values: Box::new([]),
                data: tuple(),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap(),
        )];
        let previous = ValueDraft {
            schema: matrix,
            shape_values: Box::new([]),
            data: ValueDataDraft::Matrix(vec![tuple()].into_boxed_slice()),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let mut output = [Some(previous.clone())];

        assert_eq!(
            kernel.execute(
                &Inputs(vec![ResidentValueRef::Snapshot(&source)]),
                ResidentValueMut::Snapshot(&mut output),
            ),
            Err(ResidentKernelError::InvalidShape),
        );
        assert!(
            output[0]
                .as_ref()
                .unwrap()
                .language_eq(&schemas, &previous, &schemas)
                .unwrap()
        );
    }

    #[test]
    fn live_u64_elements_build_one_canonical_snapshot_matrix() {
        let (schemas, scalar, matrix) = schemas_for(
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
            1,
            2,
        );
        let kernel = bind_matrix_literal(&ResidentKernelBindRequest {
            contract: &contract(scalar, matrix, 2),
            schemas: &schemas,
            inputs: &[
                layout(
                    &schemas,
                    scalar,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
                layout(
                    &schemas,
                    scalar,
                    ResidentValueKind::Snapshot,
                    ResidentShape::SCALAR,
                ),
            ],
            output: layout(
                &schemas,
                matrix,
                ResidentValueKind::Snapshot,
                ResidentShape::SCALAR,
            ),
        })
        .unwrap();
        let scalar_value = |value| {
            ValueDraft {
                schema: scalar,
                shape_values: Box::new([]),
                data: ValueDataDraft::U64(value),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap()
        };
        let first = [Some(scalar_value(2))];
        let second = [Some(scalar_value(3))];
        let mut output = [None];
        kernel
            .execute(
                &Inputs(vec![
                    ResidentValueRef::Snapshot(&first),
                    ResidentValueRef::Snapshot(&second),
                ]),
                ResidentValueMut::Snapshot(&mut output),
            )
            .unwrap();
        let value = output[0].as_ref().unwrap();
        assert_eq!(value.schema(), matrix);
        let mech_core::ValueData::Matrix(values) = value.data() else {
            panic!("literal output must be a matrix")
        };
        assert!(matches!(
            values.elements().to_values().as_slice(),
            [mech_core::ValueData::U64(2), mech_core::ValueData::U64(3)]
        ));
    }
}

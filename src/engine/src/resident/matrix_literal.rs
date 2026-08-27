use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    DimensionExpr, ExternalInteraction, FloatWidth, FunctionCatalogBuilder, MResult,
    OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError,
    ResidentKernelInputs, ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeRule,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MatrixLiteralPlan {
    rows: usize,
    columns: usize,
    kind: ResidentValueKind,
}

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(["matrix"], "literal", bind_matrix_literal)?;
    Ok(())
}

fn resident_element_kind(element: &SchemaBody) -> Option<ResidentValueKind> {
    match element {
        SchemaBody::Bool => Some(ResidentValueKind::Bool),
        SchemaBody::Index => Some(ResidentValueKind::Index),
        SchemaBody::FloatingPoint(FloatWidth::W64) => Some(ResidentValueKind::F64),
        SchemaBody::String => Some(ResidentValueKind::String),
        SchemaBody::Atom(_)
        | SchemaBody::Enum { .. }
        | SchemaBody::Option(_)
        | SchemaBody::Tuple(_)
        | SchemaBody::Record(_)
        | SchemaBody::Table { .. }
        | SchemaBody::Set { .. }
        | SchemaBody::Map { .. }
        | SchemaBody::ReifiedType => Some(ResidentValueKind::Snapshot),
        SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(FloatWidth::W32)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::Id
        | SchemaBody::Matrix { .. } => None,
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
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let rows = usize::try_from(*rows).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let columns =
        usize::try_from(*columns).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let count = rows
        .checked_mul(columns)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let rows_u32 = u32::try_from(rows).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let columns_u32 =
        u32::try_from(columns).map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let kind = resident_element_kind(element).ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    if request.inputs.len() != count
        || request.output.kind != kind
        || request.output.shape
            != (ResidentShape {
                rows: rows_u32,
                columns: columns_u32,
            })
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
    Ok(
        BoundResidentKernel::new(matrix_literal, Box::new([])).with_retained_state(Arc::new(
            MatrixLiteralPlan {
                rows,
                columns,
                kind,
            },
        )),
    )
}

fn target_index(source: usize, rows: usize, columns: usize) -> usize {
    let row = source / columns;
    let column = source % columns;
    column * rows + row
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
    if inputs.len() != count || output.kind() != plan.kind || output.len() != count {
        return Err(ResidentKernelError::InvalidShape);
    }
    if count == 0 {
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
                target[target_index(source, plan.rows, plan.columns)] = *value;
            }
        }
        ResidentValueMut::Index(target) if plan.kind == ResidentValueKind::Index => {
            for source in 0..count {
                let Some(ResidentValueRef::Index([value])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                target[target_index(source, plan.rows, plan.columns)] = *value;
            }
        }
        ResidentValueMut::F64(target) if plan.kind == ResidentValueKind::F64 => {
            for source in 0..count {
                let Some(ResidentValueRef::F64([value])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                target[target_index(source, plan.rows, plan.columns)] = *value;
            }
        }
        ResidentValueMut::String(target) if plan.kind == ResidentValueKind::String => {
            for source in 0..count {
                let Some(ResidentValueRef::String([value])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                target[target_index(source, plan.rows, plan.columns)] = value.clone();
            }
        }
        ResidentValueMut::Snapshot(target) if plan.kind == ResidentValueKind::Snapshot => {
            for source in 0..count {
                let Some(ResidentValueRef::Snapshot([Some(value)])) = inputs.get(source) else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                target[target_index(source, plan.rows, plan.columns)] = Some(value.clone());
            }
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
    fn binder_rejects_wrong_counts_schemas_shapes_and_nested_matrices() {
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
        assert!(matches!(
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
                    ResidentShape {
                        rows: 1,
                        columns: 1,
                    },
                ),
            }),
            Err(ResidentKernelBindError::UnsupportedLayout)
        ));
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
                ResidentShape {
                    rows: 1,
                    columns: 1,
                },
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
        assert_eq!(output[0].as_ref().unwrap().schema(), value.schema());

        let missing = [None];
        assert_eq!(
            kernel.execute(
                &Inputs(vec![ResidentValueRef::Snapshot(&missing)]),
                ResidentValueMut::Snapshot(&mut output),
            ),
            Err(ResidentKernelError::InvalidInput)
        );
    }
}

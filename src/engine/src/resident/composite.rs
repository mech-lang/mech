use mech_core::snapshot::{F64Bits, MatrixValue, rebuild_composite_snapshot};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    DimensionExpr, ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeInstance, ShapeRule, ValueData,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct CompositeChildPlan {
    matrix_dimensions: Option<Box<[DimensionExpr]>>,
    shape: ResidentShape,
}

#[derive(Clone, Debug)]
struct CompositePackPlan {
    children: Box<[CompositeChildPlan]>,
}

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

fn composite_pack_plan(request: &ResidentKernelBindRequest<'_>) -> Option<CompositePackPlan> {
    let output = request.schemas.get(request.output.schema_id)?;
    let plans = |expected: &[SchemaBody]| {
        request.inputs[1..]
            .iter()
            .zip(expected)
            .map(|(input, expected)| {
                let matrix_dimensions = match expected {
                    SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => {
                        Some(dimensions.clone())
                    }
                    SchemaBody::Matrix { .. } => return None,
                    _ => None,
                };
                Some(CompositeChildPlan {
                    matrix_dimensions,
                    shape: input.shape,
                })
            })
            .collect::<Option<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    };
    let children = match output.body() {
        SchemaBody::Tuple(elements) if elements.len() == request.inputs.len() - 1 => {
            plans(elements)?
        }
        SchemaBody::Record(fields) if fields.len() == request.inputs.len() - 1 => {
            let expected = fields
                .iter()
                .map(|field| field.schema.clone())
                .collect::<Vec<_>>();
            plans(&expected)?
        }
        _ => return None,
    };
    Some(CompositePackPlan { children })
}

fn composite_child_layout_supported(
    request: &ResidentKernelBindRequest<'_>,
    input: &mech_core::ResidentPortLayout,
) -> bool {
    let Some(schema) = request.schemas.get(input.schema_id) else {
        return false;
    };
    let supported_kind = matches!(
        input.kind,
        ResidentValueKind::Bool
            | ResidentValueKind::Index
            | ResidentValueKind::F64
            | ResidentValueKind::String
            | ResidentValueKind::Snapshot
    );
    supported_kind
        && if matches!(schema.body(), SchemaBody::Matrix { .. }) {
            input.shape.len().is_some()
        } else {
            input.shape == ResidentShape::SCALAR
        }
}

fn bind_composite_pack(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if matches!(
        request
            .schemas
            .get(request.output.schema_id)
            .map(|schema| schema.body()),
        Some(SchemaBody::Matrix { .. })
    ) && request.inputs.len() == 1
        && request.inputs[0].schema_id == request.output.schema_id
        && request.inputs[0].kind == request.output.kind
        && request.inputs[0].shape == request.output.shape
        && request.output.shape.len() == Some(0)
    {
        if contract.interaction != ExternalInteraction::Pure
            || contract.inputs.len() != 1
            || contract.inputs[0].schema != request.inputs[0].schema_id
            || contract.inputs[0].access != AccessMode::Read
            || contract.inputs[0].delivery != DeliveryMode::Signal
            || contract.outputs.len() != 1
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
        return Ok(BoundResidentKernel::new(
            retain_empty_matrix_composite,
            Box::new([]),
        ));
    }
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
        || request.inputs[1..]
            .iter()
            .any(|input| !composite_child_layout_supported(request, input))
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
    let plan = composite_pack_plan(request).ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(BoundResidentKernel::new(composite_pack, Box::new([])).with_retained_state(Arc::new(plan)))
}

fn retain_empty_matrix_composite(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    if inputs.len() != 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let input_is_empty = match inputs.get(0) {
        Some(ResidentValueRef::Bool(values)) => values.is_empty(),
        Some(ResidentValueRef::Index(values)) => values.is_empty(),
        Some(ResidentValueRef::F64(values)) => values.is_empty(),
        Some(ResidentValueRef::String(values)) => values.is_empty(),
        Some(ResidentValueRef::Snapshot(values)) => values.is_empty(),
        None => false,
    };
    let output_is_empty = match output {
        ResidentValueMut::Bool(values) => values.is_empty(),
        ResidentValueMut::Index(values) => values.is_empty(),
        ResidentValueMut::F64(values) => values.is_empty(),
        ResidentValueMut::String(values) => values.is_empty(),
        ResidentValueMut::Snapshot(values) => values.is_empty(),
    };
    if !input_is_empty || !output_is_empty {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(true)
}

fn composite_matrix_shapes_match_template(shape: &ShapeInstance, plan: &CompositePackPlan) -> bool {
    plan.children.iter().all(|child| {
        let Some(dimensions) = &child.matrix_dimensions else {
            return true;
        };
        let Ok(rows) = shape.resolve_dimension(&dimensions[0]) else {
            return false;
        };
        let Ok(columns) = shape.resolve_dimension(&dimensions[1]) else {
            return false;
        };
        u32::try_from(rows).ok() == Some(child.shape.rows)
            && u32::try_from(columns).ok() == Some(child.shape.columns)
    })
}

fn canonical_matrix_elements<T, U>(
    values: &[T],
    shape: ResidentShape,
    mut convert: impl FnMut(&T) -> Option<U>,
) -> Option<Box<[U]>> {
    // Resident matrices are physically column-major, while detached snapshots
    // are canonical row-major values. Composite host payloads cross that
    // boundary here for every supported element representation.
    let rows = shape.rows as usize;
    let columns = shape.columns as usize;
    if values.len() != rows.checked_mul(columns)? {
        return None;
    }
    let mut canonical = Vec::with_capacity(values.len());
    for row in 0..rows {
        for column in 0..columns {
            canonical.push(convert(&values[column * rows + row])?);
        }
    }
    Some(canonical.into_boxed_slice())
}

fn composite_child_data(
    input: ResidentValueRef<'_>,
    plan: &CompositeChildPlan,
) -> Option<ValueData> {
    if plan.matrix_dimensions.is_some() {
        let matrix = match input {
            ResidentValueRef::Bool(values) => MatrixValue::from_bool_elements(
                canonical_matrix_elements(values, plan.shape, |value| match value {
                    0 => Some(false),
                    1 => Some(true),
                    _ => None,
                })?,
            ),
            ResidentValueRef::Index(values) => MatrixValue::from_index_elements(
                canonical_matrix_elements(values, plan.shape, |value| Some(*value))?,
            ),
            ResidentValueRef::F64(values) => MatrixValue::from_f64_elements(
                canonical_matrix_elements(values, plan.shape, |value| {
                    Some(F64Bits::from_f64(*value))
                })?,
            ),
            ResidentValueRef::String(values) => MatrixValue::from_string_elements(
                canonical_matrix_elements(values, plan.shape, |value| {
                    Some(value.clone().into_boxed_str())
                })?,
            ),
            ResidentValueRef::Snapshot(values) => MatrixValue::from_value_elements(
                canonical_matrix_elements(values, plan.shape, |value| {
                    value.as_ref().map(|value| value.data().clone())
                })?,
            ),
        };
        return Some(ValueData::Matrix(matrix));
    }
    match input {
        ResidentValueRef::Bool([value]) if *value <= 1 => Some(ValueData::Bool(*value != 0)),
        ResidentValueRef::Index([value]) => Some(ValueData::Index(*value)),
        ResidentValueRef::F64([value]) => Some(ValueData::F64(F64Bits::from_f64(*value))),
        ResidentValueRef::String([value]) => {
            Some(ValueData::String(value.clone().into_boxed_str()))
        }
        ResidentValueRef::Snapshot([Some(value)]) => Some(value.data().clone()),
        _ => None,
    }
}

fn composite_pack(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::Snapshot([Some(template)])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let plan = kernel
        .retained_state::<CompositePackPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if plan.children.len() != inputs.len() - 1
        || !composite_matrix_shapes_match_template(template.shape(), plan)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let children = (1..inputs.len())
        .map(|index| composite_child_data(inputs.get(index)?, plan.children.get(index - 1)?))
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
        layout_with_shape(schemas, schema_id, kind, ResidentShape::SCALAR)
    }

    fn layout_with_shape(
        schemas: &mech_core::SchemaTable,
        schema_id: mech_core::SchemaId,
        kind: ResidentValueKind,
        shape: ResidentShape,
    ) -> mech_core::ResidentPortLayout {
        mech_core::ResidentPortLayout {
            schema_id,
            schema_key: schemas.entry(schema_id).unwrap().key(),
            kind,
            shape,
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

    #[test]
    fn matrix_valued_composite_children_convert_physical_columns_to_canonical_rows() {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let matrix = builder
            .insert(schema(SchemaBody::Matrix {
                element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                dimensions: vec![
                    mech_core::DimensionExpr::Constant(2),
                    mech_core::DimensionExpr::Constant(3),
                ]
                .into_boxed_slice(),
            }))
            .unwrap();
        let tuple = builder
            .insert(schema(SchemaBody::Tuple(
                vec![SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                    dimensions: vec![
                        mech_core::DimensionExpr::Constant(2),
                        mech_core::DimensionExpr::Constant(3),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            )))
            .unwrap();
        let build = builder.finish().unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let (schemas, _) = build.into_parts();
        let contract =
            ResolvedOperationContract::LegacyOpaque(mech_core::LegacyOpaqueOperationContract {
                input_schemas: Box::new([]),
                output_schemas: Box::new([]),
            });
        let matrix_layout = layout_with_shape(
            &schemas,
            matrix,
            ResidentValueKind::F64,
            ResidentShape {
                rows: 2,
                columns: 3,
            },
        );
        assert!(composite_child_layout_supported(
            &ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &[layout(&schemas, tuple, ResidentValueKind::Snapshot)],
                output: layout(&schemas, tuple, ResidentValueKind::Snapshot),
            },
            &matrix_layout,
        ));

        let values = [1.0, 4.0, 2.0, 5.0, 3.0, 6.0];
        let plan = CompositeChildPlan {
            matrix_dimensions: Some(
                vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into_boxed_slice(),
            ),
            shape: ResidentShape {
                rows: 2,
                columns: 3,
            },
        };
        let Some(ValueData::Matrix(matrix)) =
            composite_child_data(ResidentValueRef::F64(&values), &plan)
        else {
            panic!("matrix child must remain a matrix snapshot payload")
        };
        let mech_core::snapshot::SequenceView::F64(values) = matrix.elements() else {
            panic!("matrix child changed element representation")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn parameterized_matrix_child_shape_must_match_the_template_instance() {
        let parameter = mech_core::DimensionParameterId::new(0);
        let schema = mech_core::SchemaDraft {
            dimension_parameters: vec![mech_core::DimensionParameterDeclaration {
                id: parameter,
                origin: mech_core::DimensionParameterOrigin::Explicit,
                lifetime: mech_core::DimensionLifetime::Activation,
                lower_bound: mech_core::DimensionExpr::Constant(1),
                upper_bound: None,
            }]
            .into_boxed_slice(),
            body: SchemaBody::Tuple(
                vec![SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)),
                    dimensions: vec![
                        mech_core::DimensionExpr::Constant(1),
                        mech_core::DimensionExpr::Parameter(parameter),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        }
        .finalize()
        .unwrap();
        let template_shape = schema
            .instantiate_shape(vec![3].into_boxed_slice())
            .unwrap();
        let dimensions = vec![
            mech_core::DimensionExpr::Constant(1),
            mech_core::DimensionExpr::Parameter(parameter),
        ]
        .into_boxed_slice();

        let mismatched = CompositePackPlan {
            children: vec![CompositeChildPlan {
                matrix_dimensions: Some(dimensions.clone()),
                shape: ResidentShape {
                    rows: 1,
                    columns: 2,
                },
            }]
            .into_boxed_slice(),
        };
        assert!(!composite_matrix_shapes_match_template(
            &template_shape,
            &mismatched
        ));

        let matching = CompositePackPlan {
            children: vec![CompositeChildPlan {
                matrix_dimensions: Some(dimensions),
                shape: ResidentShape {
                    rows: 1,
                    columns: 3,
                },
            }]
            .into_boxed_slice(),
        };
        assert!(composite_matrix_shapes_match_template(
            &template_shape,
            &matching
        ));
    }
}

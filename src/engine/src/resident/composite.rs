use mech_core::snapshot::{
    F64Bits, MatrixValue, rebuild_composite_snapshot, wrap_resident_dynamic_data,
};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, CardinalitySpec, ChangeDetectionPolicy,
    DeliveryMode, DimensionExpr, ExternalInteraction, FunctionCatalogBuilder, MResult,
    OutputConstruction, ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError,
    ResidentKernelInputs, ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, ShapeInstance, ShapeRule, ValueData,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct CompositeChildPlan {
    matrix_dimensions: Option<Box<[DimensionExpr]>>,
    input_is_matrix: bool,
    shape: ResidentShape,
    dynamic: Option<DynamicChildPlan>,
}

#[derive(Clone, Debug)]
struct DynamicChildPlan {
    schema_id: mech_core::SchemaId,
    schema_key: mech_core::SchemaKey,
    shape: ShapeInstance,
    schemas: Arc<mech_core::SchemaTable>,
    body: SchemaBody,
}

#[derive(Clone, Debug)]
struct CompositePackPlan {
    children: Box<[CompositeChildPlan]>,
    table: Option<CompositeTablePlan>,
}

#[derive(Clone, Debug)]
struct CompositeTablePlan {
    rows: CardinalitySpec,
    row_count: usize,
}

fn cardinality_accepts(
    cardinality: &CardinalitySpec,
    row_count: usize,
    shape: &ShapeInstance,
) -> bool {
    let Ok(row_count) = u64::try_from(row_count) else {
        return false;
    };
    match cardinality {
        CardinalitySpec::Exact(expected) => {
            shape.resolve_dimension(expected).ok() == Some(row_count)
        }
        CardinalitySpec::Dynamic { upper_bound } => upper_bound.as_ref().is_none_or(|maximum| {
            shape
                .resolve_dimension(maximum)
                .is_ok_and(|value| row_count <= value)
        }),
    }
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
    if matches!(expected, SchemaBody::Dynamic) {
        return true;
    }
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
    composite_child_schemas(output.body(), children.len()).is_some_and(|(expected, _)| {
        children
            .iter()
            .zip(expected.iter())
            .all(|(input, expected)| port_matches_schema_body(request, input, expected))
    })
}

fn composite_child_schemas(
    output: &SchemaBody,
    child_count: usize,
) -> Option<(Vec<SchemaBody>, Option<CompositeTablePlan>)> {
    match output {
        SchemaBody::Tuple(elements) if elements.len() == child_count => {
            Some((elements.to_vec(), None))
        }
        SchemaBody::Record(fields) if fields.len() == child_count => Some((
            fields.iter().map(|field| field.schema.clone()).collect(),
            None,
        )),
        SchemaBody::Table { columns, rows } => {
            if columns.is_empty() {
                let CardinalitySpec::Exact(DimensionExpr::Constant(row_count)) = rows else {
                    return None;
                };
                let row_count = usize::try_from(*row_count).ok()?;
                return (child_count == 0).then(|| {
                    (
                        Vec::new(),
                        Some(CompositeTablePlan {
                            rows: rows.clone(),
                            row_count,
                        }),
                    )
                });
            }
            if child_count % columns.len() != 0 {
                return None;
            }
            let row_count = child_count / columns.len();
            let statically_accepted = match rows {
                CardinalitySpec::Exact(DimensionExpr::Constant(expected)) => {
                    usize::try_from(*expected).ok() == Some(row_count)
                }
                CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(maximum)),
                } => usize::try_from(*maximum).is_ok_and(|maximum| row_count <= maximum),
                _ => true,
            };
            if !statically_accepted {
                return None;
            }
            let expected = columns
                .iter()
                .flat_map(|column| std::iter::repeat_n(column.schema.clone(), row_count))
                .collect();
            Some((
                expected,
                Some(CompositeTablePlan {
                    rows: rows.clone(),
                    row_count,
                }),
            ))
        }
        _ => None,
    }
}

fn composite_pack_plan(request: &ResidentKernelBindRequest<'_>) -> Option<CompositePackPlan> {
    let output = request.schemas.get(request.output.schema_id)?;
    let plans = |expected: &[SchemaBody]| {
        request.inputs[1..]
            .iter()
            .zip(expected)
            .map(|(input, expected)| {
                let input_schema = request.schemas.get(input.schema_id)?;
                let matrix_dimensions = match expected {
                    SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => {
                        Some(dimensions.clone())
                    }
                    SchemaBody::Matrix { .. } => return None,
                    _ => None,
                };
                Some(CompositeChildPlan {
                    matrix_dimensions,
                    input_is_matrix: matches!(input_schema.body(), SchemaBody::Matrix { .. }),
                    shape: input.shape,
                    dynamic: matches!(expected, SchemaBody::Dynamic).then(|| DynamicChildPlan {
                        schema_id: input.schema_id,
                        schema_key: input.schema_key,
                        shape: input.shape_instance.clone(),
                        schemas: Arc::new(request.schemas.clone()),
                        body: input_schema.body().clone(),
                    }),
                })
            })
            .collect::<Option<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    };
    let (expected, table) = composite_child_schemas(output.body(), request.inputs.len() - 1)?;
    let children = plans(&expected)?;
    Some(CompositePackPlan { children, table })
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
            Some(SchemaBody::Tuple { .. } | SchemaBody::Record { .. } | SchemaBody::Table { .. })
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

fn composite_shapes_match_template(shape: &ShapeInstance, plan: &CompositePackPlan) -> bool {
    let matrices_match = plan.children.iter().all(|child| {
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
    });
    matrices_match
        && plan
            .table
            .as_ref()
            .is_none_or(|table| cardinality_accepts(&table.rows, table.row_count, shape))
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
    let data = if plan.input_is_matrix {
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
        ValueData::Matrix(matrix)
    } else {
        match input {
            ResidentValueRef::Bool([value]) if *value <= 1 => Some(ValueData::Bool(*value != 0)),
            ResidentValueRef::Index([value]) => Some(ValueData::Index(*value)),
            ResidentValueRef::F64([value]) => Some(ValueData::F64(F64Bits::from_f64(*value))),
            ResidentValueRef::String([value]) => {
                Some(ValueData::String(value.clone().into_boxed_str()))
            }
            ResidentValueRef::Snapshot([Some(value)]) => Some(value.data().clone()),
            _ => None,
        }?
    };
    Some(match &plan.dynamic {
        Some(dynamic) => wrap_resident_dynamic_data(
            dynamic.schema_id,
            dynamic.schema_key,
            dynamic.shape.clone(),
            Arc::clone(&dynamic.schemas),
            &dynamic.body,
            data,
        ),
        None => data,
    })
}

fn checked_cost_usize(value: u64) -> Result<usize, ResidentKernelError> {
    usize::try_from(value).map_err(|_| ResidentKernelError::InvalidShape)
}

fn resident_child_clone_cost(
    meter: &mut super::budget::ResidentBudgetMeter,
    input: ResidentValueRef<'_>,
    plan: &CompositeChildPlan,
) -> Result<(usize, usize), ResidentKernelError> {
    let expected_len = if plan.input_is_matrix {
        plan.shape.len().ok_or(ResidentKernelError::InvalidShape)?
    } else {
        1
    };
    if input.len() != expected_len {
        return Err(ResidentKernelError::InvalidInput);
    }
    let container = expected_len
        .checked_mul(core::mem::size_of::<ValueData>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let (payload, nodes) = match input {
        ResidentValueRef::Bool(values) => {
            if values.iter().any(|value| *value > 1) {
                return Err(ResidentKernelError::InvalidInput);
            }
            (values.len(), values.len())
        }
        ResidentValueRef::Index(values) => (
            values
                .len()
                .checked_mul(core::mem::size_of::<u64>())
                .ok_or(ResidentKernelError::InvalidShape)?,
            values.len(),
        ),
        ResidentValueRef::F64(values) => (
            values
                .len()
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or(ResidentKernelError::InvalidShape)?,
            values.len(),
        ),
        ResidentValueRef::String(values) => {
            let payload = values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(value.len())
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
            (payload, values.len())
        }
        ResidentValueRef::Snapshot(values) => {
            let mut retained = 0usize;
            let mut nodes = 0usize;
            for value in values {
                let value = value.as_ref().ok_or(ResidentKernelError::InvalidInput)?;
                let schemas = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
                let footprint =
                    super::budget::measure_canonical_value_footprint(meter, value, &schemas)?;
                retained = retained
                    .checked_add(checked_cost_usize(footprint.retained_bytes)?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                nodes = nodes
                    .checked_add(checked_cost_usize(footprint.node_count)?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            (retained, nodes)
        }
    };
    let dynamic_overhead = usize::from(plan.dynamic.is_some())
        .checked_mul(core::mem::size_of::<ValueData>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    Ok((
        container
            .checked_add(payload)
            .and_then(|bytes| bytes.checked_add(dynamic_overhead))
            .ok_or(ResidentKernelError::InvalidShape)?,
        nodes
            .checked_add(1 + usize::from(plan.dynamic.is_some()))
            .ok_or(ResidentKernelError::InvalidShape)?,
    ))
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
        || !composite_shapes_match_template(template.shape(), plan)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let template_schemas = template
        .schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let template_footprint = super::budget::measure_canonical_value_footprint(
        &mut footprint_meter,
        template,
        &template_schemas,
    )?;
    let mut retained_bytes = checked_cost_usize(template_footprint.retained_bytes)?;
    let mut retained_nodes = checked_cost_usize(template_footprint.node_count)?;
    for (index, child) in plan.children.iter().enumerate() {
        let (bytes, nodes) = resident_child_clone_cost(
            &mut footprint_meter,
            inputs
                .get(index + 1)
                .ok_or(ResidentKernelError::InvalidInput)?,
            child,
        )?;
        retained_bytes = retained_bytes
            .checked_add(bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        retained_nodes = retained_nodes
            .checked_add(nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    let child_containers = plan
        .children
        .len()
        .checked_mul(core::mem::size_of::<ValueData>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let admitted_children = super::budget::PreparedKernel::new(
        plan.children.len(),
        super::budget::resident_cost! {
            compute_work: plan
                .children
                .len()
                .checked_mul(2)
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: plan.children.len(),
            output_bytes: retained_bytes,
            temporary_bytes: retained_bytes
                .checked_mul(3)
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes: retained_bytes,
            container_bytes: child_containers,
            retained_nodes,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let children = (1..=admitted_children)
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
            shape_instance: schemas
                .get(schema_id)
                .unwrap()
                .instantiate_shape(Box::new([]))
                .unwrap(),
            resolved_selector: None,
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
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: Box::new([]),
            outputs: Box::new([]),
            interaction: mech_core::ExternalInteraction::Pure,
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
                output: output.clone(),
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
    fn table_composite_children_follow_column_major_schema_order() {
        let body = SchemaBody::Table {
            columns: vec![
                mech_core::SchemaField {
                    name: "id".to_owned(),
                    schema: SchemaBody::String,
                },
                mech_core::SchemaField {
                    name: "x".to_owned(),
                    schema: SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                },
            ]
            .into_boxed_slice(),
            rows: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        };
        let (children, table) = composite_child_schemas(&body, 4).unwrap();
        assert_eq!(
            children,
            vec![
                SchemaBody::String,
                SchemaBody::String,
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
            ]
        );
        assert_eq!(table.unwrap().row_count, 2);
        assert!(composite_child_schemas(&body, 3).is_none());
        assert!(composite_child_schemas(&body, 6).is_none());
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
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: Box::new([]),
            outputs: Box::new([]),
            interaction: mech_core::ExternalInteraction::Pure,
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
            input_is_matrix: true,
            shape: ResidentShape {
                rows: 2,
                columns: 3,
            },
            dynamic: None,
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
                input_is_matrix: true,
                shape: ResidentShape {
                    rows: 1,
                    columns: 2,
                },
                dynamic: None,
            }]
            .into_boxed_slice(),
            table: None,
        };
        assert!(!composite_shapes_match_template(
            &template_shape,
            &mismatched
        ));

        let matching = CompositePackPlan {
            children: vec![CompositeChildPlan {
                matrix_dimensions: Some(dimensions),
                input_is_matrix: true,
                shape: ResidentShape {
                    rows: 1,
                    columns: 3,
                },
                dynamic: None,
            }]
            .into_boxed_slice(),
            table: None,
        };
        assert!(composite_shapes_match_template(&template_shape, &matching));
    }
}

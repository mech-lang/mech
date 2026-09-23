use mech_core::snapshot::{EnumDraft, F64Bits, MatrixValue};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, CardinalitySpec, ChangeDetectionPolicy,
    DeliveryMode, DimensionExpr, ExternalInteraction, FunctionCatalogBuilder,
    ImplementationMemoryClass, MResult, OutputConstruction, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs, ResidentShape,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, ResolvedOperationContract, SchemaBody,
    ShapeInstance, ShapeRule, ValueData, ValueDataDraft,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct CompositeChildPlan {
    matrix_dimensions: Option<Box<[DimensionExpr]>>,
    input_is_matrix: bool,
    snapshot_backed_matrix: bool,
    shape: ResidentShape,
    dynamic: bool,
    source: mech_core::ResidentPortLayout,
}

fn resolved_matrix_shape(body: &SchemaBody, shape: &ShapeInstance) -> Option<ResidentShape> {
    let SchemaBody::Matrix { dimensions, .. } = body else {
        return None;
    };
    let [rows, columns] = dimensions.as_ref() else {
        return None;
    };
    Some(ResidentShape {
        rows: u32::try_from(shape.resolve_dimension(rows).ok()?).ok()?,
        columns: u32::try_from(shape.resolve_dimension(columns).ok()?).ok()?,
    })
}

#[derive(Clone, Debug)]
struct CompositePackPlan {
    children: Box<[CompositeChildPlan]>,
    table: Option<CompositeTablePlan>,
    output: mech_core::ResidentPortLayout,
    schemas: Arc<mech_core::SchemaTable>,
    // Only the shape-independent allocation containers are read from this
    // activation binding. Current values use a newly witnessed constructor.
    constructor: mech_core::snapshot::CompositeSnapshotConstructor,
    normalization: mech_core::snapshot::CompositeBindingCost,
}

#[derive(Clone, Debug)]
struct CompositeTablePlan {
    rows: CardinalitySpec,
    row_count: usize,
}

#[derive(Clone, Debug)]
struct EnumPackPlan {
    payload: CompositeChildPlan,
    accepted_ordinals: Box<[(u32, bool)]>,
    output: mech_core::ResidentPortLayout,
    schemas: Arc<mech_core::SchemaTable>,
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
    builder.insert_resident_factory(
        ["core"],
        "composite-pack",
        ImplementationMemoryClass::CanonicalFinalize,
        bind_composite_pack,
    )?;
    builder.insert_resident_factory(
        ["core"],
        "enum-pack",
        ImplementationMemoryClass::CanonicalFinalize,
        bind_enum_pack,
    )?;
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
        .and_then(|schema| schema.closed_body(&input.shape_instance).ok())
        .is_some_and(|schema| &schema == expected)
}

fn composite_children_match_output_schema(request: &ResidentKernelBindRequest<'_>) -> bool {
    let Some(output) = request.schemas.get(request.output.schema_id) else {
        return false;
    };
    let Ok(output) = output.closed_body(&request.output.shape_instance) else {
        return false;
    };
    let children = request.inputs;
    composite_child_schemas(&output, children.len()).is_some_and(|(expected, _)| {
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
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } if child_count % 2 == 0 => {
            let count = child_count / 2;
            let accepted = match cardinality {
                CardinalitySpec::Exact(DimensionExpr::Constant(n)) => *n == count as u64,
                CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(n)),
                } => count as u64 <= *n,
                _ => true,
            };
            accepted.then(|| {
                (
                    (0..count)
                        .flat_map(|_| [key.as_ref().clone(), value.as_ref().clone()])
                        .collect(),
                    None,
                )
            })
        }
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
        request
            .inputs
            .iter()
            .zip(expected)
            .map(|(input, expected)| {
                let input_schema = request.schemas.get(input.schema_id)?;
                let input_is_matrix = matches!(input_schema.body(), SchemaBody::Matrix { .. });
                let logical_matrix_shape = input_is_matrix
                    .then(|| resolved_matrix_shape(input_schema.body(), &input.shape_instance))
                    .flatten();
                if input_is_matrix && logical_matrix_shape.is_none() {
                    return None;
                }
                let matrix_dimensions = match expected {
                    SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => {
                        Some(dimensions.clone())
                    }
                    SchemaBody::Matrix { .. } => return None,
                    _ => None,
                };
                Some(CompositeChildPlan {
                    source: input.clone(),
                    matrix_dimensions,
                    input_is_matrix,
                    snapshot_backed_matrix: input_is_matrix
                        && input.kind == ResidentValueKind::Snapshot,
                    shape: logical_matrix_shape.unwrap_or(input.shape),
                    dynamic: matches!(expected, SchemaBody::Dynamic),
                })
            })
            .collect::<Option<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    };
    let (expected, table) = composite_child_schemas(output.body(), request.inputs.len())?;
    let children = plans(&expected)?;
    let constructor = mech_core::snapshot::CompositeSnapshotConstructor::bind(
        request.output.schema_id,
        request.output.shape_instance.clone(),
        &request
            .inputs
            .iter()
            .map(|input| (input.schema_id, input.shape_instance.clone()))
            .collect::<Vec<_>>(),
        Arc::new(request.schemas.clone()),
    )
    .ok()?;
    let normalization = mech_core::snapshot::CompositeSnapshotConstructor::binding_cost(
        request.output.schema_id,
        &request
            .inputs
            .iter()
            .map(|input| input.schema_id)
            .collect::<Vec<_>>(),
        request.schemas,
    )?;
    Some(CompositePackPlan {
        normalization,
        children,
        table,
        constructor,
        output: request.output.clone(),
        schemas: Arc::new(request.schemas.clone()),
    })
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
            let Some(logical_shape) = resolved_matrix_shape(schema.body(), &input.shape_instance)
            else {
                return false;
            };
            if input.kind == ResidentValueKind::Snapshot {
                input.shape == ResidentShape::SCALAR
            } else {
                input.shape == logical_shape
            }
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
    let empty_matrix = matches!(
        request
            .schemas
            .get(request.output.schema_id)
            .map(|schema| schema.body()),
        Some(SchemaBody::Matrix { .. })
    ) && request.inputs.is_empty()
        && request.output.shape.len() == Some(0);
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != request.inputs.len()
        || contract.outputs.len() != 1
        || (!empty_matrix
            && !matches!(
                request
                    .schemas
                    .get(request.output.schema_id)
                    .map(|schema| schema.body()),
                Some(
                    SchemaBody::Tuple { .. }
                        | SchemaBody::Record { .. }
                        | SchemaBody::Table { .. }
                        | SchemaBody::Map { .. }
                )
            ))
        || contract
            .inputs
            .iter()
            .any(|input| input.access != AccessMode::Read || input.delivery != DeliveryMode::Signal)
        || contract
            .inputs
            .iter()
            .zip(request.inputs.iter())
            .any(|(contract, input)| contract.schema != input.schema_id)
        || (!empty_matrix && !composite_children_match_output_schema(request))
        || request
            .inputs
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
        || output.change_detection != ChangeDetectionPolicy::KernelReported
        || (!empty_matrix
            && (request.output.kind != ResidentValueKind::Snapshot
                || request.output.shape != ResidentShape::SCALAR))
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    if empty_matrix {
        return Ok(BoundResidentKernel::new(
            construct_empty_matrix,
            Box::new([]),
        ));
    }
    let plan = composite_pack_plan(request).ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(BoundResidentKernel::new(composite_pack, Box::new([])).with_retained_state(Arc::new(plan)))
}

fn bind_enum_pack(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let [ordinal, payload] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Some(ordinal_schema) = request.schemas.get(ordinal.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Some(payload_schema) = request.schemas.get(payload.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Some(output_schema) = request.schemas.get(request.output.schema_id) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let Ok(output_body) = output_schema.closed_body(&request.output.shape_instance) else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let SchemaBody::Enum { variants, .. } = output_body else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let accepted_ordinals = variants
        .iter()
        .enumerate()
        .filter_map(|(index, variant)| {
            let expected = variant.payload.as_ref()?;
            port_matches_schema_body(request, payload, expected).then_some((
                u32::try_from(index).ok()?,
                matches!(expected, SchemaBody::Dynamic)
                    && !matches!(payload_schema.body(), SchemaBody::Dynamic),
            ))
        })
        .collect::<Vec<_>>();
    let input_is_matrix = matches!(payload_schema.body(), SchemaBody::Matrix { .. });
    let logical_matrix_shape = input_is_matrix
        .then(|| resolved_matrix_shape(payload_schema.body(), &payload.shape_instance))
        .flatten();
    if accepted_ordinals.is_empty()
        || (input_is_matrix && logical_matrix_shape.is_none())
        || contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, input)| {
                port.schema != input.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
            })
        || ordinal_schema.body() != &SchemaBody::Index
        || ordinal.kind != ResidentValueKind::Index
        || ordinal.shape != ResidentShape::SCALAR
        || !composite_child_layout_supported(request, payload)
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
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
        || output.change_detection != ChangeDetectionPolicy::KernelReported
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let plan = EnumPackPlan {
        payload: CompositeChildPlan {
            matrix_dimensions: None,
            input_is_matrix,
            snapshot_backed_matrix: input_is_matrix && payload.kind == ResidentValueKind::Snapshot,
            shape: logical_matrix_shape.unwrap_or(payload.shape),
            dynamic: false,
            source: payload.clone(),
        },
        accepted_ordinals: accepted_ordinals.into_boxed_slice(),
        output: request.output.clone(),
        schemas: Arc::new(request.schemas.clone()),
    };
    Ok(BoundResidentKernel::new(enum_pack, Box::new([])).with_retained_state(Arc::new(plan)))
}

fn enum_payload_draft(
    input: ResidentValueRef<'_>,
    plan: &EnumPackPlan,
    dynamic: bool,
) -> Result<ValueDataDraft, ResidentKernelError> {
    let (data, shape_values) = if let ResidentValueRef::Snapshot([Some(value)]) = input {
        if value.schema_key() != plan.payload.source.schema_key {
            return Err(ResidentKernelError::InvalidInput);
        }
        let schema = plan
            .schemas
            .get(plan.payload.source.schema_id)
            .ok_or(ResidentKernelError::InvalidInput)?;
        if !mech_core::shape_change_allowed(
            schema,
            &plan.payload.source.shape_instance,
            value.shape(),
        ) {
            return Err(ResidentKernelError::InvalidShape);
        }
        let context =
            mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(&plan.schemas);
        let rebound = value
            .rebind_with_context(plan.payload.source.schema_id, value.shape(), &context)
            .map_err(|_| ResidentKernelError::InvalidInput)?;
        (
            rebound
                .canonical_data_draft()
                .map_err(|_| ResidentKernelError::InvalidInput)?,
            rebound
                .shape()
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
        )
    } else {
        let data =
            composite_child_data(input, &plan.payload).ok_or(ResidentKernelError::InvalidInput)?;
        let schema = plan
            .schemas
            .get(plan.payload.source.schema_id)
            .ok_or(ResidentKernelError::InvalidInput)?;
        (
            mech_core::snapshot::canonical_snapshot_data_draft(schema.body(), &data)
                .map_err(|_| ResidentKernelError::InvalidInput)?,
            plan.payload
                .source
                .shape_instance
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
        )
    };
    Ok(if dynamic {
        ValueDataDraft::Dynamic(Some(Box::new(mech_core::ValueDraft {
            schema: plan.payload.source.schema_id,
            shape_values,
            data,
        })))
    } else {
        data
    })
}

fn enum_pack(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let plan = kernel
        .retained_state::<EnumPackPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if inputs.len() != 2 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let Some(ResidentValueRef::Index([ordinal])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ordinal = ordinal
        .checked_sub(1)
        .and_then(|ordinal| u32::try_from(ordinal).ok())
        .ok_or(ResidentKernelError::InvalidInput)?;
    let dynamic = plan
        .accepted_ordinals
        .iter()
        .find_map(|(accepted, dynamic)| (*accepted == ordinal).then_some(*dynamic))
        .ok_or(ResidentKernelError::InvalidInput)?;
    let payload_input = inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?;
    let mut meter = super::budget::ResidentBudgetMeter::default();
    let mut payload_plan = plan.payload.clone();
    payload_plan.dynamic = dynamic;
    let (payload_bytes, payload_nodes, payload_encoded_bytes) =
        resident_child_clone_cost(&mut meter, payload_input, &payload_plan)?;
    let finalization_work = match payload_input {
        ResidentValueRef::Snapshot([Some(value)]) => {
            let schema = plan
                .schemas
                .get(plan.payload.source.schema_id)
                .ok_or(ResidentKernelError::InvalidInput)?;
            super::budget::preflight_canonical_data_finalization(
                &mut meter,
                schema.body(),
                value.data(),
            )?
        }
        ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
        _ => 0,
    };
    let root_bytes = mech_core::snapshot::CompositeSnapshotConstructor::value_container_bytes(
        plan.output.shape_instance.parameter_values().len(),
    )
    .and_then(|bytes| bytes.checked_add(core::mem::size_of::<ValueData>()))
    .ok_or(ResidentKernelError::InvalidShape)?;
    let output_bytes = root_bytes
        .checked_add(payload_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_nodes = super::budget::checked_u64(payload_nodes)?
        .checked_add(2)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_footprint = mech_core::snapshot::ValueFootprint {
        encoded_bytes: payload_encoded_bytes
            .checked_add(5)
            .ok_or(ResidentKernelError::InvalidShape)?,
        retained_bytes: super::budget::checked_u64(output_bytes)?,
        node_count: output_nodes,
    };
    if let Some(previous) = target.as_ref() {
        let previous_footprint =
            super::budget::published_canonical_footprint(&mut meter, previous, &plan.schemas)?;
        let equality_work = super::budget::projected_language_equality_work(
            &plan.schemas,
            previous,
            previous_footprint,
            plan.output.schema_id,
            plan.output.shape_instance.parameter_values().len(),
            output_footprint,
        )?;
        meter.charge_comparison_work(equality_work)?;
    }
    meter.charge_compute_work(output_nodes)?;
    let measured = meter.estimate();
    let draft_bytes = core::mem::size_of::<ValueDataDraft>()
        .checked_add(payload_bytes)
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::budget::PreparedKernel::new(
        (),
        super::budget::resident_cost! {
            comparison_work: measured.comparison_work(),
            compute_work: measured.compute_work(),
            output_elements: 1,
            output_bytes,
            temporary_bytes: output_bytes
                .checked_add(draft_bytes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes: payload_bytes,
            container_bytes: root_bytes
                .checked_add(core::mem::size_of::<ValueDataDraft>())
                .ok_or(ResidentKernelError::InvalidShape)?,
            retained_nodes: measured.retained_nodes()
                .checked_add(output_nodes)
                .ok_or(ResidentKernelError::InvalidShape)?,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let payload = enum_payload_draft(payload_input, plan, dynamic)?;
    let canonicalization_budget =
        mech_core::snapshot::SnapshotCanonicalizationBudget::new(finalization_work);
    let context =
        mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(&plan.schemas)
            .with_canonicalization_budget(&canonicalization_budget);
    let next = mech_core::ValueDraft {
        schema: plan.output.schema_id,
        shape_values: plan
            .output
            .shape_instance
            .parameter_values()
            .to_vec()
            .into_boxed_slice(),
        data: ValueDataDraft::Enum(EnumDraft {
            ordinal,
            payload: Some(Box::new(payload)),
        }),
    }
    .finalize(&context)
    .map_err(|error| match error {
        mech_core::snapshot::SnapshotValueError::CanonicalizationWorkLimitExceededV1 { .. } => {
            ResidentKernelError::InvalidShape
        }
        _ => ResidentKernelError::InvalidInput,
    })?;
    let changed = match target.as_ref() {
        Some(previous) => !previous
            .snapshot_eq(&plan.schemas, &next, &plan.schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?,
        None => true,
    };
    *target = Some(next);
    Ok(changed)
}

fn construct_empty_matrix(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let empty = match output {
        ResidentValueMut::Bool(values) => values.is_empty(),
        ResidentValueMut::Index(values) => values.is_empty(),
        ResidentValueMut::F64(values) => values.is_empty(),
        ResidentValueMut::String(values) => values.is_empty(),
        ResidentValueMut::Snapshot(values) => values.is_empty(),
    };
    if inputs.len() != 0 || !empty {
        return Err(ResidentKernelError::InvalidShape);
    }
    Ok(true)
}

fn composite_shapes_match_declared_output(shape: &ShapeInstance, plan: &CompositePackPlan) -> bool {
    let matrices_match = plan.children.iter().all(|child| {
        // Canonical Snapshot children carry current logical geometry. The
        // shared constructor checks their complete current closed schemas.
        if child.snapshot_backed_matrix {
            return true;
        }
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

pub(super) fn canonical_matrix_elements<T, U>(
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
    let data = if plan.snapshot_backed_matrix {
        let ResidentValueRef::Snapshot([Some(value)]) = input else {
            return None;
        };
        let ValueData::Matrix(matrix) = value.data() else {
            return None;
        };
        ValueData::Matrix(matrix.clone())
    } else if plan.input_is_matrix {
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
            ResidentValueRef::Snapshot(_) => return None,
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
    Some(data)
}

fn checked_cost_usize(value: u64) -> Result<usize, ResidentKernelError> {
    usize::try_from(value).map_err(|_| ResidentKernelError::InvalidShape)
}

fn resident_child_clone_cost(
    meter: &mut super::budget::ResidentBudgetMeter,
    input: ResidentValueRef<'_>,
    plan: &CompositeChildPlan,
) -> Result<(usize, usize, u64), ResidentKernelError> {
    let expected_len = if plan.input_is_matrix {
        if plan.snapshot_backed_matrix {
            1
        } else {
            plan.shape.len().ok_or(ResidentKernelError::InvalidShape)?
        }
    } else {
        1
    };
    if input.len() != expected_len {
        return Err(ResidentKernelError::InvalidInput);
    }
    let container = expected_len
        .checked_mul(core::mem::size_of::<ValueData>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let (payload, nodes, mut encoded_bytes) = match input {
        ResidentValueRef::Bool(values) => {
            if values.iter().any(|value| *value > 1) {
                return Err(ResidentKernelError::InvalidInput);
            }
            (
                values.len(),
                values.len(),
                super::budget::checked_u64(values.len())?,
            )
        }
        ResidentValueRef::Index(values) => (
            values
                .len()
                .checked_mul(core::mem::size_of::<u64>())
                .ok_or(ResidentKernelError::InvalidShape)?,
            values.len(),
            super::budget::checked_u64(values.len())?
                .checked_mul(8)
                .ok_or(ResidentKernelError::InvalidShape)?,
        ),
        ResidentValueRef::F64(values) => (
            values
                .len()
                .checked_mul(core::mem::size_of::<f64>())
                .ok_or(ResidentKernelError::InvalidShape)?,
            values.len(),
            super::budget::checked_u64(values.len())?
                .checked_mul(8)
                .ok_or(ResidentKernelError::InvalidShape)?,
        ),
        ResidentValueRef::String(values) => {
            let payload = values.iter().try_fold(0usize, |bytes, value| {
                bytes
                    .checked_add(value.len())
                    .ok_or(ResidentKernelError::InvalidShape)
            })?;
            let encoded = super::budget::checked_u64(values.len())?
                .checked_mul(8)
                .and_then(|headers| headers.checked_add(payload as u64))
                .ok_or(ResidentKernelError::InvalidShape)?;
            (payload, values.len(), encoded)
        }
        ResidentValueRef::Snapshot(values) => {
            let mut retained = 0usize;
            let mut nodes = 0usize;
            let mut encoded = 0u64;
            for value in values {
                let value = value.as_ref().ok_or(ResidentKernelError::InvalidInput)?;
                let schemas = value.schemas().ok_or(ResidentKernelError::InvalidInput)?;
                let footprint =
                    super::budget::measure_canonical_value_footprint(meter, value, &schemas)?;
                encoded = encoded
                    .checked_add(footprint.encoded_bytes)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                retained = retained
                    .checked_add(checked_cost_usize(footprint.retained_bytes)?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                nodes = nodes
                    .checked_add(checked_cost_usize(footprint.node_count)?)
                    .ok_or(ResidentKernelError::InvalidShape)?;
            }
            (retained, nodes, encoded)
        }
    };
    let dynamic_overhead = if plan.dynamic {
        mech_core::snapshot::CompositeSnapshotConstructor::value_container_bytes(
            plan.source.shape_instance.parameter_values().len(),
        )
        .and_then(|bytes| bytes.checked_add(core::mem::size_of::<ValueData>()))
        .and_then(|bytes| bytes.checked_add(1 + 32 + 8 + 5 + 8))
        .and_then(|bytes| {
            bytes.checked_add(
                plan.source
                    .shape_instance
                    .parameter_values()
                    .len()
                    .checked_mul(8)?,
            )
        })
        .and_then(|bytes| bytes.checked_add(payload))
        .ok_or(ResidentKernelError::InvalidShape)?
    } else {
        0
    };
    if plan.dynamic {
        // Dynamic canonical values carry a presence tag, schema identity and
        // shape/value envelope. Retained ValueData containers are not encoded.
        encoded_bytes = encoded_bytes
            .checked_add(1 + 32 + 8 + 5 + 8)
            .and_then(|bytes| {
                bytes.checked_add(
                    (plan.source.shape_instance.parameter_values().len() as u64).checked_mul(8)?,
                )
            })
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    Ok((
        container
            .checked_add(payload)
            .and_then(|bytes| bytes.checked_add(dynamic_overhead))
            .ok_or(ResidentKernelError::InvalidShape)?,
        nodes
            .checked_add(1 + usize::from(plan.dynamic))
            .ok_or(ResidentKernelError::InvalidShape)?,
        encoded_bytes,
    ))
}

fn current_constructor(
    plan: &CompositePackPlan,
    inputs: &dyn ResidentKernelInputs,
) -> Result<mech_core::snapshot::CompositeSnapshotConstructor, ResidentKernelError> {
    let children = plan
        .children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            let shape = match inputs.get(index).ok_or(ResidentKernelError::InvalidInput)? {
                ResidentValueRef::Snapshot([Some(value)]) => {
                    if value.schema_key() != child.source.schema_key {
                        return Err(ResidentKernelError::InvalidInput);
                    }
                    let schema = plan
                        .schemas
                        .get(child.source.schema_id)
                        .ok_or(ResidentKernelError::InvalidInput)?;
                    if !mech_core::shape_change_allowed(
                        schema,
                        &child.source.shape_instance,
                        value.shape(),
                    ) {
                        return Err(ResidentKernelError::InvalidShape);
                    }
                    value.shape()
                }
                ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
                _ => &child.source.shape_instance,
            };
            Ok((child.source.schema_id, shape.clone()))
        })
        .collect::<Result<Vec<_>, ResidentKernelError>>()?;
    let shape = mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
        plan.output.schema_id,
        &children,
        &plan.schemas,
    )
    .map_err(|_| ResidentKernelError::InvalidShape)?;
    let output_schema = plan
        .schemas
        .get(plan.output.schema_id)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    if !mech_core::shape_change_allowed(output_schema, &plan.output.shape_instance, &shape)
        || !composite_shapes_match_declared_output(&shape, plan)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    mech_core::snapshot::CompositeSnapshotConstructor::bind(
        plan.output.schema_id,
        shape,
        &children,
        Arc::clone(&plan.schemas),
    )
    .map_err(|_| ResidentKernelError::InvalidShape)
}

fn composite_pack(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let plan = kernel
        .retained_state::<CompositePackPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if plan.children.len() != inputs.len() {
        return Err(ResidentKernelError::InvalidShape);
    }
    let normalization_bytes = checked_cost_usize(plan.normalization.temporary_bytes)?;
    let normalization_nodes = plan.normalization.metadata_nodes;
    let normalization_work = plan.normalization.compute_work;
    let mut footprint_meter = super::budget::ResidentBudgetMeter::default();
    let mut child_bytes = 0usize;
    // Eight bytes conservatively cover the aggregate map length prefix.
    let mut output_encoded_bytes = 8u64;
    let mut native_draft_bytes = 0usize;
    let mut native_value_bytes = 0usize;
    let mut key_bytes = 0usize;
    let mut key_work = 0u64;
    let map = matches!(
        plan.schemas
            .get(plan.output.schema_id)
            .map(|schema| schema.body()),
        Some(SchemaBody::Map { .. })
    );
    let mut staged_child_nodes = 0usize;
    for (index, child) in plan.children.iter().enumerate() {
        let (bytes, nodes, encoded_bytes) = resident_child_clone_cost(
            &mut footprint_meter,
            inputs.get(index).ok_or(ResidentKernelError::InvalidInput)?,
            child,
        )?;
        output_encoded_bytes = output_encoded_bytes
            .checked_add(encoded_bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        child_bytes = child_bytes
            .checked_add(bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        let input = inputs.get(index).ok_or(ResidentKernelError::InvalidInput)?;
        if !matches!(input, ResidentValueRef::Snapshot(_)) {
            native_draft_bytes = native_draft_bytes
                .checked_add(
                    input
                        .len()
                        .checked_mul(core::mem::size_of::<mech_core::ValueDataDraft>())
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )
                .and_then(|total| total.checked_add(bytes))
                .ok_or(ResidentKernelError::InvalidShape)?;
            native_value_bytes = native_value_bytes
                .checked_add(bytes)
                .and_then(|total| {
                    total.checked_add(
                        mech_core::snapshot::CompositeSnapshotConstructor::value_container_bytes(
                            child.source.shape_instance.parameter_values().len(),
                        )?,
                    )
                })
                .ok_or(ResidentKernelError::InvalidShape)?;
        }
        if map && index % 2 == 0 {
            key_bytes = key_bytes
                .checked_add(bytes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            // Encoded bytes and node counts conservatively bound key comparison
            // and recursive normalization work, without materializing a draft.
            key_work = key_work
                .checked_add(super::budget::checked_u64(bytes.max(nodes))?)
                .ok_or(ResidentKernelError::InvalidShape)?;
        }
        staged_child_nodes = staged_child_nodes
            .checked_add(nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
    }
    let previous_footprint = target
        .as_ref()
        .map(|previous| {
            super::budget::published_canonical_footprint(
                &mut footprint_meter,
                previous,
                &plan.schemas,
            )
        })
        .transpose()?;
    let (output_containers, table_scratch) = plan
        .constructor
        .allocation_containers()
        .ok_or(ResidentKernelError::InvalidShape)?;
    let child_containers = plan
        .children
        .len()
        .checked_mul(core::mem::size_of::<mech_core::Value>() + core::mem::size_of::<ValueData>())
        .ok_or(ResidentKernelError::InvalidShape)?;
    let output_bytes = child_bytes
        .checked_add(output_containers)
        .ok_or(ResidentKernelError::InvalidShape)?;
    // These phases can coexist: typed native children, cloned canonical output,
    // native drafts, and key normalization / table packing scratch. Payload
    // construction retains schema labels; current schema closure temporarily
    // clones them, covered separately by normalization_bytes.
    let temporary_bytes = [
        native_value_bytes,
        output_bytes,
        native_draft_bytes,
        key_bytes,
        table_scratch,
        child_containers,
        normalization_bytes,
    ]
    .into_iter()
    .try_fold(0usize, |total, bytes| total.checked_add(bytes))
    .ok_or(ResidentKernelError::InvalidShape)?;
    let container_bytes = [
        output_containers,
        child_containers,
        native_draft_bytes,
        key_bytes,
        table_scratch,
        normalization_bytes,
    ]
    .into_iter()
    .try_fold(0usize, |total, bytes| total.checked_add(bytes))
    .ok_or(ResidentKernelError::InvalidShape)?;
    // Canonical insertion checks the previous key then scans prior keys; each
    // pair comparison visits both keys, and insertion can shift every prior
    // entry. Also reserve a full key walk for recursive normalization.
    let count = super::budget::checked_u64(plan.children.len() / 2)?;
    let finalization_work = if map {
        key_work
            .checked_mul(
                count
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(1))
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )
            .and_then(|work| {
                count
                    .checked_mul(count.saturating_sub(1))
                    .and_then(|shifts| work.checked_add(shifts / 2))
            })
            .ok_or(ResidentKernelError::InvalidShape)?
    } else {
        0
    };
    footprint_meter.charge_comparison_work(finalization_work)?;
    let final_output_nodes = super::budget::checked_u64(staged_child_nodes)?
        .checked_add(2)
        .ok_or(ResidentKernelError::InvalidShape)?;
    if let (Some(previous), Some(previous_footprint)) = (target.as_ref(), previous_footprint) {
        // Equality visits canonical payloads, not native allocation containers.
        // Keep retained bytes for allocation admission and use the separately
        // witnessed payload bound here; schema and shape work are additional.
        let equality_work = super::budget::projected_language_equality_work(
            &plan.schemas,
            previous,
            previous_footprint,
            plan.output.schema_id,
            plan.output.shape_instance.parameter_values().len(),
            mech_core::snapshot::ValueFootprint {
                encoded_bytes: output_encoded_bytes,
                retained_bytes: super::budget::checked_u64(output_bytes)?,
                node_count: final_output_nodes,
            },
        )?;
        footprint_meter.charge_comparison_work(equality_work)?;
    }
    footprint_meter.charge_compute_work(normalization_work)?;
    let measured = footprint_meter.estimate();
    let cloned_bytes = [
        child_bytes,
        native_value_bytes,
        key_bytes,
        normalization_bytes,
    ]
    .into_iter()
    .try_fold(0usize, |total, bytes| total.checked_add(bytes))
    .ok_or(ResidentKernelError::InvalidShape)?;
    let admitted_children = super::budget::PreparedKernel::new(
        plan.children.len(),
        super::budget::resident_cost! {
            comparison_work: measured.comparison_work(),
            compute_work: measured.compute_work()
                .checked_add(
                    super::budget::checked_u64(plan.children.len())?
                        .checked_mul(2)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )
                .ok_or(ResidentKernelError::InvalidShape)?,
            output_elements: plan.children.len(),
            output_bytes: output_bytes,
            temporary_bytes: temporary_bytes,
            cloned_bytes: cloned_bytes,
            container_bytes: container_bytes,
            retained_nodes: measured.retained_nodes()
                .checked_add(final_output_nodes)
                .and_then(|nodes| nodes.checked_add(normalization_nodes))
                .ok_or(ResidentKernelError::InvalidShape)?,
            ..super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let constructor = current_constructor(plan, inputs)?;
    let context =
        mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(&plan.schemas);
    let children = (0..admitted_children)
        .map(|index| {
            let input = inputs.get(index).ok_or(ResidentKernelError::InvalidInput)?;
            if let ResidentValueRef::Snapshot([Some(value)]) = input {
                return Ok(value.clone());
            }
            let child = &plan.children[index];
            let data =
                composite_child_data(input, child).ok_or(ResidentKernelError::InvalidInput)?;
            let schema = plan
                .schemas
                .get(child.source.schema_id)
                .ok_or(ResidentKernelError::InvalidInput)?;
            let data = mech_core::snapshot::canonical_snapshot_data_draft(schema.body(), &data)
                .map_err(|_| ResidentKernelError::InvalidInput)?;
            mech_core::ValueDraft {
                schema: child.source.schema_id,
                shape_values: child
                    .source
                    .shape_instance
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
                data,
            }
            .finalize(&context)
            .map_err(|_| ResidentKernelError::InvalidInput)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let budget = mech_core::snapshot::SnapshotCanonicalizationBudget::new(finalization_work);
    let next = constructor
        .construct(children, Some(&budget))
        .map_err(|error| match error {
            mech_core::snapshot::SnapshotValueError::CanonicalizationWorkLimitExceededV1 {
                ..
            } => ResidentKernelError::InvalidShape,
            _ => ResidentKernelError::InvalidInput,
        })?;
    let changed = match target.as_ref() {
        Some(previous) => !previous
            .snapshot_eq(&plan.schemas, &next, &plan.schemas)
            .map_err(|_| ResidentKernelError::InvalidOutput)?,
        None => true,
    };
    *target = Some(next);
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::ValueDataDraft;

    struct Inputs<'a>(&'a [ResidentValueRef<'a>]);

    impl ResidentKernelInputs for Inputs<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0.get(index).copied()
        }
    }

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
            activation_fixed_shape: true,
            resolved_selector: None,
        }
    }

    #[test]
    fn enum_payload_rebinds_foreign_dynamic_schema_ids() {
        let mut foreign_builder = mech_core::SchemaTableBuilder::new();
        let foreign_index = foreign_builder.insert(schema(SchemaBody::Index)).unwrap();
        let foreign_dynamic = foreign_builder.insert(schema(SchemaBody::Dynamic)).unwrap();
        let foreign_build = foreign_builder.finish().unwrap();
        let foreign_index = foreign_build.resolve(foreign_index).unwrap();
        let foreign_dynamic = foreign_build.resolve(foreign_dynamic).unwrap();
        let foreign = mech_core::ValueDraft {
            schema: foreign_dynamic,
            shape_values: Box::new([]),
            data: ValueDataDraft::Dynamic(Some(Box::new(mech_core::ValueDraft {
                schema: foreign_index,
                shape_values: Box::new([]),
                data: ValueDataDraft::Index(7),
            }))),
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
            &foreign_build.table,
        ))
        .unwrap();

        let mut local_builder = mech_core::SchemaTableBuilder::new();
        for body in [
            SchemaBody::Bool,
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
            SchemaBody::String,
            SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8),
        ] {
            local_builder.insert(schema(body)).unwrap();
        }
        let local_index = local_builder.insert(schema(SchemaBody::Index)).unwrap();
        let local_dynamic = local_builder.insert(schema(SchemaBody::Dynamic)).unwrap();
        let local_build = local_builder.finish().unwrap();
        let local_index = local_build.resolve(local_index).unwrap();
        let local_dynamic = local_build.resolve(local_dynamic).unwrap();
        assert_ne!(foreign_index, local_index);
        let local = Arc::new(local_build.table);
        let source = layout(&local, local_dynamic, ResidentValueKind::Snapshot);
        let plan = EnumPackPlan {
            payload: CompositeChildPlan {
                matrix_dimensions: None,
                input_is_matrix: false,
                snapshot_backed_matrix: false,
                shape: ResidentShape::SCALAR,
                dynamic: false,
                source: source.clone(),
            },
            accepted_ordinals: Box::new([]),
            output: source,
            schemas: local,
        };
        let input = [Some(foreign)];
        let payload = enum_payload_draft(ResidentValueRef::Snapshot(&input), &plan, false).unwrap();
        let ValueDataDraft::Dynamic(Some(inner)) = payload else {
            panic!("enum payload retains its Dynamic envelope")
        };
        assert_eq!(inner.schema, local_index);
    }

    #[test]
    fn composite_change_reports_preserve_exact_snapshot_representation() {
        let f64_body = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
        let field = mech_core::SchemaField {
            name: "a".to_owned(),
            schema: f64_body.clone(),
        };
        for body in [
            SchemaBody::Tuple(vec![f64_body.clone()].into_boxed_slice()),
            SchemaBody::Record(vec![field.clone()].into_boxed_slice()),
            SchemaBody::Map {
                key: Box::new(f64_body.clone()),
                value: Box::new(f64_body.clone()),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
            },
            SchemaBody::Table {
                columns: vec![field].into_boxed_slice(),
                rows: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
            },
        ] {
            let count = if matches!(&body, SchemaBody::Map { .. }) {
                2
            } else {
                1
            };
            let mut builder = mech_core::SchemaTableBuilder::new();
            let scalar = builder.insert(schema(f64_body.clone())).unwrap();
            let output = builder.insert(schema(body)).unwrap();
            let built = builder.finish().unwrap();
            let scalar = built.resolve(scalar).unwrap();
            let output = built.resolve(output).unwrap();
            let (schemas, _) = built.into_parts();
            let contract =
                ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
                    inputs: vec![
                        mech_core::ResolvedInputPort {
                            schema: scalar,
                            access: AccessMode::Read,
                            delivery: DeliveryMode::Signal
                        };
                        count
                    ]
                    .into_boxed_slice(),
                    outputs: vec![mech_core::ResolvedOutputPort {
                        schema: output,
                        access: AccessMode::Write,
                        delivery: DeliveryMode::Signal,
                        construction: OutputConstruction::FullWrite {
                            shape: ShapeRule::Declared,
                        },
                        alias: AliasPolicy::NoAlias,
                        change_detection: ChangeDetectionPolicy::KernelReported,
                    }]
                    .into_boxed_slice(),
                    interaction: ExternalInteraction::Pure,
                });
            let input_layouts = vec![layout(&schemas, scalar, ResidentValueKind::F64); count];
            let kernel = bind_composite_pack(&ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &input_layouts,
                output: layout(&schemas, output, ResidentValueKind::Snapshot),
            })
            .unwrap();
            let mut output = [None];
            for (value, expected) in [
                (3.0, true),
                (3.0, false),
                (7.0, true),
                (7.0, false),
                (0.0, true),
                (-0.0, true),
                (f64::NAN, true),
                (f64::NAN, false),
            ] {
                let value = [value];
                let key = [1.0];
                let inputs = if count == 2 {
                    vec![ResidentValueRef::F64(&key), ResidentValueRef::F64(&value)]
                } else {
                    vec![ResidentValueRef::F64(&value)]
                };
                assert_eq!(
                    kernel
                        .execute(&Inputs(&inputs), ResidentValueMut::Snapshot(&mut output))
                        .unwrap(),
                    expected
                );
            }
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

        let good = [layout(&schemas, scalar, ResidentValueKind::F64)];
        assert!(composite_children_match_output_schema(
            &ResidentKernelBindRequest {
                contract: &contract,
                schemas: &schemas,
                inputs: &good,
                output: output.clone(),
            }
        ));

        let bad = [layout(&schemas, matrix, ResidentValueKind::F64)];
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
            snapshot_backed_matrix: false,
            shape: ResidentShape {
                rows: 2,
                columns: 3,
            },
            dynamic: false,
            source: matrix_layout,
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
    fn snapshot_backed_matrix_child_preserves_logical_shape_and_payload() {
        let matrix_body = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
            dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        };
        let mut builder = mech_core::SchemaTableBuilder::new();
        let matrix = builder.insert(schema(matrix_body.clone())).unwrap();
        let tuple = builder
            .insert(schema(SchemaBody::Tuple(
                vec![matrix_body].into_boxed_slice(),
            )))
            .unwrap();
        let build = builder.finish().unwrap();
        let matrix = build.resolve(matrix).unwrap();
        let tuple = build.resolve(tuple).unwrap();
        let (schemas, _) = build.into_parts();
        let contract = ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
            inputs: vec![mech_core::ResolvedInputPort {
                schema: matrix,
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
            outputs: vec![mech_core::ResolvedOutputPort {
                schema: tuple,
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        });
        let inputs = [layout(&schemas, matrix, ResidentValueKind::Snapshot)];
        let kernel = bind_composite_pack(&ResidentKernelBindRequest {
            contract: &contract,
            schemas: &schemas,
            inputs: &inputs,
            output: layout(&schemas, tuple, ResidentValueKind::Snapshot),
        })
        .unwrap();
        let matrix_draft = || {
            ValueDataDraft::Matrix(
                [1_u64, 2, 3, 4]
                    .into_iter()
                    .map(ValueDataDraft::U64)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };
        let child = [Some(
            mech_core::ValueDraft {
                schema: matrix,
                shape_values: Box::new([]),
                data: matrix_draft(),
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                &schemas,
            ))
            .unwrap(),
        )];
        let resident_inputs = [ResidentValueRef::Snapshot(&child)];
        let mut output = [None];
        kernel
            .execute(
                &Inputs(&resident_inputs),
                ResidentValueMut::Snapshot(&mut output),
            )
            .unwrap();
        let ValueData::Tuple(children) = output[0].as_ref().unwrap().data() else {
            panic!("composite output must remain a tuple")
        };
        let ValueData::Matrix(matrix) = &children[0] else {
            panic!("snapshot-backed matrix child was wrapped instead of retained")
        };
        assert!(matches!(
            matrix.elements().to_values().as_slice(),
            [
                ValueData::U64(1),
                ValueData::U64(2),
                ValueData::U64(3),
                ValueData::U64(4)
            ]
        ));
    }

    #[test]
    fn parameterized_matrix_child_shape_must_match_the_declared_output_instance() {
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
        let output_shape = schema
            .instantiate_shape(vec![3].into_boxed_slice())
            .unwrap();
        let mut builder = mech_core::SchemaTableBuilder::new();
        let matrix_body = match schema.body() {
            SchemaBody::Tuple(items) => items[0].clone(),
            _ => unreachable!(),
        };
        let input = builder
            .insert(
                mech_core::SchemaDraft {
                    body: matrix_body,
                    dimension_parameters: vec![mech_core::DimensionParameterDeclaration {
                        id: parameter,
                        origin: mech_core::DimensionParameterOrigin::Explicit,
                        lifetime: mech_core::DimensionLifetime::Activation,
                        lower_bound: DimensionExpr::Constant(1),
                        upper_bound: None,
                    }]
                    .into_boxed_slice(),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let output = builder.insert(schema.clone()).unwrap();
        let build = builder.finish().unwrap();
        let output = build.resolve(output).unwrap();
        let input = build.resolve(input).unwrap();
        let (schemas, _) = build.into_parts();
        let output = mech_core::ResidentPortLayout {
            schema_id: output,
            schema_key: schemas.entry(output).unwrap().key(),
            shape_instance: output_shape.clone(),
            shape: ResidentShape::SCALAR,
            kind: ResidentValueKind::Snapshot,
            activation_fixed_shape: true,
            resolved_selector: None,
        };
        let dimensions = vec![
            mech_core::DimensionExpr::Constant(1),
            mech_core::DimensionExpr::Parameter(parameter),
        ]
        .into_boxed_slice();

        let mismatched = CompositePackPlan {
            normalization: mech_core::snapshot::CompositeBindingCost::default(),
            children: vec![CompositeChildPlan {
                matrix_dimensions: Some(dimensions.clone()),
                input_is_matrix: true,
                snapshot_backed_matrix: false,
                shape: ResidentShape {
                    rows: 1,
                    columns: 2,
                },
                dynamic: false,
                source: output.clone(),
            }]
            .into_boxed_slice(),
            table: None,
            output: output.clone(),
            schemas: Arc::new(schemas.clone()),
            constructor: mech_core::snapshot::CompositeSnapshotConstructor::bind(
                output.schema_id,
                output.shape_instance.clone(),
                &[(input, output.shape_instance.clone())],
                Arc::new(schemas.clone()),
            )
            .unwrap(),
        };
        assert!(!composite_shapes_match_declared_output(
            &output_shape,
            &mismatched
        ));

        let matching = CompositePackPlan {
            normalization: mech_core::snapshot::CompositeBindingCost::default(),
            children: vec![CompositeChildPlan {
                matrix_dimensions: Some(dimensions),
                input_is_matrix: true,
                snapshot_backed_matrix: false,
                shape: ResidentShape {
                    rows: 1,
                    columns: 3,
                },
                dynamic: false,
                source: output.clone(),
            }]
            .into_boxed_slice(),
            table: None,
            output: output.clone(),
            schemas: Arc::new(schemas.clone()),
            constructor: mech_core::snapshot::CompositeSnapshotConstructor::bind(
                output.schema_id,
                output.shape_instance.clone(),
                &[(input, output.shape_instance.clone())],
                Arc::new(schemas.clone()),
            )
            .unwrap(),
        };
        assert!(composite_shapes_match_declared_output(
            &output_shape,
            &matching
        ));
    }
}

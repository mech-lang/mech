//! Promoted arithmetic retains occurrence-ordered destination reads and casts.
use super::*;
use mech_core::{
    ConversionPlan, ResolvedType, execute_conversion_draft, plan_explicit_cast,
    plan_implicit_conversion,
};

#[derive(Clone)]
struct Plan {
    mode: u8,
    arithmetic: SemanticArithmetic,
    rational_power: bool,
    rows: usize,
    columns: usize,
    source_dimensions: Option<(usize, usize)>,
    logical_selector: bool,
    source: SnapshotAccessSelectorLayout,
    target: SnapshotAccessSelectorLayout,
    selectors: Box<[SnapshotAccessSelectorLayout]>,
    max_writes: usize,
    promote: ConversionPlan,
    assign: ConversionPlan,
}

pub(super) fn bind(
    request: &ResidentKernelBindRequest<'_>,
    mode: u8,
    arithmetic: SemanticArithmetic,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let (count, region) = match mode {
        0 | 1 => (3, RegionPolicy::IndexedAxis { axis: 0 }),
        2 => (3, RegionPolicy::IndexedAxis { axis: 1 }),
        3 => (4, RegionPolicy::RectangularRegion),
        _ => return Err(ResidentKernelBindError::InvalidParameters),
    };
    validate_rmw(request, count, region)?;
    let [base, source, ..] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let schema = |port: &mech_core::ResidentPortLayout| {
        request
            .schemas
            .get(port.schema_id)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)
    };
    let base_schema = schema(base)?;
    let source_schema = schema(source)?;
    let SchemaBody::Matrix {
        element: destination,
        ..
    } = base_schema.body()
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if schema(&request.output)?.body() != base_schema.body() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let (incoming, source_dimensions) = match source_schema.body() {
        SchemaBody::Matrix { element, .. } => {
            let dimensions = if source.kind == ResidentValueKind::Snapshot {
                None
            } else {
                Some(declared_matrix_dimensions(request, source)?)
            };
            (element.as_ref(), dimensions)
        }
        scalar => (scalar, Some((1, 1))),
    };
    let rational_power = cfg!(feature = "r64")
        && arithmetic == SemanticArithmetic::Power
        && destination.as_ref() == &SchemaBody::Rational64
        && incoming == &SchemaBody::SignedInteger(mech_core::IntegerWidth::W32);
    if !rational_power && !snapshot_arithmetic_element_supported(arithmetic, incoming) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let target_type = ResolvedType::from_schema_body(destination, &[])
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let arithmetic_type = ResolvedType::from_schema_body(
        if rational_power {
            destination.as_ref()
        } else {
            incoming
        },
        &[],
    )
    .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let promote = plan_implicit_conversion(&target_type, &arithmetic_type)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let assign = plan_explicit_cast(&arithmetic_type, &target_type)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let (rows, columns) = declared_matrix_dimensions(request, base)?;
    if declared_matrix_dimensions(request, &request.output)? != (rows, columns) {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let layout = |port: &mech_core::ResidentPortLayout| SnapshotAccessSelectorLayout {
        schema: port.schema_id,
        shape: port.shape_instance.clone(),
        resident_shape: port.shape,
    };
    let mut axis_capacities = Vec::new();
    let mut logical_selector = false;
    for selector in &request.inputs[2..] {
        if !positional_selector_layout(request, selector) {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        let axis_capacity = declared_selector_cardinality(request, selector)?;
        logical_selector |= request
            .schemas
            .get(selector.schema_id)
            .is_some_and(|schema| is_logical_selector_schema(schema.body()));
        axis_capacities.push(axis_capacity);
    }
    let max_writes = match mode {
        0 => Some(axis_capacities[0]),
        1 => axis_capacities[0].checked_mul(columns),
        2 => axis_capacities[0].checked_mul(rows),
        _ => axis_capacities[0].checked_mul(axis_capacities[1]),
    }
    .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    Ok(BoundResidentKernel::new(execute, Box::new([]))
        .with_retained_state(Arc::new(Plan {
            mode,
            arithmetic,
            rational_power,
            rows,
            columns,
            source_dimensions,
            logical_selector,
            source: layout(source),
            target: layout(&request.output),
            selectors: request.inputs[2..].iter().map(layout).collect(),
            max_writes,
            promote,
            assign,
        }))
        .with_snapshot_output(snapshot_output_metadata(request))
        .with_snapshot_schemas(request.schemas.clone()))
}

fn execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let plan = kernel
        .retained_state::<Plan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if inputs.len() != plan.selectors.len() + 1 {
        return Err(ResidentKernelError::InvalidInput);
    }
    let count = plan
        .rows
        .checked_mul(plan.columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let current_ref = match &output {
        ResidentValueMut::F64(values) => ResidentValueRef::F64(values),
        ResidentValueMut::Snapshot(values) => ResidentValueRef::Snapshot(values),
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    let source_ref = input(inputs, 0)?;
    let source_schema = schemas
        .get(plan.source.schema)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let (source_rows, source_columns) = match source_schema.body() {
        body @ SchemaBody::Matrix { .. } => match source_ref {
            ResidentValueRef::Snapshot([Some(value)]) => {
                value
                    .validate_against(schemas)
                    .map_err(|_| ResidentKernelError::InvalidInput)?;
                snapshot_matrix_dimensions(value, body)?
            }
            ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
            _ => plan
                .source_dimensions
                .ok_or(ResidentKernelError::InvalidShape)?,
        },
        _ => (1, 1),
    };
    if let Some(expected) = plan.source_dimensions
        && expected != (source_rows, source_columns)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let source_len = source_rows
        .checked_mul(source_columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let mut meter = super::super::budget::ResidentBudgetMeter::default();
    let current_cost =
        snapshot_selector_materialization_cost(schemas, &plan.target, current_ref, &mut meter)?;
    let source_cost =
        snapshot_selector_materialization_cost(schemas, &plan.source, source_ref, &mut meter)?;
    if current_cost.elements != count || source_cost.elements != source_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let mut selector_cost = SelectorMaterializationCost::default();
    for (index, selector) in plan.selectors.iter().enumerate() {
        selector_cost = add_selector_cost(
            selector_cost,
            snapshot_selector_materialization_cost(
                schemas,
                selector,
                input(inputs, index + 1)?,
                &mut meter,
            )?,
        )?;
    }
    // Numeric snapshots have fixed scalar payloads. Admit the complete draft,
    // selector, conversion and publication storage before materializing it.
    let elements = count
        .checked_add(source_len)
        .and_then(|n| n.checked_add(selector_cost.elements))
        .and_then(|n| n.checked_add(plan.max_writes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let draft_bytes = elements
        .checked_mul(core::mem::size_of::<ValueDataDraft>())
        .and_then(|n| n.checked_mul(4))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let measured = meter.estimate();
    let cloned_bytes = current_cost
        .cloned_bytes
        .checked_add(source_cost.cloned_bytes)
        .and_then(|bytes| bytes.checked_add(selector_cost.cloned_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let materialized_bytes = current_cost
        .retained_bytes
        .checked_add(source_cost.retained_bytes)
        .and_then(|bytes| bytes.checked_add(selector_cost.retained_bytes))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let retained_nodes = current_cost
        .retained_nodes
        .checked_add(source_cost.retained_nodes)
        .and_then(|nodes| nodes.checked_add(selector_cost.retained_nodes))
        .and_then(|nodes| {
            nodes.checked_add(
                super::super::budget::checked_u64(
                    elements.checked_mul(4).and_then(|n| n.checked_add(8))?,
                )
                .ok()?,
            )
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::super::budget::PreparedKernel::new(
        (),
        super::super::budget::resident_cost! {
            compute_work: measured.compute_work()
                .checked_add(super::super::budget::checked_u64(
                    elements.checked_mul(if plan.rational_power { 128 } else { 8 })
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )?)
                .ok_or(ResidentKernelError::InvalidShape)?,
            comparison_work: measured.comparison_work()
                .checked_add(super::super::budget::checked_u64(count)?)
                .ok_or(ResidentKernelError::InvalidShape)?,
            temporary_bytes: materialized_bytes
                .checked_add(super::super::budget::checked_u64(draft_bytes)?)
                .ok_or(ResidentKernelError::InvalidShape)?,
            cloned_bytes,
            retained_nodes,
            output_elements: count,
            output_bytes: super::super::budget::checked_u64(
                count.checked_mul(core::mem::size_of::<ValueDataDraft>())
                    .ok_or(ResidentKernelError::InvalidShape)?,
            )?,
            selector_bytes: selector_cost.retained_bytes,
            ..super::super::budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let current = selector_value(schemas, &plan.target, current_ref)?;
    let source = selector_value(schemas, &plan.source, source_ref)?;
    let ValueDataDraft::Matrix(mut next) = current
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidOutput)?
    else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let source = snapshot_numeric_elements(&source)?;
    if next.len() != count || source.len() != source_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    let selectors = plan
        .selectors
        .iter()
        .enumerate()
        .map(|(i, layout)| selector_value(schemas, layout, input(inputs, i + 1)?))
        .collect::<Result<Vec<_>, _>>()?;
    let (positions, selected_rows, selected_columns) = if plan.mode == 0 {
        // Dense logical masks carry physical column-major positions. Read
        // those before selector_value normalizes matrix data to row-major.
        // Numeric selectors retain their authored occurrence order.
        let selector = input(inputs, 1)?;
        let physical = if matches!(selector, ResidentValueRef::Bool(_)) {
            let selected = ValidatedPositions::new(selector, count)?;
            let mut positions = Vec::with_capacity(selected.len());
            selected.try_for_each(|_, position| {
                positions.push(position);
                Ok(())
            })?;
            positions
        } else {
            access_indices(&selectors[0], count)?
        };
        let length = physical.len();
        (
            physical
                .into_iter()
                .map(|p| (p % plan.rows) * plan.columns + p / plan.rows)
                .collect::<Vec<_>>(),
            length,
            1,
        )
    } else {
        let rows = if plan.mode == 2 {
            (0..plan.rows).collect()
        } else {
            access_indices(&selectors[0], plan.rows)?
        };
        let columns = if plan.mode == 1 {
            (0..plan.columns).collect()
        } else {
            access_indices(&selectors[usize::from(plan.mode == 3)], plan.columns)?
        };
        (
            rows.iter()
                .flat_map(|r| columns.iter().map(move |c| r * plan.columns + c))
                .collect::<Vec<_>>(),
            rows.len(),
            columns.len(),
        )
    };
    let source_index = |ordinal: usize, destination: usize| -> Result<usize, ResidentKernelError> {
        if source.len() == 1 {
            return Ok(0);
        }
        if plan.logical_selector && source_rows == plan.rows && source_columns == plan.columns {
            return Ok(destination);
        }
        if plan.mode == 0 {
            return (source.len() == positions.len())
                .then_some(ordinal)
                .ok_or(ResidentKernelError::InvalidShape);
        }
        if (source_rows != 1 && source_rows != selected_rows)
            || (source_columns != 1 && source_columns != selected_columns)
        {
            return Err(ResidentKernelError::InvalidShape);
        }
        let row = if source_rows == 1 {
            0
        } else {
            ordinal / selected_columns
        };
        let column = if source_columns == 1 {
            0
        } else {
            ordinal % selected_columns
        };
        Ok(row * source_columns + column)
    };
    for (ordinal, &destination) in positions.iter().enumerate() {
        let left = execute_conversion_draft(next[destination].clone(), &plan.promote.step)
            .map_err(|_| ResidentKernelError::Arithmetic)?;
        let right = source[source_index(ordinal, destination)?].clone();
        let value = if plan.rational_power {
            numeric_rational_power(left, right)?
        } else {
            numeric_arithmetic(plan.arithmetic, left, right)?
        };
        next[destination] = execute_conversion_draft(value, &plan.assign.step)
            .map_err(|_| ResidentKernelError::Arithmetic)?;
    }
    let next = finalize_snapshot_data_with_work_budget(kernel, ValueDataDraft::Matrix(next), None)?;
    let changed = match output {
        ResidentValueMut::Snapshot([target]) => {
            let changed = !current
                .snapshot_eq(schemas, &next, schemas)
                .map_err(|_| ResidentKernelError::InvalidOutput)?;
            *target = Some(next);
            changed
        }
        ResidentValueMut::F64(target) => {
            let ValueData::Matrix(matrix) = next.data() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            let SequenceView::F64(values) = matrix.elements() else {
                return Err(ResidentKernelError::InvalidOutput);
            };
            if values.len() != target.len() {
                return Err(ResidentKernelError::InvalidShape);
            }
            let mut changed = false;
            for row in 0..plan.rows {
                for column in 0..plan.columns {
                    let destination = &mut target[column * plan.rows + row];
                    let value = values[row * plan.columns + column].to_f64();
                    changed |= destination.to_bits() != value.to_bits();
                    *destination = value;
                }
            }
            changed
        }
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    Ok(changed)
}

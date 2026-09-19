//! Nested selected arithmetic is owned by one addressed RMW kernel.
use super::*;
use mech_core::{
    ConversionPlan, ResolvedType, execute_conversion_draft, plan_explicit_cast,
    plan_implicit_conversion,
};

const LINEAR_ALL: u64 = 1;
const LINEAR_GATHER: u64 = 2;
const ROWS: u64 = 3;
const COLUMNS: u64 = 4;
const RECTANGLE: u64 = 5;
const WHOLE: u64 = 6;

#[derive(Clone, Copy)]
struct Stage {
    mode: u64,
    mode_input: usize,
    selector_input: usize,
    selector_count: usize,
    logical: bool,
}

#[derive(Clone, Copy)]
struct SelectedPosition {
    destination: usize,
    logical_source: Option<LogicalSource>,
}

#[derive(Clone, Copy)]
enum LogicalSource {
    Matrix { row: usize, column: usize },
    Linear { ordinal: usize },
}

#[derive(Clone)]
struct Plan {
    arithmetic: SemanticArithmetic,
    rational_power: bool,
    rows: usize,
    columns: usize,
    source_len: usize,
    source_rows: usize,
    source_columns: usize,
    dense_f64: bool,
    source: SnapshotAccessSelectorLayout,
    target: SnapshotAccessSelectorLayout,
    stages: Box<[Stage]>,
    selector_capacity: usize,
    promote: ConversionPlan,
    assign: ConversionPlan,
}

pub(super) fn bind<const OPERATION: u64>(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let arithmetic = SemanticArithmetic::from_parameter(OPERATION)
        .ok_or(ResidentKernelBindError::InvalidParameters)?;
    validate_rmw(request, request.inputs.len(), RegionPolicy::WholeValue)?;
    let [base, source, extras @ ..] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if extras.is_empty() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
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
    let (incoming, source_rows, source_columns) = match source_schema.body() {
        SchemaBody::Matrix { element, .. } => {
            let (rows, columns) = declared_matrix_dimensions(request, source)?;
            (element.as_ref(), rows, columns)
        }
        scalar => (scalar, 1, 1),
    };
    let source_len = source_rows
        .checked_mul(source_columns)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
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
    let dense_f64 = base.kind == ResidentValueKind::F64
        && source.kind == ResidentValueKind::F64
        && request.output.kind == ResidentValueKind::F64
        && base.shape.len() == rows.checked_mul(columns)
        && source.shape.len() == Some(source_len)
        && request.output.shape == base.shape;
    let snapshot_output = base.kind == ResidentValueKind::Snapshot
        && request.output.kind == ResidentValueKind::Snapshot
        && base.shape == ResidentShape::SCALAR
        && request.output.shape == ResidentShape::SCALAR
        && matches!(
            source.kind,
            ResidentValueKind::F64 | ResidentValueKind::Snapshot
        );
    if !dense_f64 && !snapshot_output {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }

    // Mode inputs are compile-time Index constants. They describe how many
    // selector inputs belong to each stage, so malformed artifacts are rejected
    // before execution while selector payloads remain live turn dependencies.
    let mut stages = Vec::new();
    let mut extra = 0usize;
    let mut selector_capacity = 0usize;
    while extra < extras.len() {
        let mode = match extras[extra].resolved_selector {
            Some(mech_core::ResidentResolvedSelector::Ordinal(ordinal)) => u64::try_from(ordinal)
                .ok()
                .and_then(|ordinal| ordinal.checked_add(1))
                .ok_or(ResidentKernelBindError::UnsupportedLayout)?,
            _ => return Err(ResidentKernelBindError::UnsupportedLayout),
        };
        let selector_count = match mode {
            LINEAR_ALL | WHOLE => 0,
            LINEAR_GATHER | ROWS | COLUMNS => 1,
            RECTANGLE => 2,
            _ => return Err(ResidentKernelBindError::UnsupportedLayout),
        };
        let selector_start = extra
            .checked_add(1)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let end = selector_start
            .checked_add(selector_count)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let selectors = extras
            .get(selector_start..end)
            .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        let mut stage_logical = false;
        for selector in selectors {
            if !positional_selector_layout(request, selector) {
                return Err(ResidentKernelBindError::UnsupportedLayout);
            }
            stage_logical |= request
                .schemas
                .get(selector.schema_id)
                .is_some_and(|schema| is_logical_selector_schema(schema.body()));
            selector_capacity = selector_capacity
                .checked_add(declared_selector_cardinality(request, selector)?)
                .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
        }
        stages.push(Stage {
            mode,
            // Resident execution omits the aliased base input.
            mode_input: extra + 1,
            selector_input: selector_start + 1,
            selector_count,
            logical: stage_logical,
        });
        extra = end;
    }
    if stages.is_empty() {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let layout = |port: &mech_core::ResidentPortLayout| SnapshotAccessSelectorLayout {
        schema: port.schema_id,
        shape: port.shape_instance.clone(),
        resident_shape: port.shape,
    };
    Ok(BoundResidentKernel::new(execute, Box::new([]))
        .with_retained_state(Arc::new(Plan {
            arithmetic,
            rational_power,
            rows,
            columns,
            source_len,
            source_rows,
            source_columns,
            dense_f64,
            source: layout(source),
            target: layout(&request.output),
            stages: stages.into_boxed_slice(),
            selector_capacity,
            promote,
            assign,
        }))
        .with_snapshot_output(snapshot_output_metadata(request))
        .with_snapshot_schemas(request.schemas.clone()))
}

fn mode_value(value: ResidentValueRef<'_>) -> Result<u64, ResidentKernelError> {
    let ResidentValueRef::Index([mode]) = value else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok(*mode)
}

fn selected_count(value: ResidentValueRef<'_>, upper: usize) -> Result<usize, ResidentKernelError> {
    let mut count = 0usize;
    selector_for_each_access_index(value, upper, |_| {
        count = count
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    Ok(count)
}

fn selection_geometry(
    plan: &Plan,
    inputs: &dyn ResidentKernelInputs,
) -> Result<(usize, usize, usize, Option<(usize, usize)>), ResidentKernelError> {
    let mut rows = plan.rows;
    let mut columns = plan.columns;
    let mut maximum = 0usize;
    let mut logical_geometry = None;
    for stage in &plan.stages {
        if mode_value(input(inputs, stage.mode_input)?)? != stage.mode {
            return Err(ResidentKernelError::InvalidInput);
        }
        let selectors = |ordinal: usize| input(inputs, stage.selector_input + ordinal);
        if stage.logical {
            logical_geometry = Some((rows, columns));
        }
        (rows, columns) = match stage.mode {
            LINEAR_ALL => (
                rows.checked_mul(columns)
                    .ok_or(ResidentKernelError::InvalidShape)?,
                1,
            ),
            LINEAR_GATHER => (
                selected_count(
                    selectors(0)?,
                    rows.checked_mul(columns)
                        .ok_or(ResidentKernelError::InvalidShape)?,
                )?,
                1,
            ),
            ROWS => (selected_count(selectors(0)?, rows)?, columns),
            COLUMNS => (rows, selected_count(selectors(0)?, columns)?),
            RECTANGLE => (
                selected_count(selectors(0)?, rows)?,
                selected_count(selectors(1)?, columns)?,
            ),
            WHOLE => (rows, columns),
            _ => return Err(ResidentKernelError::InvalidInput),
        };
        maximum = maximum.max(
            rows.checked_mul(columns)
                .ok_or(ResidentKernelError::InvalidShape)?,
        );
    }
    Ok((rows, columns, maximum, logical_geometry))
}

fn collect_selector(
    value: ResidentValueRef<'_>,
    upper: usize,
) -> Result<Vec<usize>, ResidentKernelError> {
    let count = selected_count(value, upper)?;
    let mut selected = Vec::with_capacity(count);
    selector_for_each_access_index(value, upper, |position| {
        selected.push(position);
        Ok(())
    })?;
    Ok(selected)
}

fn selected_positions(
    plan: &Plan,
    inputs: &dyn ResidentKernelInputs,
) -> Result<(Vec<SelectedPosition>, usize, usize), ResidentKernelError> {
    let mut current: Option<Vec<SelectedPosition>> = None;
    let mut rows = plan.rows;
    let mut columns = plan.columns;
    let mut linear = false;
    let base_position = |row: usize, column: usize| {
        Ok(SelectedPosition {
            destination: row
                .checked_mul(plan.columns)
                .and_then(|position| position.checked_add(column))
                .ok_or(ResidentKernelError::InvalidShape)?,
            logical_source: None,
        })
    };
    for stage in &plan.stages {
        let previous =
            |row: usize, column: usize| -> Result<SelectedPosition, ResidentKernelError> {
                let mut position = match current.as_ref() {
                    Some(positions) => positions
                        .get(
                            row.checked_mul(columns)
                                .and_then(|position| position.checked_add(column))
                                .ok_or(ResidentKernelError::InvalidShape)?,
                        )
                        .copied()
                        .ok_or(ResidentKernelError::InvalidShape),
                    None => base_position(row, column),
                }?;
                if stage.logical {
                    position.logical_source = if linear {
                        let ordinal = column
                            .checked_mul(rows)
                            .and_then(|ordinal| ordinal.checked_add(row))
                            .ok_or(ResidentKernelError::InvalidShape)?;
                        Some(LogicalSource::Linear { ordinal })
                    } else {
                        Some(LogicalSource::Matrix { row, column })
                    };
                }
                Ok(position)
            };
        let selectors = |ordinal: usize| input(inputs, stage.selector_input + ordinal);
        let (next_rows, next_columns, mut next) = match stage.mode {
            WHOLE => continue,
            LINEAR_ALL => {
                let count = rows
                    .checked_mul(columns)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let mut next = Vec::with_capacity(count);
                for column in 0..columns {
                    for row in 0..rows {
                        next.push(previous(row, column)?);
                    }
                }
                (count, 1, next)
            }
            LINEAR_GATHER => {
                let count = rows
                    .checked_mul(columns)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let selected = collect_selector(selectors(0)?, count)?;
                let mut next = Vec::with_capacity(selected.len());
                for position in selected {
                    next.push(previous(position % rows, position / rows)?);
                }
                (next.len(), 1, next)
            }
            ROWS => {
                let selected = collect_selector(selectors(0)?, rows)?;
                let capacity = selected
                    .len()
                    .checked_mul(columns)
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let mut next = Vec::with_capacity(capacity);
                for row in &selected {
                    for column in 0..columns {
                        next.push(previous(*row, column)?);
                    }
                }
                (selected.len(), columns, next)
            }
            COLUMNS => {
                let selected = collect_selector(selectors(0)?, columns)?;
                let capacity = rows
                    .checked_mul(selected.len())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let mut next = Vec::with_capacity(capacity);
                for row in 0..rows {
                    for column in &selected {
                        next.push(previous(row, *column)?);
                    }
                }
                (rows, selected.len(), next)
            }
            RECTANGLE => {
                let selected_rows = collect_selector(selectors(0)?, rows)?;
                let selected_columns = collect_selector(selectors(1)?, columns)?;
                let capacity = selected_rows
                    .len()
                    .checked_mul(selected_columns.len())
                    .ok_or(ResidentKernelError::InvalidShape)?;
                let mut next = Vec::with_capacity(capacity);
                for row in &selected_rows {
                    for column in &selected_columns {
                        next.push(previous(*row, *column)?);
                    }
                }
                (selected_rows.len(), selected_columns.len(), next)
            }
            _ => return Err(ResidentKernelError::InvalidInput),
        };
        if next.len()
            != next_rows
                .checked_mul(next_columns)
                .ok_or(ResidentKernelError::InvalidShape)?
        {
            return Err(ResidentKernelError::InvalidShape);
        }
        current = Some(core::mem::take(&mut next));
        rows = next_rows;
        columns = next_columns;
        linear |= matches!(stage.mode, LINEAR_ALL | LINEAR_GATHER);
    }
    let positions = match current {
        Some(positions) => positions,
        None => (0..plan
            .rows
            .checked_mul(plan.columns)
            .ok_or(ResidentKernelError::InvalidShape)?)
            .map(|destination| SelectedPosition {
                destination,
                logical_source: None,
            })
            .collect(),
    };
    Ok((positions, rows, columns))
}

fn source_index(
    plan: &Plan,
    source_len: usize,
    selected_rows: usize,
    selected_columns: usize,
    logical_geometry: Option<(usize, usize)>,
    ordinal: usize,
    position: SelectedPosition,
) -> Result<usize, ResidentKernelError> {
    if source_len == 1 {
        return Ok(0);
    }
    if logical_geometry == Some((plan.source_rows, plan.source_columns)) {
        let source = position
            .logical_source
            .ok_or(ResidentKernelError::InvalidShape)?;
        let (row, column) = match source {
            LogicalSource::Matrix { row, column } => (row, column),
            LogicalSource::Linear { ordinal } => {
                if plan.source_rows == 0 {
                    return Err(ResidentKernelError::InvalidShape);
                }
                (ordinal % plan.source_rows, ordinal / plan.source_rows)
            }
        };
        if row >= plan.source_rows || column >= plan.source_columns {
            return Err(ResidentKernelError::InvalidShape);
        }
        return row
            .checked_mul(plan.source_columns)
            .and_then(|source| source.checked_add(column))
            .ok_or(ResidentKernelError::InvalidShape);
    }
    if (plan.source_rows != 1 && plan.source_rows != selected_rows)
        || (plan.source_columns != 1 && plan.source_columns != selected_columns)
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    let row = if plan.source_rows == 1 {
        0
    } else {
        ordinal / selected_columns
    };
    let column = if plan.source_columns == 1 {
        0
    } else {
        ordinal % selected_columns
    };
    Ok(row * plan.source_columns + column)
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
    let expected_inputs = plan
        .stages
        .last()
        .and_then(|stage| stage.selector_input.checked_add(stage.selector_count))
        .ok_or(ResidentKernelError::InvalidInput)?;
    if inputs.len() != expected_inputs {
        return Err(ResidentKernelError::InvalidInput);
    }
    let count = plan
        .rows
        .checked_mul(plan.columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let (selected_rows, selected_columns, maximum_population, logical_geometry) =
        selection_geometry(plan, inputs)?;
    let selected_count = selected_rows
        .checked_mul(selected_columns)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let snapshot_elements = usize::from(!plan.dense_f64)
        .checked_mul(
            count
                .checked_add(plan.source_len)
                .ok_or(ResidentKernelError::InvalidShape)?,
        )
        .ok_or(ResidentKernelError::InvalidShape)?;
    let elements = snapshot_elements
        .checked_add(plan.selector_capacity)
        .and_then(|value| value.checked_add(maximum_population.checked_mul(2)?))
        .ok_or(ResidentKernelError::InvalidShape)?;
    let bytes = snapshot_elements
        .checked_mul(core::mem::size_of::<ValueDataDraft>())
        .and_then(|value| {
            maximum_population
                .checked_mul(2)?
                .checked_mul(core::mem::size_of::<SelectedPosition>())?
                .checked_add(
                    plan.selector_capacity
                        .checked_mul(core::mem::size_of::<usize>())?,
                )?
                .checked_add(value)
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    super::super::budget::PreparedKernel::new((), super::super::budget::resident_cost! {
        compute_work: super::super::budget::checked_u64(elements.checked_mul(if plan.rational_power { 128 } else { 8 }).ok_or(ResidentKernelError::InvalidShape)?)?,
        comparison_work: super::super::budget::checked_u64(if plan.dense_f64 { selected_count } else { count })?,
        temporary_bytes: super::super::budget::checked_u64(bytes)?,
        cloned_bytes: super::super::budget::checked_u64(bytes)?,
        retained_nodes: super::super::budget::checked_u64(elements.checked_mul(4).and_then(|value| value.checked_add(8)).ok_or(ResidentKernelError::InvalidShape)?)?,
        output_elements: if plan.dense_f64 { selected_count } else { count },
        output_bytes: super::super::budget::checked_u64((if plan.dense_f64 { selected_count } else { count }).checked_mul(if plan.dense_f64 { core::mem::size_of::<f64>() } else { core::mem::size_of::<ValueDataDraft>() }).ok_or(ResidentKernelError::InvalidShape)?)?,
        ..super::super::budget::KernelCostEstimate::default()
    }).admit()?.into_plan();

    let (positions, actual_rows, actual_columns) = selected_positions(plan, inputs)?;
    if (actual_rows, actual_columns) != (selected_rows, selected_columns)
        || positions.len() != selected_count
    {
        return Err(ResidentKernelError::InvalidShape);
    }
    // Validate every RHS route before the dense lane mutates its aliased
    // candidate. A late intermediate-view coordinate must reject atomically.
    for (ordinal, position) in positions.iter().copied().enumerate() {
        source_index(
            plan,
            plan.source_len,
            selected_rows,
            selected_columns,
            logical_geometry,
            ordinal,
            position,
        )?;
    }
    if plan.dense_f64 {
        let ResidentValueRef::F64(source) = input(inputs, 0)? else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let ResidentValueMut::F64(target) = output else {
            return Err(ResidentKernelError::InvalidOutput);
        };
        if source.len() != plan.source_len || target.len() != count {
            return Err(ResidentKernelError::InvalidShape);
        }
        let mut changed = false;
        for (ordinal, position) in positions.into_iter().enumerate() {
            let source_position = source_index(
                plan,
                source.len(),
                selected_rows,
                selected_columns,
                logical_geometry,
                ordinal,
                position,
            )?;
            let source_row = source_position / plan.source_columns;
            let source_column = source_position % plan.source_columns;
            let source_position = source_column * plan.source_rows + source_row;
            let row = position.destination / plan.columns;
            let column = position.destination % plan.columns;
            let destination = column * plan.rows + row;
            let next = compound_f64(
                plan.arithmetic,
                target[destination],
                source[source_position],
            );
            changed |= target[destination].to_bits() != next.to_bits();
            target[destination] = next;
        }
        return Ok(changed);
    }

    let current_ref = match &output {
        ResidentValueMut::F64(values) => ResidentValueRef::F64(values),
        ResidentValueMut::Snapshot(values) => ResidentValueRef::Snapshot(values),
        _ => return Err(ResidentKernelError::InvalidOutput),
    };
    let current = selector_value(schemas, &plan.target, current_ref)?;
    let source = selector_value(schemas, &plan.source, input(inputs, 0)?)?;
    let ValueDataDraft::Matrix(mut next) = current
        .canonical_data_draft()
        .map_err(|_| ResidentKernelError::InvalidOutput)?
    else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let source = snapshot_numeric_elements(&source)?;
    if next.len() != count || source.len() != plan.source_len {
        return Err(ResidentKernelError::InvalidShape);
    }
    for (ordinal, position) in positions.into_iter().enumerate() {
        let destination = position.destination;
        let left = execute_conversion_draft(next[destination].clone(), &plan.promote.step)
            .map_err(|_| ResidentKernelError::Arithmetic)?;
        let right = source[source_index(
            plan,
            source.len(),
            selected_rows,
            selected_columns,
            logical_geometry,
            ordinal,
            position,
        )?]
        .clone();
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

//! Borrowed table joins with one admitted staging buffer and atomic canonical publication.
use super::budget::{
    self, MutationRetainedNodeFootprint, PreparedKernel, PreparedMutationPlan,
    PublishedOutputFootprint, ResidentBudgetMeter, checked_cost_product, checked_cost_sum,
    checked_u64,
};
use crate::intrinsics::table_ops::{JoinMode, joined_table_fields, sequence_language_eq_at};
use mech_core::snapshot::{SequenceView, TableColumnDraft, TableSnapshotBuilder, ValueFootprint};
use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, ImplementationMemoryClass, MResult,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, SchemaBody, SchemaField, SchemaId, SchemaTable, Value, ValueData,
};
use std::sync::Arc;

type Result<T> = std::result::Result<T, ResidentKernelError>;
type RowPair = (Option<usize>, Option<usize>);

#[derive(Clone, Debug)]
struct Projection {
    left: Option<usize>,
    right: Option<usize>,
}

#[derive(Clone, Debug)]
struct TableJoinPlan {
    mode: JoinMode,
    input_schemas: [SchemaId; 2],
    output_schema: SchemaId,
    left: Box<[SchemaField]>,
    right: Box<[SchemaField]>,
    output: Box<[SchemaField]>,
    common: Box<[(usize, usize)]>,
    projections: Box<[Projection]>,
    schemas: Arc<SchemaTable>,
}

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    macro_rules! register {
        ($name:literal, $mode:ident) => {
            builder.insert_resident_factory(
                ["table"],
                $name,
                ImplementationMemoryClass::CanonicalFinalize,
                |request| bind(request, JoinMode::$mode),
            )?;
        };
    }
    register!("join", Inner);
    register!("left-outer-join", LeftOuter);
    register!("right-outer-join", RightOuter);
    register!("full-outer-join", FullOuter);
    register!("left-semi-join", LeftSemi);
    register!("left-anti-join", LeftAnti);
    Ok(())
}

fn bind(
    request: &ResidentKernelBindRequest<'_>,
    mode: JoinMode,
) -> std::result::Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(actual) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    let expected = mech_core::maintained_operation_contract("table/join", 2, false)
        .expect("maintained table join contract");
    let inputs = expected
        .inputs
        .resolve(2)
        .map_err(|_| ResidentKernelBindError::UnsupportedContract)?;
    let output = &expected.outputs[0];
    if request.inputs.len() != 2
        || actual.inputs.len() != 2
        || actual.outputs.len() != 1
        || actual.interaction != expected.interaction
        || actual
            .inputs
            .iter()
            .zip(request.inputs)
            .zip(inputs.iter())
            .any(|((port, layout), policy)| {
                port.schema != layout.schema_id
                    || port.access != policy.access
                    || port.delivery != policy.delivery
            })
        || actual.outputs[0].schema != request.output.schema_id
        || actual.outputs[0].access != output.access
        || actual.outputs[0].delivery != output.delivery
        || actual.outputs[0].construction != output.construction
        || actual.outputs[0].alias != output.alias
        || actual.outputs[0].change_detection != output.change_detection
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let fields = |port: &mech_core::ResidentPortLayout| {
        if port.kind != ResidentValueKind::Snapshot || port.shape != ResidentShape::SCALAR {
            return Err(ResidentKernelBindError::UnsupportedLayout);
        }
        match request
            .schemas
            .get(port.schema_id)
            .and_then(|schema| schema.closed_body(&port.shape_instance).ok())
        {
            Some(SchemaBody::Table { columns, .. }) => Ok(columns),
            _ => Err(ResidentKernelBindError::UnsupportedLayout),
        }
    };
    let left = fields(&request.inputs[0])?;
    let right = fields(&request.inputs[1])?;
    let output = fields(&request.output)?;
    let derived = joined_table_fields(&left, &right, mode)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Table { columns, .. } = output_schema.body() else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    if columns.len() != derived.len()
        || columns
            .iter()
            .zip(&derived)
            .any(|(expected, actual)| expected.name != actual.name)
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let components = columns
        .iter()
        .zip(derived)
        .map(|(expected, actual)| (&expected.schema, actual.schema))
        .collect::<Vec<_>>();
    // Snapshot output extents need not equal activation placeholders. Resolve
    // field witnesses against the declared schema, preserving its constraints.
    mech_core::shape_for_schema_components(output_schema, &components, None)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let common = left
        .iter()
        .enumerate()
        .filter_map(|(l, field)| {
            right
                .iter()
                .position(|r| r.name == field.name)
                .map(|r| (l, r))
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let projections = output
        .iter()
        .map(|field| Projection {
            left: left.iter().position(|l| l.name == field.name),
            right: right.iter().position(|r| r.name == field.name),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok(
        BoundResidentKernel::new(execute, Box::new([])).with_retained_state(Arc::new(
            TableJoinPlan {
                mode,
                input_schemas: [request.inputs[0].schema_id, request.inputs[1].schema_id],
                output_schema: request.output.schema_id,
                left,
                right,
                output,
                common,
                projections,
                schemas: Arc::new(request.schemas.clone()),
            },
        )),
    )
}

struct CurrentTableSchemas {
    left: Box<[SchemaField]>,
    right: Box<[SchemaField]>,
    output: Box<[SchemaField]>,
    bytes: u64,
    nodes: u64,
    work: u64,
}

fn current_schemas(
    plan: &TableJoinPlan,
    left: &Value,
    right: &Value,
    previous: Option<&Value>,
) -> Result<CurrentTableSchemas> {
    let mut units = 0u64;
    let mut parameters = 0u64;
    for id in [
        plan.input_schemas[0],
        plan.input_schemas[1],
        plan.output_schema,
    ] {
        let entry = plan
            .schemas
            .entry(id)
            .ok_or(ResidentKernelError::InvalidInput)?;
        let schema = entry.schema();
        units = checked_cost_sum(&[units, checked_u64(entry.canonical_bytes().len())?])?;
        parameters = checked_cost_sum(&[
            parameters,
            checked_u64(schema.dimension_parameters().len())?,
        ])?;
    }
    // Every schema node/field/dimension occupies at least one encoded byte.
    // Bound all schema clones and shape-witness containers, including the
    // component closure performed by the shared resolver after admission.
    let bytes = checked_cost_product(&[
        units,
        5,
        checked_u64(
            std::mem::size_of::<SchemaBody>()
                + std::mem::size_of::<SchemaField>()
                + std::mem::size_of::<mech_core::DimensionExpr>()
                + std::mem::size_of::<Vec<u64>>(),
        )?,
    ])?;
    let nodes = checked_cost_product(&[units, 5])?;
    let columns = checked_u64(plan.left.len() + plan.right.len() + plan.output.len())?;
    let work = checked_cost_product(&[
        units,
        checked_cost_sum(&[
            4,
            checked_cost_product(&[columns, 4])?,
            checked_cost_product(&[parameters + 1, parameters + 1])?,
        ])?,
    ])?;
    let mut meter = ResidentBudgetMeter::default();
    for value in [Some(left), Some(right), previous].into_iter().flatten() {
        budget::measure_canonical_value_footprint(&mut meter, value, &plan.schemas)?;
    }
    let mut cost = meter.estimate();
    cost.add_temporary_bytes(bytes)?;
    cost.set_cloned_bytes(bytes);
    cost.set_retained_nodes(checked_cost_sum(&[cost.retained_nodes(), nodes])?);
    cost.set_compute_work(checked_cost_sum(&[cost.compute_work(), work])?)?;
    PreparedKernel::new((), cost).admit()?.into_plan();
    let fields = |value: &Value| match plan
        .schemas
        .get(value.schema())
        .and_then(|schema| schema.closed_body(value.shape()).ok())
    {
        Some(SchemaBody::Table { columns, .. }) => Ok(columns),
        _ => Err(ResidentKernelError::InvalidInput),
    };
    let left = fields(left)?;
    let right = fields(right)?;
    let output = joined_table_fields(&left, &right, plan.mode)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    Ok(CurrentTableSchemas {
        left,
        right,
        output,
        bytes,
        nodes,
        work,
    })
}

struct Table<'a> {
    value: &'a Value,
    columns: &'a [SchemaField],
    rows: usize,
}

impl<'a> Table<'a> {
    fn new(value: &'a Value, columns: &'a [SchemaField]) -> Result<Self> {
        let ValueData::Table(table) = value.data() else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let rows = table.column(0).map_or(0, SequenceView::len);
        if table.len() != columns.len()
            || (0..table.len()).any(|column| {
                table
                    .column(column)
                    .is_none_or(|values| values.len() != rows)
            })
        {
            return Err(ResidentKernelError::InvalidInput);
        }
        Ok(Self {
            value,
            columns,
            rows,
        })
    }
    fn column(&self, column: usize) -> SequenceView<'_> {
        let ValueData::Table(table) = self.value.data() else {
            unreachable!("validated table")
        };
        table.column(column).expect("bound column")
    }
}

fn key_work(
    meter: &mut ResidentBudgetMeter,
    table: &Table<'_>,
    column: usize,
    row: usize,
) -> Result<u64> {
    let schema = &table.columns[column].schema;
    match table.column(column) {
        SequenceView::Values(values) => {
            budget::measure_canonical_data_comparison_work(meter, schema, &values[row])
        }
        values => {
            let footprint = mech_core::snapshot::canonical_sequence_element_retained_footprint(
                schema, values, row,
            )
            .map_err(|_| ResidentKernelError::InvalidInput)?;
            meter.charge_compute_work(1)?;
            Ok(footprint.encoded_bytes.max(1))
        }
    }
}

fn rows_match(
    plan: &TableJoinPlan,
    left: &Table<'_>,
    right: &Table<'_>,
    l: usize,
    r: usize,
    meter: &mut ResidentBudgetMeter,
) -> Result<bool> {
    meter.charge_compute_work(1)?;
    for &(lc, rc) in &plan.common {
        let work = checked_cost_sum(&[
            key_work(meter, left, lc, l)?,
            key_work(meter, right, rc, r)?,
        ])?;
        // The footprint walk and the subsequent language equality are separate work.
        meter.charge_comparison_work(work)?;
        if !sequence_language_eq_at(
            &left.columns[lc].schema,
            left.column(lc),
            l,
            right.column(rc),
            r,
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Stream matches in the provider's row order. A second borrowed scan finds
/// unmatched right rows, so neither matched flags nor an owned pair list is
/// allocated while discovering data-dependent cardinality.
fn visit_pairs(
    plan: &TableJoinPlan,
    left: &Table<'_>,
    right: &Table<'_>,
    meter: &mut ResidentBudgetMeter,
    mut visit: impl FnMut(RowPair, &mut ResidentBudgetMeter) -> Result<()>,
) -> Result<()> {
    for l in 0..left.rows {
        meter.charge_compute_work(1)?;
        let mut matched = false;
        for r in 0..right.rows {
            if !rows_match(plan, left, right, l, r, meter)? {
                continue;
            }
            matched = true;
            match plan.mode {
                JoinMode::Inner
                | JoinMode::LeftOuter
                | JoinMode::RightOuter
                | JoinMode::FullOuter => visit((Some(l), Some(r)), meter)?,
                JoinMode::LeftSemi | JoinMode::LeftAnti => break,
            }
        }
        match plan.mode {
            JoinMode::LeftOuter | JoinMode::FullOuter if !matched => visit((Some(l), None), meter)?,
            JoinMode::LeftSemi if matched => visit((Some(l), None), meter)?,
            JoinMode::LeftAnti if !matched => visit((Some(l), None), meter)?,
            _ => {}
        }
    }
    if matches!(plan.mode, JoinMode::RightOuter | JoinMode::FullOuter) {
        for r in 0..right.rows {
            meter.charge_compute_work(1)?;
            let mut matched = false;
            for l in 0..left.rows {
                if rows_match(plan, left, right, l, r, meter)? {
                    matched = true;
                    break;
                }
            }
            if !matched {
                visit((None, Some(r)), meter)?
            }
        }
    }
    Ok(())
}

fn selected<'a>(
    projection: &Projection,
    pair: RowPair,
    left: &'a Table<'a>,
    right: &'a Table<'a>,
) -> Option<(&'a Table<'a>, usize, usize)> {
    projection
        .left
        .zip(pair.0)
        .map(|(column, row)| (left, column, row))
        .or_else(|| {
            projection
                .right
                .zip(pair.1)
                .map(|(column, row)| (right, column, row))
        })
}

#[derive(Debug)]
struct JoinMaterialization {
    rows: usize,
}

fn prepare(
    plan: &TableJoinPlan,
    left: &Table<'_>,
    right: &Table<'_>,
    previous: Option<&Value>,
    current: &CurrentTableSchemas,
) -> Result<JoinMaterialization> {
    let mut meter = ResidentBudgetMeter::default();
    let left_footprint =
        budget::measure_canonical_value_footprint(&mut meter, left.value, &plan.schemas)?;
    let right_footprint =
        budget::measure_canonical_value_footprint(&mut meter, right.value, &plan.schemas)?;
    let previous_footprint = previous
        .map(|value| budget::measure_canonical_value_footprint(&mut meter, value, &plan.schemas))
        .transpose()?;
    let current_nodes = checked_cost_sum(&[
        left_footprint.node_count,
        right_footprint.node_count,
        previous_footprint.map_or(0, |f| f.node_count),
    ])?;
    let mut rows = 0usize;
    let mut data = ValueFootprint::zero();
    visit_pairs(plan, left, right, &mut meter, |pair, meter| {
        rows = rows
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        if checked_u64(rows)? > budget::MAX_RESIDENT_OUTPUT_ELEMENTS as u64 {
            return Err(ResidentKernelError::InvalidShape);
        }
        for (projection, field) in plan.projections.iter().zip(&plan.output) {
            meter.charge_compute_work(1)?;
            let needs_option = match selected(projection, pair, left, right) {
                Some((table, column, row)) => {
                    super::numeric::selected_sequence_footprint(
                        &mut data,
                        meter,
                        &table.columns[column].schema,
                        table.column(column),
                        row,
                    )?;
                    matches!(field.schema, SchemaBody::Option(_))
                        && !matches!(table.columns[column].schema, SchemaBody::Option(_))
                }
                None if matches!(field.schema, SchemaBody::Option(_)) => true,
                None => return Err(ResidentKernelError::InvalidOutput),
            };
            if needs_option {
                data = data
                    .checked_add(ValueFootprint {
                        encoded_bytes: 1,
                        retained_bytes: checked_u64(std::mem::size_of::<ValueData>())?,
                        node_count: 1,
                    })
                    .map_err(|_| ResidentKernelError::InvalidShape)?;
                meter.charge_retained_nodes(1)?;
            }
        }
        Ok(())
    })?;
    let cells = checked_cost_product(&[checked_u64(rows)?, checked_u64(plan.output.len())?])?;
    let labels = plan.output.iter().try_fold(0u64, |sum, field| {
        checked_cost_sum(&[sum, checked_u64(field.name.len())?])
    })?;
    // Canonical cell staging and packed publication coexist. The shared draft
    // and finalization bounds conservatively cover both ValueData populations,
    // per-column packing, and retained nested Dynamic ownership.
    let containers = checked_cost_sum(&[
        checked_u64(std::mem::size_of::<ValueData>())?,
        checked_cost_product(&[
            checked_u64(plan.output.len())?,
            checked_u64(
                std::mem::size_of::<TableColumnDraft>() + std::mem::size_of::<ValueData>(),
            )?,
        ])?,
        labels,
    ])?;
    data = data
        .checked_add(ValueFootprint {
            encoded_bytes: checked_cost_sum(&[
                labels,
                16,
                checked_cost_product(&[8, checked_u64(plan.output.len())?])?,
            ])?,
            retained_bytes: containers,
            node_count: 1,
        })
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let entry = plan
        .schemas
        .entry(plan.output_schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schema = entry.schema();
    let final_output =
        budget::projected_canonical_value_footprint(data, schema.dimension_parameters().len())?;
    let draft_nodes = checked_cost_sum(&[data.node_count, cells])?;
    let footprint = mech_core::CurrentMemoryFootprint {
        logical_elements: checked_u64(rows)?,
        payload_bytes: final_output.retained_bytes,
        encoded_bytes: final_output.encoded_bytes,
        retained_nodes: draft_nodes,
        schema_bytes: checked_u64(entry.canonical_bytes().len())?,
        shape_parameter_count: checked_u64(schema.dimension_parameters().len())?,
        ..mech_core::CurrentMemoryFootprint::default()
    };
    let draft_bytes = mech_core::canonical_snapshot_draft_bytes(footprint)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let finalization_bytes = mech_core::canonical_snapshot_finalization_bytes(footprint)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let mut cost = meter.estimate();
    // Repeat the same bounded borrowed traversal for materialization; reserve
    // both passes plus shape validation and final table packing before writing.
    let column_order_work = checked_cost_product(&[
        checked_u64(plan.output.len())?,
        checked_cost_sum(&[labels, checked_u64(plan.output.len())?])?,
    ])?;
    cost.set_comparison_work(checked_cost_sum(&[
        checked_cost_product(&[cost.comparison_work(), 2])?,
        column_order_work,
    ])?)?;
    cost.set_compute_work(checked_cost_sum(&[
        checked_cost_product(&[cost.compute_work(), 2])?,
        checked_cost_product(&[data.node_count, 3])?,
        checked_cost_product(&[data.encoded_bytes, 2])?,
        cells,
        column_order_work,
        current.work,
        checked_cost_product(&[cells, checked_u64(entry.canonical_bytes().len())?])?,
    ])?)?;
    cost.add_temporary_bytes(checked_cost_sum(&[
        draft_bytes,
        finalization_bytes,
        containers,
        current.bytes,
    ])?)?;
    cost.set_cloned_bytes(checked_cost_sum(&[data.retained_bytes, current.bytes])?);
    if let (Some(previous), Some(footprint)) = (previous, previous_footprint) {
        let work = budget::projected_language_equality_work(
            &plan.schemas,
            previous,
            footprint,
            plan.output_schema,
            schema.dimension_parameters().len(),
            final_output,
        )?;
        cost.set_comparison_work(checked_cost_sum(&[cost.comparison_work(), work])?)?;
        cost.set_compute_work(checked_cost_sum(&[cost.compute_work(), work])?)?;
    }
    cost.set_retained_nodes(0);
    PreparedMutationPlan::new(
        JoinMaterialization { rows },
        PublishedOutputFootprint {
            elements: checked_u64(rows)?,
            retained_bytes: final_output.retained_bytes,
            retained_nodes: final_output.node_count,
        },
        MutationRetainedNodeFootprint {
            current_persistent: current_nodes,
            temporary_draft: draft_nodes,
            normalized_plan: current.nodes,
        },
        cost,
    )?
    .admit()
    .map(|admitted| admitted.into_plan())
}

fn execute(
    kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool> {
    let plan = kernel
        .retained_state::<TableJoinPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let input = |index| match inputs.get(index) {
        Some(ResidentValueRef::Snapshot([Some(value)]))
            if value.schema() == plan.input_schemas[index] =>
        {
            Ok(value)
        }
        _ => Err(ResidentKernelError::InvalidInput),
    };
    let left_value = input(0)?;
    let right_value = input(1)?;
    let ResidentValueMut::Snapshot([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let current = current_schemas(plan, left_value, right_value, target.as_ref())?;
    let left = Table::new(left_value, &current.left)?;
    let right = Table::new(right_value, &current.right)?;
    let prepared = prepare(plan, &left, &right, target.as_ref(), &current)?;
    let schema = plan
        .schemas
        .get(plan.output_schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let SchemaBody::Table {
        columns: declared,
        rows,
    } = schema.body()
    else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let components = declared
        .iter()
        .zip(current.output)
        .map(|(expected, actual)| (&expected.schema, actual.schema))
        .collect::<Vec<_>>();
    let shape =
        mech_core::shape_for_schema_components(schema, &components, Some((rows, prepared.rows)))
            .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let sources = [left_value, right_value];
    let mut builder = TableSnapshotBuilder::bind(
        plan.output_schema,
        shape,
        prepared.rows,
        Arc::clone(&plan.schemas),
        &sources,
    )
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    visit_pairs(
        plan,
        &left,
        &right,
        &mut ResidentBudgetMeter::default(),
        |pair, _| {
            for (output_column, projection) in plan.projections.iter().enumerate() {
                let source =
                    selected(projection, pair, &left, &right).map(|(table, column, row)| {
                        let input = if std::ptr::eq(table.value, left_value) {
                            0
                        } else {
                            1
                        };
                        (input, column, row)
                    });
                builder
                    .push(output_column, source)
                    .map_err(|_| ResidentKernelError::InvalidOutput)?;
            }
            Ok(())
        },
    )?;
    let next = builder
        .finish()
        .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let changed = target
        .as_ref()
        .map(|previous| {
            previous
                .language_eq(&plan.schemas, &next, &plan.schemas)
                .map(|equal| !equal)
        })
        .transpose()
        .map_err(|_| ResidentKernelError::InvalidOutput)?
        .unwrap_or(true);
    *target = Some(next);
    Ok(changed)
}

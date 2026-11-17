//! Table row gathers share the existing selector, admission and publication path.
use super::*;
use crate::resident::budget::{checked_cost_product, checked_cost_sum, checked_u64};

pub(super) fn bind(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let [source, selector] = request.inputs else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let source_schema = request
        .schemas
        .get(source.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let output_schema = request
        .schemas
        .get(request.output.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let (
        SchemaBody::Table {
            columns: source_columns,
            ..
        },
        SchemaBody::Table { columns, .. },
    ) = (source_schema.body(), output_schema.body())
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let selector_schema = request
        .schemas
        .get(selector.schema_id)
        .ok_or(ResidentKernelBindError::UnsupportedLayout)?;
    let source_body = source_schema
        .closed_body(&source.shape_instance)
        .map_err(|_| ResidentKernelBindError::UnsupportedLayout)?;
    let SchemaBody::Table {
        columns: current, ..
    } = source_body
    else {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    };
    let components = columns
        .iter()
        .zip(current.iter())
        .map(|(expected, actual)| (&expected.schema, actual.schema.clone()))
        .collect::<Vec<_>>();
    if source_columns.len() != columns.len()
        || source_columns
            .iter()
            .zip(columns)
            .any(|(source, output)| source.name != output.name)
        || mech_core::shape_for_schema_components(output_schema, &components, None).is_err()
        || request.output.kind != ResidentValueKind::Snapshot
        || request.output.shape != ResidentShape::SCALAR
        || !is_access_positional_selector_schema(selector_schema.body())
        || !access_selector_layout_matches_schema(selector, selector_schema.body())
    {
        return Err(ResidentKernelBindError::UnsupportedLayout);
    }
    let plan = SnapshotAccessPlan {
        dense_source: None,
        matrix_constructor: None,
        selectors: vec![SnapshotAccessSelectorLayout {
            schema: selector.schema_id,
            shape: selector.shape_instance.clone(),
            resident_shape: selector.shape,
        }]
        .into_boxed_slice(),
        matrix_mode: Some(ResolvedSelectionMode::Rows),
        source_dimensions: None,
        output_dimensions: None,
        output_dimensions_dynamic: false,
        linear_gather_row: false,
        output_geometry: ResolvedAccessGeometry {
            logical_output_rows: 1,
            logical_output_columns: 1,
        },
        aggregate_ordinal: None,
        output_schema: request.output.schema_id,
    };
    Ok(
        bound(snapshot_access, Vec::<u64>::new().into_boxed_slice())?
            .with_retained_state(Arc::new(plan))
            .with_snapshot_output(snapshot_output_metadata(request))
            .with_snapshot_schemas(request.schemas.clone()),
    )
}

pub(super) fn output_shape(
    kernel: &BoundResidentKernel,
    source: &mech_core::Value,
    count: usize,
) -> Result<ShapeInstance, ResidentKernelError> {
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let target = schemas
        .get(metadata.schema)
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let source_schema = source
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    let (
        SchemaBody::Table { columns, rows },
        SchemaBody::Table {
            columns: actual, ..
        },
    ) = (
        target.body(),
        source_schema
            .closed_body(source.shape())
            .map_err(|_| ResidentKernelError::InvalidInput)?,
    )
    else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let components = columns
        .iter()
        .zip(actual.iter())
        .map(|(expected, actual)| (&expected.schema, actual.schema.clone()))
        .collect::<Vec<_>>();
    mech_core::shape_for_schema_components(target, &components, Some((rows, count)))
        .map_err(|_| ResidentKernelError::InvalidOutput)
}

fn row_count(table: &mech_core::snapshot::TableValue) -> Result<usize, ResidentKernelError> {
    let rows = table.column(0).map_or(0, SequenceView::len);
    if (0..table.len()).any(|column| {
        table
            .column(column)
            .is_none_or(|values| values.len() != rows)
    }) {
        return Err(ResidentKernelError::InvalidInput);
    }
    Ok(rows)
}

pub(super) fn output_cost(
    table: &mech_core::snapshot::TableValue,
    columns: &[mech_core::SchemaField],
    selector: ResidentValueRef<'_>,
    meter: &mut super::super::budget::ResidentBudgetMeter,
) -> Result<SnapshotAccessOutputCost, ResidentKernelError> {
    let rows = row_count(table)?;
    let mut selected = 0usize;
    let mut footprint = ValueFootprint::zero();
    let mut finalization_work = 0u64;
    selector_for_each_access_index(selector, rows, |row| {
        selected = selected
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.charge_compute_work(1)?;
        for (column, field) in columns.iter().enumerate() {
            selected_sequence_footprint_with_finalization(
                &mut footprint,
                &mut finalization_work,
                meter,
                &field.schema,
                table
                    .column(column)
                    .ok_or(ResidentKernelError::InvalidInput)?,
                row,
            )?;
        }
        Ok(())
    })?;
    let labels = columns.iter().try_fold(0u64, |total, field| {
        checked_cost_sum(&[total, checked_u64(field.name.len())?])
    })?;
    let containers = checked_cost_sum(&[
        checked_u64(core::mem::size_of::<ValueData>())?,
        labels,
        checked_cost_product(&[
            checked_u64(columns.len())?,
            checked_u64(core::mem::size_of::<mech_core::snapshot::TableColumnDraft>())?,
        ])?,
    ])?;
    footprint = footprint
        .checked_add(ValueFootprint {
            encoded_bytes: checked_cost_sum(&[
                labels,
                16,
                checked_cost_product(&[8, checked_u64(columns.len())?])?,
            ])?,
            retained_bytes: containers,
            node_count: checked_cost_sum(&[1, checked_u64(columns.len())?])?,
        })
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    finalization_work = checked_cost_sum(&[
        finalization_work,
        checked_cost_product(&[
            checked_u64(columns.len())?,
            checked_cost_sum(&[labels, checked_u64(columns.len())?])?,
        ])?,
    ])?;
    Ok(SnapshotAccessOutputCost {
        footprint,
        // Table population counts rows; the footprint already includes every
        // selected cell and each column's staging container.
        count: selected,
        index_elements: selected,
        selected_ordinal: None,
        finalization_work,
        output_dimensions: None,
    })
}

pub(super) fn data(
    table: &mech_core::snapshot::TableValue,
    columns: &[mech_core::SchemaField],
    selector: &mech_core::Value,
) -> Result<ValueDataDraft, ResidentKernelError> {
    let rows = row_count(table)?;
    let indices = access_indices(selector, rows)?;
    let columns = columns
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let values = table
                .column(index)
                .ok_or(ResidentKernelError::InvalidInput)?;
            Ok(mech_core::snapshot::TableColumnDraft {
                name: field.name.clone(),
                values: indices
                    .iter()
                    .map(|row| sequence_data_draft_at(&field.schema, values, *row))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_boxed_slice(),
            })
        })
        .collect::<Result<Vec<_>, ResidentKernelError>>()?;
    Ok(ValueDataDraft::Table(columns.into_boxed_slice()))
}

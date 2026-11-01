//! Compact canonical String-matrix selection after ordinary selector/shape
//! validation and complete finite admission. No new selection semantics.
use super::*;

pub(super) fn supports(
    kernel: &BoundResidentKernel,
    source: &mech_core::Value,
    plan: &SnapshotAccessPlan,
) -> bool {
    let Some(schemas) = kernel.snapshot_schemas() else {
        return false;
    };
    let Some(schema) = schemas.get(plan.output_schema) else {
        return false;
    };
    matches!(schema.body(), SchemaBody::Matrix { element, .. }
        if element.as_ref() == &SchemaBody::String)
        && matches!(source.data(), ValueData::Matrix(_))
        && plan.output_dimensions.is_some()
        && matches!(
            plan.matrix_mode,
            Some(
                ResolvedSelectionMode::LinearGather
                    | ResolvedSelectionMode::Rows
                    | ResolvedSelectionMode::Columns
                    | ResolvedSelectionMode::Rectangle
            )
        )
}

pub(super) fn construct(
    kernel: &BoundResidentKernel,
    source: &mech_core::Value,
    source_schema: &SchemaBody,
    selectors: &[mech_core::Value],
    plan: &SnapshotAccessPlan,
    cost: &SnapshotAccessOutputCost,
    shape: &ShapeInstance,
) -> Result<mech_core::Value, ResidentKernelError> {
    let (rows, columns) = snapshot_access_source_dimensions(source, source_schema, plan)?;
    let ValueData::Matrix(matrix) = source.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let strings = matrix.elements();
    let schemas = kernel
        .shared_snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let constructor = mech_core::snapshot::StringMatrixSnapshotConstructor::bind(
        plan.output_schema,
        shape.clone(),
        Arc::clone(schemas),
    )
    .map_err(|_| ResidentKernelError::InvalidOutput)?;
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(cost.count)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let mut copy = |index: usize| -> Result<(), ResidentKernelError> {
        let source: &str = match strings {
            SequenceView::String(strings) => strings.get(index).map(|text| text.as_ref()),
            SequenceView::Values(values) => values.get(index).and_then(|value| match value {
                ValueData::String(text) => Some(text.as_ref()),
                _ => None,
            }),
            _ => None,
        }
        .ok_or(ResidentKernelError::InvalidInput)?;
        let mut text = String::new();
        text.try_reserve_exact(source.len())
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        text.push_str(source);
        selected.push(text.into_boxed_str());
        Ok(())
    };
    match plan.matrix_mode {
        Some(ResolvedSelectionMode::LinearGather) => {
            let mut linear = |index: usize| copy((index % rows) * columns + index / rows);
            if selectors.is_empty() {
                // Fixed all-elements constructors have no coordinate-array
                // reservation. Iterate directly without creating one.
                for index in 0..cost.count {
                    linear(index)?;
                }
            } else {
                for index in access_indices(&selectors[0], strings.len())? {
                    linear(index)?;
                }
            }
        }
        Some(ResolvedSelectionMode::Rows) => {
            for row in access_indices(&selectors[0], rows)? {
                for column in 0..columns {
                    copy(row * columns + column)?;
                }
            }
        }
        Some(ResolvedSelectionMode::Columns) => {
            let selected_columns = access_indices(&selectors[0], columns)?;
            for row in 0..rows {
                for &column in &selected_columns {
                    copy(row * columns + column)?;
                }
            }
        }
        Some(ResolvedSelectionMode::Rectangle) => {
            let selected_rows = access_indices(&selectors[0], rows)?;
            let selected_columns = access_indices(&selectors[1], columns)?;
            for row in selected_rows {
                for &column in &selected_columns {
                    copy(row * columns + column)?;
                }
            }
        }
        _ => return Err(ResidentKernelError::InvalidInput),
    }
    constructor
        .construct(selected.into_boxed_slice())
        .map_err(|_| ResidentKernelError::InvalidOutput)
}

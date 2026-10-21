//! Non-owning geometry of the maintained linear String-matrix selector.
//! The caller pins both real Values and prepays the complete metadata scan.
use super::*;

#[derive(Clone, Copy, Debug)]
pub(in crate::resident) struct Geometry {
    rows: usize,
    columns: usize,
    pub count: usize,
    pub payload: u64,
}

fn cells(value: &mech_core::Value) -> Result<&[Box<str>], ResidentKernelError> {
    let ValueData::Matrix(matrix) = value.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let SequenceView::String(strings) = matrix.elements() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    Ok(strings)
}

/// The SAME maintained scalar index conversion/error, after a private line
/// source has bound the actual selected scalar-access contract and shape.
pub(in crate::resident) fn scalar_index(
    kernel: &BoundResidentKernel,
    index: u64,
    count: usize,
) -> Result<usize, ResidentKernelError> {
    let plan = kernel
        .retained_state::<SnapshotAccessPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if plan.dense_source.is_some()
        || plan.matrix_mode != Some(ResolvedSelectionMode::LinearScalar)
        || plan.selectors.len() != 1
        || plan.output_dimensions.is_some()
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    access_index(&ValueData::U64(index), count)
}

/// Uses the SAME bound access plan, positional conversion and shape resolver
/// as ordinary snapshot_access. No Matrix result or index vector is built.
pub(in crate::resident) fn prepare(
    kernel: &BoundResidentKernel,
    source: &mech_core::Value,
    selector: &mech_core::Value,
) -> Result<Geometry, ResidentKernelError> {
    let plan = kernel
        .retained_state::<SnapshotAccessPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schemas = kernel
        .snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let schema = source
        .validate_against(schemas)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    // Preserve ordinary selection's qualification/error order before reading
    // source dimensions. The shared geometry entry checks this again for views.
    if plan.dense_source.is_some()
        || plan.matrix_mode != Some(ResolvedSelectionMode::LinearGather)
        || plan.selectors.len() != 1
        || plan.output_dimensions.is_none()
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    let (rows, columns) = snapshot_access_source_dimensions(source, schema.body(), plan)?;
    let strings = cells(source)?;
    prepare_cells(kernel, (rows, columns), strings.len(), selector, |index| {
        strings.get(index).map(|text| text.as_ref())
    })
}

/// SAME selector validation and output-shape solver, with a compiler-private
/// owner-backed String sequence instead of a fabricated canonical Matrix.
/// Callers validate the selected input schema/shape and pin every cell owner.
pub(in crate::resident) fn prepare_cells<'a>(
    kernel: &BoundResidentKernel,
    (rows, columns): (usize, usize),
    count: usize,
    selector: &mech_core::Value,
    mut cell: impl FnMut(usize) -> Option<&'a str>,
) -> Result<Geometry, ResidentKernelError> {
    let plan = kernel
        .retained_state::<SnapshotAccessPlan>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    if plan.dense_source.is_some()
        || plan.matrix_mode != Some(ResolvedSelectionMode::LinearGather)
        || plan.selectors.len() != 1
        || plan.output_dimensions.is_none()
    {
        return Err(ResidentKernelError::InvalidInput);
    }
    if rows.checked_mul(columns) != Some(count) {
        return Err(ResidentKernelError::InvalidShape);
    }
    let ValueData::Matrix(indices) = selector.data() else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let mut geometry = Geometry {
        rows,
        columns,
        count: 0,
        payload: 0,
    };
    sequence_for_each_access_index(indices.elements(), count, |linear| {
        // Canonical selectors are column-major; canonical payload cells are
        // row-major. Bounds above exclude zero-row division for every read.
        let text = cell((linear % rows) * columns + linear / rows)
            .ok_or(ResidentKernelError::InvalidInput)?;
        geometry.count = geometry
            .count
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        geometry.payload = geometry
            .payload
            .checked_add(text.len() as u64)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    let dimensions = if plan.linear_gather_row {
        (1, geometry.count)
    } else {
        (geometry.count, 1)
    };
    if !plan.output_dimensions_dynamic && plan.output_dimensions != Some(dimensions) {
        return Err(ResidentKernelError::InvalidShape);
    }
    // Enforce every declared axis/parameter bound even though no intermediate
    // canonical Matrix exists. This is not cardinality-only retagging.
    resolved_snapshot_output_shape(kernel, dimensions.0, dimensions.1)?;
    Ok(geometry)
}

impl Geometry {
    /// Both owners are immutable and retained by the opaque selected view.
    /// Complete traversal/copy work must be prepaid before calling this leaf.
    pub(in crate::resident) fn for_each(
        self,
        source: &mech_core::Value,
        selector: &mech_core::Value,
        visit: impl FnMut(usize, &str) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        let strings = cells(source)?;
        self.for_each_cells(
            strings.len(),
            selector,
            |index| strings.get(index).map(|text| text.as_ref()),
            visit,
        )
    }

    pub(in crate::resident) fn for_each_cells<'a>(
        self,
        count: usize,
        selector: &mech_core::Value,
        mut cell: impl FnMut(usize) -> Option<&'a str>,
        mut visit: impl FnMut(usize, &str) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        let ValueData::Matrix(indices) = selector.data() else {
            return Err(ResidentKernelError::InvalidInput);
        };
        if self.rows.checked_mul(self.columns) != Some(count)
            || indices.elements().len() != self.count
        {
            return Err(ResidentKernelError::InvalidInput);
        }
        let mut ordinal = 0;
        sequence_for_each_access_index(indices.elements(), count, |linear| {
            visit(
                ordinal,
                cell((linear % self.rows) * self.columns + linear / self.rows)
                    .ok_or(ResidentKernelError::InvalidInput)?,
            )?;
            ordinal += 1;
            Ok(())
        })
    }
}

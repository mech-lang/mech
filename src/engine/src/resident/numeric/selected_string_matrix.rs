//! Non-owning geometry of the maintained linear String-matrix selector.
//! The caller pins the real cell/selector owners or immutable range metadata
//! and prepays the complete metadata scan.
use super::*;

/// Two representations of the same positional selector. A unit range arrives
/// only after its original Range visit has validated cardinality and shape.
#[derive(Clone, Copy)]
pub(in crate::resident) enum Selector<'a> {
    Canonical(&'a mech_core::Value),
    UnitRange { start: u64, count: u64 },
}

#[cfg(test)]
#[path = "selected_string_matrix_tests.rs"]
mod tests;

impl Selector<'_> {
    pub(in crate::resident) fn count(self) -> Result<usize, ResidentKernelError> {
        match self {
            Self::Canonical(value) => match value.data() {
                ValueData::Matrix(matrix) => Ok(matrix.elements().len()),
                _ => Err(ResidentKernelError::InvalidInput),
            },
            Self::UnitRange { count, .. } => {
                usize::try_from(count).map_err(|_| ResidentKernelError::InvalidShape)
            }
        }
    }

    fn for_each_index(
        self,
        upper: usize,
        mut visit: impl FnMut(usize) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        match self {
            Self::Canonical(value) => {
                let ValueData::Matrix(matrix) = value.data() else {
                    return Err(ResidentKernelError::InvalidInput);
                };
                sequence_for_each_access_index(matrix.elements(), upper, visit)
            }
            Self::UnitRange { start, .. } => {
                for offset in 0..self.count()? {
                    // The terminal successor is never evaluated, even when the
                    // final inclusive endpoint is u64::MAX.
                    let index = start
                        .checked_add(offset as u64)
                        .ok_or(ResidentKernelError::InvalidInput)?;
                    // Matrix selectors use the shared sequence conversion,
                    // including its portable limit and InvalidInput mapping;
                    // scalar access has a different out-of-bounds diagnostic.
                    let position =
                        mech_core::canonical_positional_index(&ValueData::U64(index), upper)
                            .map_err(|_| ResidentKernelError::InvalidInput)?;
                    visit(position)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::resident) struct Geometry {
    rows: usize,
    columns: usize,
    pub count: usize,
    pub payload: u64,
    contiguous: Option<(usize, usize)>,
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
    available_scalar_index(index, count)
}

/// The maintained one-based conversion for an already available physical
/// cell. This does not establish a language Matrix shape or execute a visit.
pub(in crate::resident) fn available_scalar_index(
    index: u64,
    available: usize,
) -> Result<usize, ResidentKernelError> {
    access_index(&ValueData::U64(index), available)
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
    prepare_cells(
        kernel,
        (rows, columns),
        strings.len(),
        Selector::Canonical(selector),
        |index| strings.get(index).map(|text| text.as_ref()),
    )
}

/// SAME selector validation and output-shape solver, with a compiler-private
/// owner-backed String sequence instead of a fabricated canonical Matrix.
/// Callers validate the selected input schema/shape and pin every cell owner.
pub(in crate::resident) fn prepare_cells<'a>(
    kernel: &BoundResidentKernel,
    (rows, columns): (usize, usize),
    count: usize,
    selector: Selector<'_>,
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
    selector.count()?;
    let mut geometry = Geometry {
        rows,
        columns,
        count: 0,
        payload: 0,
        contiguous: Some((0, 0)),
    };
    selector.for_each_index(count, |linear| {
        // Canonical selectors are column-major; canonical payload cells are
        // row-major. Bounds above exclude zero-row division for every read.
        let position = (linear % rows) * columns + linear / rows;
        let text = cell(position).ok_or(ResidentKernelError::InvalidInput)?;
        geometry.contiguous = if geometry.count == 0 {
            Some((position, position + 1))
        } else {
            geometry
                .contiguous
                .and_then(|(first, end)| (end == position).then_some((first, position + 1)))
        };
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
    pub(in crate::resident) const fn contiguous_range(self) -> Option<(usize, usize)> {
        self.contiguous
    }
    /// Both owners are immutable and retained by the opaque selected view.
    /// Complete traversal/copy work must be prepaid before calling this leaf.
    pub(in crate::resident) fn for_each(
        self,
        source: &mech_core::Value,
        selector: Selector<'_>,
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
        selector: Selector<'_>,
        mut cell: impl FnMut(usize) -> Option<&'a str>,
        mut visit: impl FnMut(usize, &str) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        if self.rows.checked_mul(self.columns) != Some(count) || selector.count()? != self.count {
            return Err(ResidentKernelError::InvalidInput);
        }
        let mut ordinal = 0;
        selector.for_each_index(count, |linear| {
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

//! Prepaid metadata for actual packed String cells, not materialization authority.
use super::*;
#[cfg(test)]
#[path = "selected_string_footprint_tests.rs"]
mod tests;

pub(super) struct SourceMeasurement<'a> {
    source: &'a mech_core::Value,
    footprint: ValueFootprint,
}

impl SourceMeasurement<'_> {
    pub(super) fn footprint(&self) -> ValueFootprint {
        self.footprint
    }
}

pub(super) fn measure_source<'a>(
    meter: &mut super::super::budget::ResidentBudgetMeter,
    source: &'a mech_core::Value,
    schemas: &mech_core::SchemaTable,
) -> Result<SourceMeasurement<'a>, ResidentKernelError> {
    Ok(SourceMeasurement {
        source,
        footprint: super::super::budget::measure_snapshot_access_source_footprint(
            meter, source, schemas,
        )?,
    })
}

/// Obtained during the existing complete selector validation/count pass.
/// Strictly monotone indices prove uniqueness without an extra walk or buffer.
#[derive(Clone, Copy, Debug)]
pub(super) struct SelectorSummary<'a> {
    value: ResidentValueRef<'a>,
    pub(super) count: usize,
    unique: bool,
}

impl<'a> SelectorSummary<'a> {
    pub(super) fn value(self) -> ResidentValueRef<'a> {
        self.value
    }
}

pub(super) fn summarize(
    value: ResidentValueRef<'_>,
    upper: usize,
) -> Result<SelectorSummary<'_>, ResidentKernelError> {
    let mut count = 0usize;
    let mut previous = None;
    let (mut increasing, mut decreasing) = (true, true);
    selector_for_each_access_index(value, upper, |index| {
        if let Some(previous) = previous {
            increasing &= previous < index;
            decreasing &= previous > index;
        }
        previous = Some(index);
        count = count
            .checked_add(1)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(())
    })?;
    Ok(SelectorSummary {
        value,
        count,
        unique: increasing || decreasing,
    })
}

pub(super) fn try_begin(
    meter: &mut super::super::budget::ResidentBudgetMeter,
    source: &mech_core::Value,
    source_measurement: Option<&SourceMeasurement<'_>>,
    count: usize,
    plan: &SnapshotAccessPlan,
    dimensions: (usize, usize),
    summaries: &[Option<SelectorSummary<'_>>; 2],
) -> Result<bool, ResidentKernelError> {
    let Some(measured) =
        source_measurement.filter(|measured| core::ptr::eq(measured.source, source))
    else {
        return Ok(false);
    };
    if count < 2 {
        return Ok(false);
    }
    let ValueData::Matrix(matrix) = source.data() else {
        return Ok(false);
    };
    let SequenceView::String(strings) = matrix.elements() else {
        return Ok(false);
    };
    let (rows, columns) = dimensions;
    if rows.checked_mul(columns) != Some(strings.len()) {
        return Ok(false);
    }
    let unique = |index: usize| summaries[index].is_some_and(|summary| summary.unique);
    let injective = match plan.matrix_mode {
        Some(ResolvedSelectionMode::LinearGather) => plan.selectors.is_empty() || unique(0),
        Some(ResolvedSelectionMode::Rows | ResolvedSelectionMode::Columns) => unique(0),
        Some(ResolvedSelectionMode::Rectangle) => unique(0) && unique(1),
        _ => false,
    };
    if !injective {
        return Ok(false);
    }
    let selected_count = |index: usize| summaries[index].map(|summary| summary.count);
    let expected = match plan.matrix_mode {
        Some(ResolvedSelectionMode::LinearGather) if plan.selectors.is_empty() => {
            Some(strings.len())
        }
        Some(ResolvedSelectionMode::LinearGather) => selected_count(0),
        Some(ResolvedSelectionMode::Rows) => selected_count(0).and_then(|n| n.checked_mul(columns)),
        Some(ResolvedSelectionMode::Columns) => selected_count(0).and_then(|n| rows.checked_mul(n)),
        Some(ResolvedSelectionMode::Rectangle) => selected_count(0)
            .zip(selected_count(1))
            .and_then(|(r, c)| r.checked_mul(c)),
        _ => None,
    };
    if expected != Some(count) {
        return Ok(false);
    }
    let Some(nodes) = u64::try_from(count).ok() else {
        return Ok(false);
    };
    // The caller just measured this SAME immutable source, before selecting.
    // No-repeat selectors imply selected payload <= its full retained bytes.
    // Each selected scalar additionally retains an inline ValueData header.
    // Preview the entire upper bound without debiting it or reading cells.
    let Some(compute) = nodes
        .checked_mul(64 + core::mem::size_of::<ValueData>() as u64)
        .and_then(|bytes| bytes.checked_add(measured.footprint.retained_bytes))
    else {
        return Ok(false);
    };
    if !meter.can_charge_compute_and_nodes(compute, nodes) {
        return Ok(false);
    }
    // Fund every metadata inspection BEFORE the walk. Exact retained/copy
    // work is charged after measuring, before any allocation/publication.
    // No speculative upper-bound debit or refund occurs.
    meter.try_charge_borrowed_metadata_scan(nodes, true)
}

pub(super) fn add_cell(
    total: &mut ValueFootprint,
    element: &SchemaBody,
    values: SequenceView<'_>,
    index: usize,
) -> Result<(), ResidentKernelError> {
    let footprint = canonical_sequence_element_retained_footprint(element, values, index)
        .map_err(|_| ResidentKernelError::InvalidInput)?;
    checked_footprint_add(total, footprint)
}

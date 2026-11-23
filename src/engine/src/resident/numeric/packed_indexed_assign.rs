//! Packed construction after the ordinary indexed-assignment checks/admission.
use super::*;

#[cfg(test)]
#[path = "packed_indexed_assign_tests.rs"]
mod tests;

/// The caller has already validated one selected destination and one source
/// element. A packed unsigned replacement cannot change the result footprint.
/// Prepay the SAME planning work only when the existing meter proves it fits;
/// a miss retains the ordinary traversal and its first-denied work prefix.
pub(super) fn try_project_single_source(
    meter: &mut super::super::budget::ResidentBudgetMeter,
    element: &SchemaBody,
    current: SequenceView<'_>,
    source_schema: &SchemaBody,
    source: &ValueData,
    scan_work: u64,
) -> Result<bool, ResidentKernelError> {
    macro_rules! source_kind {
        ($kind:ident) => {
                match (source_schema, source) {
                    (body, ValueData::$kind(_)) if body == element => true,
                    (SchemaBody::Matrix { element: source_element, .. }, ValueData::Matrix(matrix))
                        if source_element.as_ref() == element
                            && matches!(matrix.elements(), SequenceView::$kind(words) if words.len() == 1) => false,
                    _ => return Ok(false),
                }
        };
    }
    let scalar = match (element, current) {
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), SequenceView::U8(_)) => source_kind!(U8),
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), SequenceView::U16(_)) => source_kind!(U16),
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), SequenceView::U32(_)) => source_kind!(U32),
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), SequenceView::U64(_)) => source_kind!(U64),
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), SequenceView::U128(_)) => {
            source_kind!(U128)
        }
        _ => return Ok(false),
    };
    let source_work = if scalar {
        // Bounded primitive metadata only: use the maintained finalization
        // visitor to derive its exact fee without spending the live ledger.
        // Unsupported/non-compute charges decline this optimization.
        let mut probe =
            super::super::budget::ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
        if snapshot_element_finalization_work(&mut probe, source_schema, source, 0) != Ok(0) {
            return Ok(false);
        }
        let cost = probe.estimate();
        if cost
            != super::super::budget::resident_cost! {
                compute_work: cost.compute_work(),
                ..super::super::budget::KernelCostEstimate::default()
            }
        {
            return Ok(false);
        }
        cost.compute_work()
    } else {
        // Packed sequence elements already have zero finalization work and
        // the actual one-cell source above establishes index-zero validity.
        0
    };
    let Some(work) = scan_work.checked_add(source_work) else {
        return Ok(false);
    };
    if !meter.can_charge_compute_and_nodes(work, 0) {
        return Ok(false);
    }
    meter.charge_compute_work(work)?;
    Ok(true)
}

pub(super) struct Plan<'a> {
    pub positions: ValidatedPositions<'a>,
    pub routing: ResolvedSourceRouting,
    pub rows: usize,
    pub columns: usize,
    pub source_rows: usize,
    pub source_columns: usize,
}

pub(in crate::resident) fn canonical_index(
    position: usize,
    rows: usize,
    columns: usize,
) -> Result<usize, ResidentKernelError> {
    position
        .checked_rem(rows)
        .and_then(|row| row.checked_mul(columns))
        .and_then(|offset| offset.checked_add(position.checked_div(rows)?))
        .ok_or(ResidentKernelError::InvalidShape)
}

pub(super) fn construct(
    kernel: &BoundResidentKernel,
    current: &mech_core::Value,
    source: &mech_core::Value,
    plan: Plan<'_>,
) -> Result<mech_core::Value, ResidentKernelError> {
    let ValueData::Matrix(matrix) = current.data() else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let metadata = kernel
        .snapshot_output()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    let schemas = kernel
        .shared_snapshot_schemas()
        .ok_or(ResidentKernelError::InvalidOutput)?;
    macro_rules! update {
        ($words:expr, $kind:ident, $constructor:ident) => {{
            let mut next = $words.to_vec();
            plan.positions.try_for_each(|ordinal, position| {
                let source_index = match plan.routing {
                    ResolvedSourceRouting::ScalarBroadcast => 0,
                    ResolvedSourceRouting::Positional => {
                        canonical_index(position, plan.source_rows, plan.source_columns)?
                    }
                    ResolvedSourceRouting::CompactSelectionOrder => ordinal,
                };
                let incoming = match source.data() {
                    ValueData::$kind(value) if source_index == 0 => *value,
                    ValueData::Matrix(matrix) => {
                        let SequenceView::$kind(words) = matrix.elements() else {
                            return Err(ResidentKernelError::InvalidInput);
                        };
                        *words
                            .get(source_index)
                            .ok_or(ResidentKernelError::InvalidShape)?
                    }
                    _ => return Err(ResidentKernelError::InvalidInput),
                };
                let destination = canonical_index(position, plan.rows, plan.columns)?;
                *next
                    .get_mut(destination)
                    .ok_or(ResidentKernelError::InvalidShape)? = incoming;
                Ok(())
            })?;
            mech_core::Value::$constructor(
                metadata.schema,
                current.shape().clone(),
                next.into_boxed_slice(),
                Arc::clone(schemas),
            )
            .map_err(|_| ResidentKernelError::InvalidOutput)
        }};
    }
    match matrix.elements() {
        SequenceView::U8(words) => update!(words, U8, from_u8_matrix),
        SequenceView::U16(words) => update!(words, U16, from_u16_matrix),
        SequenceView::U32(words) => update!(words, U32, from_u32_matrix),
        SequenceView::U64(words) => update!(words, U64, from_u64_matrix),
        SequenceView::U128(words) => update!(words, U128, from_u128_matrix),
        _ => Err(ResidentKernelError::InvalidOutput),
    }
}

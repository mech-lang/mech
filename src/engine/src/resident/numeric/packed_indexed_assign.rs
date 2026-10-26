//! Packed construction after the ordinary indexed-assignment checks/admission.
use super::*;

pub(super) struct Plan<'a> {
    pub positions: ValidatedPositions<'a>,
    pub routing: ResolvedSourceRouting,
    pub rows: usize,
    pub columns: usize,
    pub source_rows: usize,
    pub source_columns: usize,
}

fn canonical_index(
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

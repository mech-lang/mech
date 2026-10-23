//! Same maintained range cardinality, addition and admission; packed output.
use super::*;

fn packed<T: Copy>(
    first: T,
    step: T,
    count: usize,
    add: impl Fn(T, T) -> Option<T>,
) -> Result<Box<[T]>, ResidentKernelError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    let mut current = first;
    for index in 0..count {
        values.push(current);
        // Do not evaluate the unused successor of a terminal value.
        if index + 1 < count {
            current = add(current, step).ok_or(ResidentKernelError::Arithmetic)?;
        }
    }
    Ok(values.into_boxed_slice())
}

pub(super) fn construct(
    kernel: &BoundResidentKernel,
    values: &[ValueDataDraft],
    element: &SchemaBody,
    inclusive: bool,
    incremented: bool,
    expected_count: usize,
) -> Result<mech_core::Value, ResidentKernelError> {
    let count = canonical_range_draft_cardinality(values, inclusive, incremented)
        .filter(|count| *count != 0 && *count == expected_count)
        .ok_or(ResidentKernelError::InvalidInput)?;
    let step = if incremented {
        values[1].clone()
    } else {
        range_one(element)?
    };
    macro_rules! materialize {
        ($variant:ident, $constructor:ident, $add:expr) => {
            if let (ValueDataDraft::$variant(first), ValueDataDraft::$variant(step)) =
                (&values[0], &step)
            {
                // The caller has already performed the ORIGINAL whole-range
                // preflight, including draft layout and conservative reserves.
                // Keep arithmetic before output shape/finalization failures.
                let elements = packed(*first, *step, count, $add)?;
                let shape = resolved_snapshot_output_shape(kernel, 1, count)?;
                let metadata = kernel
                    .snapshot_output()
                    .ok_or(ResidentKernelError::InvalidOutput)?;
                let schemas = kernel
                    .shared_snapshot_schemas()
                    .ok_or(ResidentKernelError::InvalidOutput)?;
                let next = mech_core::Value::$constructor(
                    metadata.schema,
                    shape,
                    elements,
                    Arc::clone(schemas),
                )
                .map_err(|_| ResidentKernelError::InvalidOutput)?;
                if next.schema_key() != metadata.schema_key {
                    return Err(ResidentKernelError::InvalidOutput);
                }
                return Ok(next);
            }
        };
    }
    materialize!(U8, from_u8_matrix, u8::checked_add);
    materialize!(U16, from_u16_matrix, u16::checked_add);
    materialize!(U32, from_u32_matrix, u32::checked_add);
    materialize!(U64, from_u64_matrix, u64::checked_add);
    materialize!(U128, from_u128_matrix, u128::checked_add);
    materialize!(I8, from_i8_matrix, i8::checked_add);
    materialize!(I16, from_i16_matrix, i16::checked_add);
    materialize!(I32, from_i32_matrix, i32::checked_add);
    materialize!(I64, from_i64_matrix, i64::checked_add);
    materialize!(I128, from_i128_matrix, i128::checked_add);
    materialize!(Index, from_index_matrix, u64::checked_add);
    // Repeated rounded addition, NOT first + index * step: preserve every bit
    // of the ordinary numeric_add path, including signed zero and infinity.
    materialize!(F32, from_f32_matrix, |a: F32Bits, b: F32Bits| {
        Some(F32Bits::from_f32(a.to_f32() + b.to_f32()))
    });
    materialize!(F64, from_f64_matrix, |a: F64Bits, b: F64Bits| {
        Some(F64Bits::from_f64(a.to_f64() + b.to_f64()))
    });
    Err(ResidentKernelError::InvalidInput)
}

#[cfg(test)]
#[path = "compact_range_tests.rs"]
mod tests;

use super::*;

#[test]
fn packed_ranges_match_the_retained_draft_oracle_for_every_numeric_kind() {
    macro_rules! check {
        ($variant:ident, $body:expr, $values:expr, $add:expr) => {{
            let values = $values;
            for inclusive in [false, true] {
                for incremented in [false, true] {
                    let inputs = if incremented {
                        vec![
                            ValueDataDraft::$variant(values[0]),
                            ValueDataDraft::$variant(values[1]),
                            ValueDataDraft::$variant(values[2]),
                        ]
                    } else {
                        vec![
                            ValueDataDraft::$variant(values[0]),
                            ValueDataDraft::$variant(values[2]),
                        ]
                    };
                    let Some(count) =
                        canonical_range_draft_cardinality(&inputs, inclusive, incremented)
                            .filter(|n| *n > 0)
                    else {
                        continue;
                    };
                    let step = if incremented {
                        inputs[1].clone()
                    } else {
                        range_one(&$body).unwrap()
                    };
                    let ValueDataDraft::$variant(step) = step else {
                        panic!("same range kind")
                    };
                    let actual = packed(values[0], step, count, $add).map(|cells| {
                        cells
                            .iter()
                            .copied()
                            .map(ValueDataDraft::$variant)
                            .collect::<Vec<_>>()
                    });
                    let expected = canonical_range_draft_values(
                        &inputs,
                        &$body,
                        inclusive,
                        incremented,
                        count,
                    );
                    assert_eq!(
                        actual, expected,
                        "inclusive={inclusive}, incremented={incremented}"
                    );
                }
            }
        }};
    }
    macro_rules! integer {
        ($variant:ident, $kind:ident, $width:ident, $ty:ty) => {{
            for values in [[1, 2, 9], [<$ty>::MAX - 6, 2, <$ty>::MAX]] {
                check!(
                    $variant,
                    SchemaBody::$kind(IntegerWidth::$width),
                    values,
                    <$ty>::checked_add
                );
            }
        }};
    }
    integer!(U8, UnsignedInteger, W8, u8);
    integer!(U16, UnsignedInteger, W16, u16);
    integer!(U32, UnsignedInteger, W32, u32);
    integer!(U64, UnsignedInteger, W64, u64);
    integer!(U128, UnsignedInteger, W128, u128);
    integer!(I8, SignedInteger, W8, i8);
    integer!(I16, SignedInteger, W16, i16);
    integer!(I32, SignedInteger, W32, i32);
    integer!(I64, SignedInteger, W64, i64);
    integer!(I128, SignedInteger, W128, i128);
    check!(
        Index,
        SchemaBody::Index,
        [u64::MAX - 6, 2, u64::MAX],
        u64::checked_add
    );
    check!(
        I128,
        SchemaBody::SignedInteger(IntegerWidth::W128),
        [i128::MIN + 6, -2, i128::MIN],
        i128::checked_add
    );
    check!(
        U128,
        SchemaBody::UnsignedInteger(IntegerWidth::W128),
        [1u128 << 100, 3, (1u128 << 100) + 11],
        u128::checked_add
    );
    for values in [[0.5, -0.1, 0.1], [-0.0, 0.1, 0.4], [1.0, 0.25, 2.0]] {
        check!(
            F32,
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W32),
            values.map(F32Bits::from_f32),
            |a: F32Bits, b: F32Bits| Some(F32Bits::from_f32(a.to_f32() + b.to_f32()))
        );
    }
    for values in [[0.5, -0.1, 0.1], [-0.0, 0.1, 0.4], [1.0, 0.25, 2.0]] {
        check!(
            F64,
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
            values.map(F64Bits::from_f64),
            |a: F64Bits, b: F64Bits| Some(F64Bits::from_f64(a.to_f64() + b.to_f64()))
        );
    }
}

#[test]
fn packed_range_does_not_add_after_the_last_cell_and_rejects_real_overflow() {
    assert_eq!(
        packed(u64::MAX, 1, 1, u64::checked_add).unwrap().as_ref(),
        [u64::MAX]
    );
    assert_eq!(
        packed(i128::MIN, -1, 1, i128::checked_add)
            .unwrap()
            .as_ref(),
        [i128::MIN]
    );
    assert_eq!(
        packed(u64::MAX, 1, 2, u64::checked_add),
        Err(ResidentKernelError::Arithmetic)
    );
    assert_eq!(
        packed(i128::MIN, -1, 2, i128::checked_add),
        Err(ResidentKernelError::Arithmetic)
    );
    assert_eq!(
        packed(0u64, 1, usize::MAX, u64::checked_add),
        Err(ResidentKernelError::InvalidShape)
    );
}

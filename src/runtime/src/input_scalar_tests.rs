use super::*;
use crate::RuntimeHostInputValue;
use mech_core::ValueCell;

fn equivalent(actual: Value, expected: Value) {
    assert_eq!(actual.schema_key(), expected.schema_key());
    assert_eq!(actual.shape(), expected.shape());
    assert_eq!(
        actual
            .canonical_snapshot_bytes(&actual.schemas().unwrap())
            .unwrap(),
        expected
            .canonical_snapshot_bytes(&expected.schemas().unwrap())
            .unwrap()
    );
    assert_eq!(
        actual.memory_budget_required_bytes().unwrap(),
        expected.memory_budget_required_bytes().unwrap()
    );
    actual.validate_against(&actual.schemas().unwrap()).unwrap();
}

#[test]
fn unit_and_index_inputs_match_mutable_cell_snapshots() {
    equivalent(
        RuntimeHostInputValue::Unit.into_value().unwrap(),
        ValueCell::unit().snapshot().unwrap(),
    );
    for value in [1, 2, usize::MAX] {
        equivalent(
            RuntimeHostInputValue::Index(value).into_value().unwrap(),
            ValueCell::from_exact(value).unwrap().snapshot().unwrap(),
        );
    }
    assert!(
        RuntimeHostInputValue::Index(0)
            .into_value()
            .unwrap_err()
            .display_message()
            .contains("index host input values must be in 1..=max")
    );
}

#[allow(unused_macros, reason = "minimal profiles test only unit and index")]
macro_rules! scalar_test {
    ($feature:literal, $name:ident, $variant:ident, $values:expr) => {
        #[cfg(feature = $feature)]
        #[test]
        fn $name() {
            for value in $values {
                equivalent(
                    RuntimeHostInputValue::$variant(value).into_value().unwrap(),
                    ValueCell::from_exact(value).unwrap().snapshot().unwrap(),
                );
            }
        }
    };
}

scalar_test!("bool", bool_inputs, Bool, [false, true]);
scalar_test!("u8", u8_inputs, U8, [u8::MIN, 0, 1, u8::MAX]);
scalar_test!("u16", u16_inputs, U16, [u16::MIN, 0, 1, u16::MAX]);
scalar_test!("u32", u32_inputs, U32, [u32::MIN, 0, 1, u32::MAX]);
scalar_test!("u64", u64_inputs, U64, [u64::MIN, 0, 1, u64::MAX]);
scalar_test!("u128", u128_inputs, U128, [u128::MIN, 0, 1, u128::MAX]);
scalar_test!("i8", i8_inputs, I8, [i8::MIN, 0, 1, i8::MAX]);
scalar_test!("i16", i16_inputs, I16, [i16::MIN, 0, 1, i16::MAX]);
scalar_test!("i32", i32_inputs, I32, [i32::MIN, 0, 1, i32::MAX]);
scalar_test!("i64", i64_inputs, I64, [i64::MIN, 0, 1, i64::MAX]);
scalar_test!("i128", i128_inputs, I128, [i128::MIN, 0, 1, i128::MAX]);
scalar_test!(
    "f32",
    f32_inputs,
    F32,
    [
        -0.0_f32,
        0.0,
        -1.5,
        f32::MIN,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234)
    ]
);
scalar_test!(
    "f64",
    f64_inputs,
    F64,
    [
        -0.0_f64,
        0.0,
        -1.5,
        f64::MIN,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7ff8_1234_5678_9abc)
    ]
);

#[test]
fn unavailable_scalar_features_still_reject_inputs() {
    #[allow(
        unused_macros,
        reason = "full scalar profiles have no disabled variants"
    )]
    macro_rules! reject {
        ($feature:literal, $variant:ident, $value:expr) => {
            #[cfg(not(feature = $feature))]
            assert!(
                RuntimeHostInputValue::$variant($value)
                    .into_value()
                    .unwrap_err()
                    .display_message()
                    .contains("host input values require")
            );
        };
    }
    reject!("bool", Bool, true);
    reject!("string", String, String::new());
    reject!("u8", U8, 1 as u8);
    reject!("u16", U16, 1 as u16);
    reject!("u32", U32, 1 as u32);
    reject!("u64", U64, 1 as u64);
    reject!("u128", U128, 1 as u128);
    reject!("i8", I8, 1 as i8);
    reject!("i16", I16, 1 as i16);
    reject!("i32", I32, 1 as i32);
    reject!("i64", I64, 1 as i64);
    reject!("i128", I128, 1 as i128);
    reject!("f32", F32, 1 as f32);
    reject!("f64", F64, 1 as f64);
}

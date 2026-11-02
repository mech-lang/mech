#![cfg(feature = "string")]

use mech_core::{ValueCell, ValueData};
use mech_runtime::RuntimeHostInputValue;

#[test]
fn string_input_moves_exact_capacity_backing_into_canonical_snapshot() {
    for text in ["ascii".to_owned(), "a\0é\r\n🧑‍🚀".repeat(4096)] {
        let input = text.clone().into_boxed_str().into_string();
        assert_eq!(input.len(), input.capacity());
        let backing = input.as_ptr();
        let value = RuntimeHostInputValue::String(input).into_value().unwrap();
        let ValueData::String(actual) = value.data() else {
            panic!("canonical String");
        };
        assert_eq!(actual.as_ref(), text);
        assert_eq!(
            actual.as_ptr(),
            backing,
            "detached input must not be cloned through a mutable cell"
        );
    }
}

#[test]
fn string_input_preserves_canonical_type_shape_and_bytes() {
    for text in ["", "ascii", "a\0é\r\n🧑‍🚀", "e\u{301}"] {
        let expected = ValueCell::from_exact(text.to_owned())
            .unwrap()
            .snapshot()
            .unwrap();
        for spare in [0, 4096] {
            let mut input = String::with_capacity(text.len() + spare);
            input.push_str(text);
            let value = RuntimeHostInputValue::String(input).into_value().unwrap();
            assert_eq!(value.schema_key(), expected.schema_key());
            assert_eq!(value.shape(), expected.shape());
            assert_eq!(
                value
                    .canonical_snapshot_bytes(&value.schemas().unwrap())
                    .unwrap(),
                expected
                    .canonical_snapshot_bytes(&expected.schemas().unwrap())
                    .unwrap()
            );
            assert_eq!(
                value.memory_budget_required_bytes().unwrap(),
                expected.memory_budget_required_bytes().unwrap()
            );
        }
    }
}

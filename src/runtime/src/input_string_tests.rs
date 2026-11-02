use super::*;
use mech_core::ValueData;

#[test]
fn acquired_utf8_backing_moves_into_the_same_canonical_string_finalizer() {
    for text in [
        "".to_owned(),
        "a\0é\r\n🧑‍🚀".to_owned(),
        "acgt".repeat(16_384),
    ] {
        let mut bytes = text.as_bytes().to_vec();
        let pointer = bytes.as_ptr();
        assert_eq!(bytes.capacity(), bytes.len());
        let output = snapshot_bytes(&mut bytes).unwrap();
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), 0);
        assert!(
            matches!(output.data(),ValueData::String(value) if value.as_ref() == text && value.as_ptr() == pointer)
        );
        let ordinary = snapshot(text).unwrap();
        assert!(matches!((output.data(), ordinary.data()),
            (ValueData::String(left), ValueData::String(right)) if left == right));
        assert_eq!(output.schema_key(), ordinary.schema_key());
        assert!(!output.shares_frozen_storage(&ordinary));
    }
}

#[test]
fn invalid_utf8_restores_exact_bytes_before_any_canonical_preparation() {
    for input in [
        vec![0xff],
        vec![b'a', 0, 0xf0, 0x9f],
        vec![b'a', 0x80, b'b'],
    ] {
        let mut bytes = Vec::with_capacity(4096);
        bytes.extend_from_slice(&input);
        let pointer = bytes.as_ptr();
        let capacity = bytes.capacity();
        let expected = core::str::from_utf8(&input).unwrap_err();
        let error = snapshot_bytes_with_prepare(&mut bytes, |_| panic!("invalid text prepared"))
            .unwrap_err();
        assert!(matches!(error, RuntimeHostStringError::InvalidUtf8(actual) if actual == expected));
        assert_eq!(bytes, input);
        assert_eq!(bytes.as_ptr(), pointer);
        assert_eq!(bytes.capacity(), capacity);
    }
}

#[test]
fn preparation_and_finish_errors_both_return_the_untouched_input_backing() {
    for finish_failure in [false, true] {
        let mut bytes = Vec::with_capacity(4096);
        bytes.extend_from_slice("replay é\0".as_bytes());
        let pointer = bytes.as_ptr();
        let capacity = bytes.capacity();
        let error = snapshot_bytes_with_prepare(&mut bytes, |length| {
            if finish_failure {
                prepare(length + 1)
            } else {
                Err(super::super::input_error(
                    "PreparationDenied",
                    "test canonical denial",
                ))
            }
        })
        .unwrap_err();
        assert!(matches!(error, RuntimeHostStringError::Snapshot(_)));
        assert_eq!(bytes, "replay é\0".as_bytes());
        assert_eq!(bytes.as_ptr(), pointer);
        assert_eq!(bytes.capacity(), capacity);
        let retried = snapshot_bytes(&mut bytes).unwrap();
        assert!(
            matches!(retried.data(),ValueData::String(value) if value.as_ref() == "replay é\0")
        );
        assert!(bytes.is_empty());
    }
}

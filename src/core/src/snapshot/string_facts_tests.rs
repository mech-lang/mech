//! Immutable representation facts follow data ownership, never byte equality.
use super::*;
use crate::{ManagedMemoryBudget, SchemaDraft, SchemaTableBuilder};

fn scalar(body: SchemaBody, data: ValueDataDraft) -> Value {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(SchemaDraft {
        body, dimension_parameters: Box::new([]),
    }.finalize().unwrap()).unwrap();
    let built = builder.finish().unwrap();
    ValueDraft {
        schema: built.resolve(handle).unwrap(), shape_values: Box::new([]), data,
    }.finalize(&SnapshotValidationContext::with_shared_schemas(&Arc::new(built.table))).unwrap()
}

fn string(text: &str) -> Value {
    scalar(SchemaBody::String, ValueDataDraft::String(text.into()))
}

fn admit(value: Value, memory: &ManagedMemoryBudget) -> Value {
    let mut reservation = memory.reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap()).unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

#[test]
fn string_facts_are_lazy_exact_and_shared_only_by_the_immutable_data_owner() {
    for text in ["", "a\0b\r", "\r\n", "\n\r", "é", "é\r\n", "a\u{301}", "🧑‍🚀"] {
        let value = string(text);
        let expected = text.is_ascii() && !text.as_bytes().windows(2).any(|p| p == b"\r\n");
        assert!(value.root.data.string_ascii_without_crlf.get().is_none());
        let cloned = value.clone();
        assert_eq!(cloned.string_ascii_without_crlf(), Some(expected));
        assert_eq!(value.root.data.string_ascii_without_crlf.get(), Some(&expected));
        let equal = string(text);
        assert!(!value.shares_frozen_storage(&equal));
        assert!(equal.root.data.string_ascii_without_crlf.get().is_none());
        assert_eq!(equal.string_ascii_without_crlf(), Some(expected));
    }
    let non_string = scalar(SchemaBody::Bool, ValueDataDraft::Bool(true));
    assert_eq!(non_string.string_ascii_without_crlf(), None);
    assert!(non_string.root.data.string_ascii_without_crlf.get().is_none());
}

#[test]
fn string_facts_cover_every_ascii_byte_and_crlf_across_scan_boundaries() {
    for length in [0, 1, 2, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 257] {
        for byte in 0u8..=127 {
            let text = String::from_utf8(vec![byte; length]).unwrap();
            assert_eq!(string(&text).string_ascii_without_crlf(), Some(true));
        }
        for index in 0..length.saturating_sub(1) {
            let mut bytes = vec![b'A'; length];
            bytes[index..index + 2].copy_from_slice(b"\r\n");
            let text = String::from_utf8(bytes).unwrap();
            assert_eq!(string(&text).string_ascii_without_crlf(), Some(false));
        }
    }
}

#[test]
fn string_facts_share_across_budget_imports_without_new_allocation_or_authority() {
    let raw = string(&"Ab\0\r".repeat(4096));
    let first = ManagedMemoryBudget::new(1_000_000);
    let second = ManagedMemoryBudget::new(1_000_000);
    let a = admit(raw.clone(), &first);
    let b = admit(raw.clone(), &second);
    let before = (first.used_bytes(), second.used_bytes());
    assert!(before.0 > 0 && before.1 > 0);
    assert_eq!(a.string_ascii_without_crlf(), Some(true));
    assert_eq!(b.root.data.string_ascii_without_crlf.get(), Some(&true));
    assert_eq!(b.string_ascii_without_crlf(), Some(true));
    assert_eq!((first.used_bytes(), second.used_bytes()), before);
    assert_eq!(raw.string_ascii_without_crlf(), Some(true));
    drop((a, b, raw));
    assert_eq!((first.used_bytes(), second.used_bytes()), (0, 0));
}

#[test]
fn string_facts_retire_before_owned_bytes_can_change_and_refreeze() {
    let memory = ManagedMemoryBudget::new(1_000_000);
    let original = admit(string("AB"), &memory);
    assert_eq!(original.string_ascii_without_crlf(), Some(true));
    let schema = original.schema();
    let schemas = original.schemas().unwrap();
    let identity = original.frozen_storage_identity();
    let StringOwnershipTransfer::Owned(owned) = original.try_into_owned_string(&memory).unwrap()
    else { panic!("representation metadata must not pin a strong owner"); };
    let (mut text, mut reservation) = owned.into_parts();
    text.replace_range(.., "é");
    let output = ValueDraft {
        schema, shape_values: Box::new([]), data: ValueDataDraft::String(text),
    }.finalize(&SnapshotValidationContext::with_shared_schemas(&schemas)).unwrap()
        .into_memory_budget(&mut reservation).unwrap();
    assert!(!identity.matches(&output));
    assert!(output.root.data.string_ascii_without_crlf.get().is_none());
    assert_eq!(output.string_ascii_without_crlf(), Some(false));
    drop((output, reservation));
    assert_eq!(memory.used_bytes(), 0);
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
#[test]
fn string_facts_are_shared_safely_by_concurrent_readers() {
    for text in ["AGCT".repeat(16_384), "AG\r\n".repeat(16_384), "é🧑‍🚀".repeat(4096)] {
        let value = string(&text);
        let expected = text.is_ascii() && !text.as_bytes().windows(2).any(|p| p == b"\r\n");
        assert!(value.root.data.string_ascii_without_crlf.get().is_none());
        std::thread::scope(|scope| {
            let mut readers = Vec::new();
            for _ in 0..8 {
                let value = &value;
                readers.push(scope.spawn(move || value.string_ascii_without_crlf()));
            }
            for reader in readers { assert_eq!(reader.join().unwrap(), Some(expected)); }
        });
        assert_eq!(value.root.data.string_ascii_without_crlf.get(), Some(&expected));
    }
}

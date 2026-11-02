use crate::native_artifact::memory::*;

#[test]
fn typed_transport_admission_includes_exact_initialization_metadata() {
    for count in [0, 1, 7, 8, 9, 63, 64, 65, 128, 4097] {
        let memory = ManagedMemoryBudget::new(64 << 20);
        let bytes = NativeWordStorage::admission_bytes(count).unwrap();
        let words = NativeWordStorage::new(count, 64 << 20, Some(&memory)).unwrap();
        assert_eq!(memory.used_bytes(), bytes, "word count={count}");
        drop(words);
        assert_eq!(memory.used_bytes(), 0);
        let bytes = NativeTypedStorage::<Option<mech_core::Value>>::admission_bytes(count).unwrap();
        let values = NativeTypedStorage::new_values(count, 64 << 20, &memory).unwrap();
        assert_eq!(memory.used_bytes(), bytes, "value count={count}");
        drop(values);
        assert_eq!(memory.used_bytes(), 0);
        if bytes > 0 {
            let tight = memory.with_limits(bytes - 1, u64::MAX);
            assert!(NativeTypedStorage::new_values(count, 64 << 20, &tight).is_err());
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

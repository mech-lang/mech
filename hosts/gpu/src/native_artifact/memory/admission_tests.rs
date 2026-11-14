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

#[test]
fn compact_word_transport_keeps_full_admission_and_transport_floors() {
    for (funded, physical) in [
        (0, 0),
        (1, 0),
        (1, 1),
        (9, 0),
        (9, 7),
        (3127, 1),
        (4097, 970),
    ] {
        let bytes = NativeWordStorage::admission_bytes(funded).unwrap();
        let raw_bytes = (funded * core::mem::size_of::<u64>()) as u64;
        let memory = ManagedMemoryBudget::new(64 << 20);
        let mut words =
            NativeWordStorage::new_compact(funded, physical, 64 << 20, &memory).unwrap();
        assert_eq!(words.values().len(), physical);
        assert!(words.values().iter().all(|word| *word == 0));
        words.values_mut().fill(41);
        assert_eq!(
            memory.used_bytes(),
            bytes,
            "funded={funded} physical={physical}"
        );
        drop(words);
        assert_eq!(memory.used_bytes(), 0);
        let exact = memory.with_limits(bytes, u64::MAX);
        let words = NativeWordStorage::new_compact(funded, physical, 64 << 20, &exact).unwrap();
        assert_eq!(memory.used_bytes(), bytes);
        drop(words);
        if bytes > 0 {
            let short = memory.with_limits(bytes - 1, u64::MAX);
            assert!(NativeWordStorage::new_compact(funded, physical, 64 << 20, &short).is_err());
            assert_eq!(memory.used_bytes(), 0);
        }
        if raw_bytes > 0 {
            assert!(matches!(
                NativeWordStorage::new_compact(funded, physical, raw_bytes - 1, &memory),
                Err(NativeExecutionError::TransportLimit {required, limit})
                    if required == raw_bytes && limit == raw_bytes - 1
            ));
            assert_eq!(memory.used_bytes(), 0);
        }
        assert!(NativeWordStorage::new_compact(funded, funded + 1, 64 << 20, &memory).is_err());
        assert_eq!(memory.used_bytes(), 0);
    }
}

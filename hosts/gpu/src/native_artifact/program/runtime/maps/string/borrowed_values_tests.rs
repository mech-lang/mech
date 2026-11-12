use super::*;
use mech_core::{
    CardinalitySpec, IntegerWidth, MemoryBudgetLimits, SchemaBody, SchemaDraft, SchemaId,
    SchemaTableBuilder,
};
use std::sync::Arc;

fn schema() -> (Arc<SchemaTable>, SchemaId) {
    let mut b = SchemaTableBuilder::new();
    let h = b
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Map {
                    key: Box::new(SchemaBody::String),
                    value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let b = b.finish().unwrap();
    let id = b.resolve(h).unwrap();
    let (t, _) = b.into_parts();
    (Arc::new(t), id)
}
fn limits() -> Limits {
    Limits {
        entries: 4096,
        retained_bytes: 16 << 20,
        temporary_bytes: 32 << 20,
    }
}
fn source(
    texts: &[String],
    m: &ManagedMemoryBudget,
    w: &NativeLibraryWorkBudget,
    schemas: &Arc<SchemaTable>,
    id: SchemaId,
) -> Value {
    source_with_base(texts, m, w, schemas, id, 19)
}
fn source_with_base(
    texts: &[String],
    m: &ManagedMemoryBudget,
    w: &NativeLibraryWorkBudget,
    schemas: &Arc<SchemaTable>,
    id: SchemaId,
    base: u64,
) -> Value {
    let mut map = OwnedStringMap::new_with_memory(w.clone(), limits(), m.clone()).unwrap();
    for (i, text) in texts.iter().enumerate() {
        map.insert(text, base + i as u64).unwrap();
    }
    map.publish_result(
        schemas,
        id,
        &[],
        PublicationGeometry {
            live_bytes: m.used_bytes(),
            live_nodes: m.retained_nodes(),
        },
        MemoryBudgetLimits {
            max_output_elements: Some(1),
            max_output_bytes: Some(16 << 20),
            max_cloned_bytes: Some(16 << 20),
            max_temporary_bytes: Some(32 << 20),
            max_retained_nodes: Some(100_000),
            max_compute_work: Some(10_000_000_000),
            max_comparison_work: Some(1_000_000_000),
            ..Default::default()
        },
    )
    .unwrap()
}
#[test]
fn borrowed_dictionary_values_equal_keys_do_not_reuse_old_histogram_values() {
    let m = ManagedMemoryBudget::new(64 << 20);
    let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
    let (schemas, id) = schema();
    let texts = vec!["a".into(), "é\0🙂".into(), "界".repeat(73)];
    let keys = source_with_base(&texts, &m, &w, &schemas, id, 19);
    let seed = source_with_base(&texts, &m, &w, &schemas, id, 900);
    assert!(!seed.shares_frozen_storage(&keys));
    let mut bank =
        BorrowedStringValues::new_with_keys(&seed, &keys, &schemas, w.clone(), m.clone(), limits())
            .unwrap();
    assert!(bank.snapshot().shares_frozen_storage(&keys));
    assert_eq!(
        bank.entries()
            .unwrap()
            .map(|(k, v)| (k.to_owned(), v))
            .collect::<Vec<_>>(),
        contents(&seed)
    );
    let ValueData::Map(map) = keys.data() else {
        unreachable!()
    };
    for (i, e) in map.entries().iter().enumerate() {
        let ValueData::String(text) = e.key().data() else {
            unreachable!()
        };
        bank.replace(&keys, i, text, i as u64).unwrap();
    }
    assert!(contents(&keys).iter().all(|(_, v)| *v >= 19 && *v < 900));
    assert!(contents(&seed).iter().all(|(_, v)| *v >= 900));
    let wrong = source(
        &["a".into(), "b".into(), "different".into()],
        &m,
        &w,
        &schemas,
        id,
    );
    let baseline = (m.used_bytes(), m.retained_nodes());
    assert!(
        BorrowedStringValues::new_with_keys(
            &seed,
            &wrong,
            &schemas,
            w.clone(),
            m.clone(),
            limits()
        )
        .is_err()
    );
    assert_eq!((m.used_bytes(), m.retained_nodes()), baseline);
    drop((bank, keys, seed, wrong, w));
    assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
}
fn contents(v: &Value) -> Vec<(String, u64)> {
    let ValueData::Map(map) = v.data() else {
        panic!("Map");
    };
    map.entries()
        .iter()
        .map(|e| {
            let (ValueData::String(k), ValueData::U64(v)) = (e.key().data(), e.value()) else {
                panic!("String/U64");
            };
            (k.to_string(), *v)
        })
        .collect()
}
#[test]
fn borrowed_dictionary_values_pin_utf8_keys_but_keep_histogram_counts_independent() {
    for texts in [
        vec![],
        vec!["".into(), "a\0b".into(), "é🙂".into(), "界".repeat(100)],
        (0..73)
            .map(|i| format!("key{i}-{}", "é".repeat(i)))
            .collect(),
    ] {
        let m = ManagedMemoryBudget::new(64 << 20);
        let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
        let (schemas, id) = schema();
        let original = source(&texts, &m, &w, &schemas, id);
        let pin = original.clone();
        let expected = contents(&pin);
        let mut bank =
            BorrowedStringValues::new(&original, &schemas, w.clone(), m.clone(), limits()).unwrap();
        assert!(bank.snapshot().shares_frozen_storage(&pin));
        assert_eq!(bank.len(), expected.len());
        assert_eq!(
            bank.key_bytes(),
            texts.iter().map(|s| s.len() as u64).sum::<u64>()
        );
        assert!(bank.retained_bytes().unwrap() <= m.used_bytes());
        assert_eq!(bank.retained_nodes(), 2);
        drop(original);
        let ValueData::Map(map) = pin.data() else {
            unreachable!()
        };
        let mut addresses = Vec::new();
        for (i, entry) in map.entries().iter().enumerate() {
            let (ValueData::String(key), ValueData::U64(old)) = (entry.key().data(), entry.value())
            else {
                unreachable!()
            };
            addresses.push(key.as_ptr() as usize);
            assert_eq!(bank.replace(&pin, i, key, i as u64).unwrap(), *old);
        }
        assert_eq!(
            contents(&pin),
            expected,
            "renumbering must not mutate original histogram counts"
        );
        assert_eq!(
            bank.entries()
                .unwrap()
                .map(|(k, v)| (k.to_owned(), v))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .enumerate()
                .map(|(i, (k, _))| (k.clone(), i as u64))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            bank.entries()
                .unwrap()
                .map(|(k, _)| k.as_ptr() as usize)
                .collect::<Vec<_>>(),
            addresses
        );
        let before = bank
            .entries()
            .unwrap()
            .map(|(k, v)| (k.to_owned(), v))
            .collect::<Vec<_>>();
        assert!(bank.replace(&pin, usize::MAX, "", 999).is_err());
        let equal = source(&texts, &m, &w, &schemas, id);
        assert!(!equal.shares_frozen_storage(&pin));
        assert!(bank.replace(&equal, 0, "", 999).is_err());
        if map.entries().len() > 1 {
            let ValueData::String(other) = map.entries()[1].key().data() else {
                unreachable!()
            };
            assert!(bank.replace(&pin, 0, other, 999).is_err());
        }
        assert_eq!(
            bank.entries()
                .unwrap()
                .map(|(k, v)| (k.to_owned(), v))
                .collect::<Vec<_>>(),
            before
        );
        drop((bank, pin, equal, w));
        assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
    }
}
#[test]
fn borrowed_dictionary_values_deny_unadmitted_and_finite_capacity_without_leaking() {
    let m = ManagedMemoryBudget::new(64 << 20);
    let w = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &m).unwrap();
    let (schemas, id) = schema();
    let original = source(&["é\0🙂".into(), "long".repeat(300)], &m, &w, &schemas, id);
    let baseline = (m.used_bytes(), m.retained_nodes());
    let foreign = ManagedMemoryBudget::new(64 << 20);
    let fw = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &foreign).unwrap();
    assert!(BorrowedStringValues::new(&original, &schemas, fw, foreign.clone(), limits()).is_err());
    assert_eq!((foreign.used_bytes(), foreign.retained_nodes()), (0, 0));
    let exact = size_of::<BorrowedStringValues>() as u64 + 2 * 2 * size_of::<u64>() as u64;
    for l in [
        Limits {
            entries: 1,
            ..limits()
        },
        Limits {
            retained_bytes: exact - 1,
            ..limits()
        },
        Limits {
            temporary_bytes: exact - 1,
            ..limits()
        },
    ] {
        assert!(BorrowedStringValues::new(&original, &schemas, w.clone(), m.clone(), l).is_err());
        assert_eq!((m.used_bytes(), m.retained_nodes()), baseline);
    }
    let bank = BorrowedStringValues::new(
        &original,
        &schemas,
        w.clone(),
        m.clone(),
        Limits {
            retained_bytes: exact,
            temporary_bytes: exact,
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(bank.retained_bytes().unwrap(), exact);
    drop(bank);
    assert_eq!((m.used_bytes(), m.retained_nodes()), baseline);
    let owner_bytes = size_of::<OwnedStringMap>() as u64;
    let bank_bytes = size_of::<BorrowedStringValues>() as u64;
    let boxed_peak = owner_bytes + 2 * bank_bytes + 4 * size_of::<u64>() as u64;
    for temporary_bytes in [boxed_peak - 1, boxed_peak] {
        let result = OwnedStringMap::try_with_borrowed_keys(
            &original,
            &original,
            &schemas,
            w.clone(),
            Limits {
                temporary_bytes,
                ..limits()
            },
            m.clone(),
        );
        assert_eq!(result.is_ok(), temporary_bytes == boxed_peak);
        drop(result);
        assert_eq!((m.used_bytes(), m.retained_nodes()), baseline);
    }
    let mut first = BorrowedStringValues::new(&original, &schemas, w, m.clone(), limits()).unwrap();
    let ValueData::Map(map) = original.data() else {
        unreachable!()
    };
    let ValueData::String(text) = map.entries()[0].key().data() else {
        unreachable!()
    };
    let short = NativeLibraryWorkBudget::new(127, u64::MAX, &m).unwrap();
    first.work = short;
    let before = first.values.to_vec();
    assert!(first.replace(&original, 0, text, 0).is_err());
    assert_eq!(&*first.values, &*before);
    drop((first, original));
    assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
}

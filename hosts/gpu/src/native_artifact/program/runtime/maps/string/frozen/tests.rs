use super::*;
use mech_core::{
    CardinalitySpec, IntegerWidth, ManagedMemoryBudget, SchemaBody, SchemaDraft,
    SchemaTableBuilder, ValueData,
};
use std::collections::BTreeMap;
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
fn limits() -> MemoryBudgetLimits {
    MemoryBudgetLimits {
        max_output_elements: Some(1),
        max_output_bytes: Some(16 * 1024 * 1024),
        max_temporary_bytes: Some(32 * 1024 * 1024),
        max_cloned_bytes: Some(16 * 1024 * 1024),
        max_retained_nodes: Some(100_000),
        max_compute_work: Some(1_000_000_000),
        max_comparison_work: Some(100_000_000),
        ..Default::default()
    }
}
fn table(m: &ManagedMemoryBudget, global: u64) -> (OwnedStringMap, NativeLibraryWorkBudget) {
    let w = NativeLibraryWorkBudget::new(global, 1_000_000_000, m).unwrap();
    let map = OwnedStringMap::new_with_memory(
        w.clone(),
        Limits {
            entries: 4096,
            retained_bytes: 16 * 1024 * 1024,
            temporary_bytes: 32 * 1024 * 1024,
        },
        m.clone(),
    )
    .unwrap();
    (map, w)
}
fn geometry(m: &ManagedMemoryBudget) -> Geometry {
    Geometry {
        live_bytes: m.used_bytes(),
        live_nodes: m.retained_nodes(),
    }
}
fn contents(v: &Value) -> BTreeMap<String, u64> {
    let ValueData::Map(map) = v.data() else {
        panic!("Map")
    };
    map.entries()
        .iter()
        .map(|e| {
            let (ValueData::String(k), ValueData::U64(v)) = (e.key().data(), e.value()) else {
                panic!("String/U64")
            };
            (k.to_string(), *v)
        })
        .collect()
}
#[test]
fn frozen_generic_dictionary_moves_all_utf8_keys_and_preserves_values_and_late_canonical_order() {
    for keys in [
        vec![],
        vec![
            "".to_string(),
            "é\0🙂".into(),
            "z".repeat(300),
            "東京".repeat(70),
            "a".into(),
        ],
    ] {
        let m = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let (mut map, w) = table(&m, 10_000_000_000);
        let (schemas, id) = schema();
        let mut expected = BTreeMap::new();
        for (k, v) in keys.iter().zip(0u64..) {
            map.insert(k, v).unwrap();
            expected.insert(k.clone(), v);
        }
        if let Some(k) = keys.first() {
            map.insert(k, 99).unwrap();
            expected.insert(k.clone(), 99);
        }
        let addresses = map
            .entries()
            .unwrap()
            .map(|(k, _)| (k.to_string(), k.as_ptr() as usize))
            .collect::<BTreeMap<_, _>>();
        let FreezeChoice::Frozen(f) = map
            .freeze_or_publish(&schemas, id, geometry(&m), limits())
            .unwrap()
        else {
            panic!("view refused")
        };
        assert_eq!(f.schema(), id);
        assert_eq!(f.len(), expected.len());
        assert_eq!(
            f.logical_bytes(),
            logical_map_bytes(f.len() as u64, keys.iter().map(|k| k.len() as u64).sum()).unwrap()
        );
        assert_eq!(
            f.entries()
                .unwrap()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<BTreeMap<_, _>>(),
            expected
        );
        assert_eq!(
            f.entries()
                .unwrap()
                .map(|(k, _)| (k.to_string(), k.as_ptr() as usize))
                .collect::<BTreeMap<_, _>>(),
            addresses
        );
        assert!(f.retained_bytes().unwrap() <= m.used_bytes());
        assert!(f.retained_nodes() <= m.retained_nodes());
        f.check_account(&w).unwrap();
        let other = NativeLibraryWorkBudget::new(100_000, 100_000, &m).unwrap();
        assert!(f.check_account(&other).is_err());
        drop(other);
        let out = f.materialize(geometry(&m)).unwrap();
        assert_eq!(contents(&out), expected);
        drop((out, w));
        assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
    }
}
#[test]
fn frozen_generic_dictionary_nested_call_keeps_immediate_canonical_path_and_logical_caps() {
    let m = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (mut map, w) = table(&m, 10_000_000_000);
    map.insert("kept", 91).unwrap();
    let (schemas, id) = schema();
    let choice = w
        .with_call_limits(1_000_000_000, 100_000_000, || {
            map.freeze_or_publish(&schemas, id, geometry(&m), limits())
        })
        .unwrap()
        .unwrap();
    let FreezeChoice::Canonical(out) = choice else {
        panic!("enclosing relative allowance escaped")
    };
    assert_eq!(contents(&out), BTreeMap::from([("kept".into(), 91)]));
    drop((out, w));
    assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
    for field in ["output", "cloned"] {
        let (mut map, w) = table(&m, 10_000_000_000);
        map.insert("é", 7).unwrap();
        let mut l = limits();
        if field == "output" {
            l.max_output_bytes = Some(0)
        } else {
            l.max_cloned_bytes = Some(0)
        };
        assert!(
            map.freeze_or_publish(&schemas, id, geometry(&m), l)
                .is_err()
        );
        drop(w);
        assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
    }
}
fn attempt(source: u64, global: u64, intervening: u64) -> Result<(u64, u64), Error> {
    let m = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (schemas, id) = schema();
    let result = (|| {
        let (mut map, w) = table(&m, global);
        for (k, v) in [("b", 8), ("a", 9), ("é\0", 3)] {
            map.insert(k, v)?;
        }
        let start = w.used();
        let mut l = limits();
        l.max_compute_work = Some(source);
        let FreezeChoice::Frozen(f) = map.freeze_or_publish(&schemas, id, geometry(&m), l)? else {
            panic!("unexpected canonical route")
        };
        w.charge(intervening, 0)?;
        let out = f.materialize(geometry(&m))?;
        assert_eq!(
            contents(&out),
            BTreeMap::from([("a".into(), 9), ("b".into(), 8), ("é\0".into(), 3)])
        );
        let used = w.used();
        Ok((used.0 - start.0 - intervening, used.0))
    })();
    assert_eq!((m.used_bytes(), m.retained_nodes()), (0, 0));
    result
}
#[test]
fn frozen_generic_dictionary_late_publication_keeps_spent_prefix_relative_cap_and_intervening_global_work()
 {
    super::super::with_fixed_publication_hash_seed(|| {
        let total = attempt(1_000_000_000, 10_000_000_000, 0).unwrap();
        let mut lo = 0;
        let mut hi = total.0;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if attempt(mid, 10_000_000_000, 0).is_ok() {
                hi = mid
            } else {
                lo = mid + 1
            }
        }
        let exact = attempt(lo, 10_000_000_000, 37).unwrap();
        assert!(attempt(lo - 1, 10_000_000_000, 37).is_err());
        assert_eq!(exact.0, total.0);
        assert_eq!(exact.1, total.1 + 37);
        assert!(attempt(lo, exact.1 - 1, 37).is_err());
        assert_eq!(attempt(lo, exact.1, 37).unwrap(), exact);
    });
}

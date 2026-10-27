use super::*;
use mech_core::{
    CardinalitySpec, DimensionExpr, IntegerWidth, SchemaBody, SchemaDraft, SchemaTableBuilder,
};

fn schema(count: Option<u64>) -> (mech_core::SchemaTable, mech_core::SchemaId) {
    let mut builder = SchemaTableBuilder::new();
    let id = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Map {
                    key: Box::new(SchemaBody::String),
                    value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                    cardinality: count
                        .map_or(CardinalitySpec::Dynamic { upper_bound: None }, |n| {
                            CardinalitySpec::Exact(DimensionExpr::Constant(n))
                        }),
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let id = built.resolve(id).unwrap();
    (built.into_parts().0, id)
}

#[test]
fn frozen_storage_retains_unqueried_keys_zero_full_counts_and_domain_membership() {
    for (domain, keys, missing) in [
        (
            boundary_domain(b"abc"),
            vec![b"ab".to_vec(), b"ca".to_vec(), b"bb".to_vec()],
            "eb".to_owned(),
        ),
        (
            Domain::ascii(),
            vec![
                b"abcdefghijklmnopqr".to_vec(),
                b"Abcdefghijklmnopqr".to_vec(),
                b"zbcdefghijklmnopqr".to_vec(),
            ],
            "missing".to_owned(),
        ),
    ] {
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = new_work(&memory);
        let mut map = PackedMap::new_encoded(
            work.clone(),
            limits(),
            keys[0].len() as u64,
            body(),
            0,
            domain,
        )
        .unwrap();
        put(&mut map, &keys[0], 0);
        put(&mut map, &keys[1], u64::MAX);
        put(&mut map, &keys[2], 19); // No demanded-key information exists during freeze.
        let old_claims = (memory.used_bytes(), memory.retained_nodes());
        let frozen = map.freeze().unwrap();
        assert_eq!((frozen.updates(), frozen.traversals()), (3, 3));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), old_claims);
        for (key, expected) in keys.iter().zip([0, u64::MAX, 19]) {
            assert_eq!(
                frozen.lookup(std::str::from_utf8(key).unwrap()).unwrap(),
                Some(expected)
            );
        }
        for query in [missing.as_str(), "", "é", "abb"] {
            assert_eq!(frozen.lookup(query).unwrap(), None, "{query:?}");
        }
        let (schemas, id) = schema(Some(3));
        let bytes = crate::native_artifact::owned_string_map::logical_map_bytes(
            3,
            3 * keys[0].len() as u64,
        )
        .unwrap();
        let mut exact = limits();
        exact.max_output_bytes = Some(bytes);
        exact.max_cloned_bytes = Some(bytes);
        frozen.admit_output(&schemas, id, exact).unwrap();
        for dimension in 0..3 {
            let mut short = exact;
            match dimension {
                0 => short.max_output_bytes = Some(bytes - 1),
                1 => short.max_cloned_bytes = Some(bytes - 1),
                _ => short.max_output_elements = Some(0),
            }
            assert!(matches!(
                frozen.admit_output(&schemas, id, short),
                Err(Error::Limit { .. })
            ));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), old_claims);
        }
        let (wrong, id) = schema(Some(2));
        assert!(frozen.admit_output(&wrong, id, limits()).is_err());
        drop((frozen, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn frozen_storage_rejects_duplicate_unreachable_control_and_count_corruption() {
    for corruption in 0..4 {
        let memory = ManagedMemoryBudget::new(64 << 20);
        let work = new_work(&memory);
        let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
        put(&mut map, b"a", 11);
        let (low, high) = packed(b"a");
        let first = PackedMap::probe(&map.slots, &map.controls, low, high).unwrap();
        match corruption {
            0 => {
                // A duplicate with a correct fingerprint still fails exact reachability.
                let second = (first + 1) & 7;
                map.slots.put(
                    second,
                    Slot {
                        low,
                        high,
                        value: 77,
                    },
                );
                set_control(&mut map.controls, 8, second, fingerprint(hash(low, high)));
                map.frame.len = 2;
            }
            1 => {
                map.frame.len = 2;
            }
            2 => {
                set_control(&mut map.controls, 8, first, 0xff);
            }
            _ => {
                // Full-key validation catches bits beyond the declared width.
                map.slots.put(
                    first,
                    Slot {
                        low: low | (1 << 20),
                        high,
                        value: 11,
                    },
                );
            }
        }
        assert!(map.freeze().is_err());
        drop(work);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn frozen_validation_and_queries_keep_one_finite_call_scope_and_cleanup_on_denial() {
    let memory = ManagedMemoryBudget::new(64 << 20);
    let work = new_work(&memory);
    let ledger_claims = (memory.used_bytes(), memory.retained_nodes());
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    for byte in b"abcdef" {
        put(&mut map, &[*byte], u64::from(*byte));
    }
    map.limits.max_comparison_work = Some(16); // One group succeeds; the complete second key cannot renew it.
    assert!(matches!(map.freeze(), Err(Error::Work(_))));
    assert_eq!(
        (memory.used_bytes(), memory.retained_nodes()),
        ledger_claims
    );
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    put(&mut map, b"a", 0);
    let frozen = map.freeze().unwrap();
    let claims = (memory.used_bytes(), memory.retained_nodes());
    let before = work.used();
    let denied = work
        .with_call_limits(255, 100, || frozen.lookup("a"))
        .unwrap();
    assert!(matches!(denied, Err(Error::Work(_))));
    assert_eq!(work.used(), before, "denied first debit is atomic");
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), claims);
    assert_eq!(
        work.with_call_limits(4096, 100, || frozen.lookup("a"))
            .unwrap()
            .unwrap(),
        Some(0)
    );
    drop((frozen, work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

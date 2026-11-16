use super::*;

fn region() -> Region {
    Region {
        collection: 0,
        iteration: 1,
        header: 2,
        exit: 3,
        source: 4,
        classification_site: Some(5),
        map_root: 6,
        map_ordinal: None,
        element_site: 7,
        project_site: None,
        read_site: 8,
        conversion_site: Some(9),
        output_width: 1,
        push_site: 10,
        output: 11,
        replay_facts: None,
    }
}

#[test]
fn replay_cache_is_inline_nonserialized_and_not_source_proof_authority() {
    assert_eq!(core::mem::size_of::<ReplayFacts>(), 16);
    let empty = region();
    let wire = serde_json::to_vec(&empty).unwrap();
    let mut cached = empty.clone();
    cached.replay_facts = Some(ReplayFacts {
        traversal_base: u64::MAX,
        probe_base: 1,
    });
    assert_eq!(
        cached, empty,
        "source-proof equality excludes the derived cache"
    );
    assert_eq!(serde_json::to_vec(&cached).unwrap(), wire);
    let loaded: Region = serde_json::from_slice(&wire).unwrap();
    assert_eq!(
        loaded.replay_facts(),
        None,
        "no image can supply cached authority"
    );
    let source_fields = serde_json::to_value(&empty).unwrap();
    let fields = source_fields.as_object().unwrap();
    for (name, original) in fields {
        let mut changed = source_fields.clone();
        changed[name] = match original {
            serde_json::Value::Number(number) => {
                serde_json::json!(number.as_u64().unwrap().checked_add(1).unwrap())
            }
            serde_json::Value::Null => serde_json::json!(0),
            serde_json::Value::Bool(value) => serde_json::json!(!value),
            serde_json::Value::String(value) => serde_json::json!(format!("{value}changed")),
            _ => panic!("new source proof field {name} needs an explicit mutation witness"),
        };
        let different: Region = serde_json::from_value(changed).unwrap();
        assert_ne!(
            different, empty,
            "source proof comparison must include wire field {name}"
        );
    }
    let regions = Vec::<Region>::with_capacity(4);
    assert_eq!(
        regions
            .capacity()
            .checked_mul(core::mem::size_of::<Region>())
            .unwrap(),
        regions.capacity() * core::mem::size_of::<Region>()
    );
    assert!(
        usize::MAX
            .checked_mul(core::mem::size_of::<Region>())
            .is_none()
    );
}

#[test]
fn cached_positive_sum_reassociation_preserves_some_none_at_u64_boundaries() {
    let boundaries = [
        0,
        1,
        127,
        128,
        65536,
        u64::MAX / 128,
        u64::MAX / 128 + 1,
        u64::MAX - 65536,
        u64::MAX,
    ];
    for visits in boundaries {
        for library in boundaries {
            for inventory_work in boundaries {
                let base = visits
                    .checked_add(library)
                    .and_then(|v| v.checked_add(inventory_work))
                    .and_then(|v| v.checked_add(65536));
                for dynamic in boundaries {
                    // Original traversal order from runtime: visits+library,
                    // map-call, payload*128, retirement, inventory,65536.
                    let original = visits
                        .checked_add(library)
                        .and_then(|v| v.checked_add(dynamic))
                        .and_then(|v| v.checked_add(dynamic.checked_mul(128)?))
                        .and_then(|v| v.checked_add(dynamic))
                        .and_then(|v| v.checked_add(inventory_work))
                        .and_then(|v| v.checked_add(65536));
                    let cached = base.and_then(|traversal_base| {
                        ReplayFacts {
                            traversal_base,
                            probe_base: 0,
                        }
                        .traversal(dynamic, dynamic, dynamic)
                    });
                    assert_eq!(cached, original);
                    let old_probe = visits
                        .checked_add(library)
                        .and_then(|v| v.checked_mul(128))
                        .and_then(|v| v.checked_add(inventory_work))
                        .and_then(|v| v.checked_add(dynamic))
                        .and_then(|v| v.checked_add(65536));
                    let new_probe = visits
                        .checked_add(library)
                        .and_then(|v| v.checked_mul(128))
                        .and_then(|v| v.checked_add(inventory_work))
                        .and_then(|v| v.checked_add(65536))
                        .and_then(|probe_base| {
                            ReplayFacts {
                                traversal_base: 0,
                                probe_base,
                            }
                            .probe(dynamic)
                        });
                    assert_eq!(new_probe, old_probe);
                }
            }
        }
    }
}

use super::*;
use mech_core::{ManagedMemoryBudget, SchemaDraft, SchemaTableBuilder};
use std::collections::BTreeMap;

fn schemas(body: SchemaBody) -> (Arc<SchemaTable>, SchemaId) {
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let id = built.resolve(handle).unwrap();
    let (table, _) = built.into_parts();
    (Arc::new(table), id)
}
fn map_schema(cardinality: CardinalitySpec) -> (Arc<SchemaTable>, SchemaId) {
    schemas(SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
        cardinality,
    })
}
fn dynamic() -> (Arc<SchemaTable>, SchemaId) {
    map_schema(CardinalitySpec::Dynamic { upper_bound: None })
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
fn table(memory: &ManagedMemoryBudget) -> OwnedStringMap {
    let work = NativeLibraryWorkBudget::new(10_000_000_000, 1_000_000_000, memory).unwrap();
    OwnedStringMap::new(
        work,
        Limits {
            entries: 4096,
            retained_bytes: 16 * 1024 * 1024,
            temporary_bytes: 32 * 1024 * 1024,
        },
    )
    .unwrap()
}
fn geometry(map: &OwnedStringMap) -> Geometry {
    Geometry {
        live_bytes: map.work.memory_budget().used_bytes(),
        live_nodes: map.len() as u64 + 1,
    }
}
fn contents(value: &Value) -> BTreeMap<String, u64> {
    let ValueData::Map(map) = value.data() else {
        panic!("map");
    };
    map.entries()
        .iter()
        .map(|entry| {
            let (ValueData::String(key), ValueData::U64(value)) =
                (entry.key().data(), entry.value())
            else {
                panic!("String/U64");
            };
            (key.to_string(), *value)
        })
        .collect()
}

#[test]
fn owned_map_publication_preserves_arbitrary_values_and_independent_prior_snapshots() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (schemas, schema) = dynamic();
    let mut map = table(&memory);
    let empty = map
        .publish(&schemas, schema, geometry(&map), limits())
        .unwrap();
    assert!(contents(&empty).is_empty());
    for (key, value) in [
        ("z", 0),
        ("é\0🙂", u64::MAX),
        ("", 7),
        ("a\0", 43),
        ("a", 19),
    ] {
        map.insert(key, value).unwrap();
    }
    let output = map
        .publish(&schemas, schema, geometry(&map), limits())
        .unwrap();
    let expected = BTreeMap::from([
        ("z".into(), 0),
        ("é\0🙂".into(), u64::MAX),
        ("".into(), 7),
        ("a\0".into(), 43),
        ("a".into(), 19),
    ]);
    assert_eq!(contents(&output), expected);
    assert!(output.memory_budget_retained_bytes(&memory).is_some());
    let foreign = ManagedMemoryBudget::new(64 * 1024 * 1024);
    assert!(output.memory_budget_retained_bytes(&foreign).is_none());
    map.insert("é\0🙂", 2).unwrap();
    assert_eq!(contents(&output), expected);
    assert!(contents(&empty).is_empty());
    let new = map
        .publish(&schemas, schema, geometry(&map), limits())
        .unwrap();
    assert_eq!(contents(&new)["é\0🙂"], 2);
    drop((new, empty, output, map));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn owned_map_publication_memory_denial_preserves_table_and_retry() {
    let memory = ManagedMemoryBudget::new(8 * 1024 * 1024);
    let (schemas, schema) = dynamic();
    let mut map = table(&memory);
    map.insert("one", 91).unwrap();
    let before = memory.used_bytes();
    let capacity = map.capacity();
    let live = geometry(&map);
    let pressure = memory
        .reserve_capacity(memory.limit_bytes() - before)
        .unwrap();
    assert!(matches!(
        map.publish(&schemas, schema, live, limits()),
        Err(Error::Memory(_))
    ));
    assert_eq!(memory.used_bytes(), memory.limit_bytes());
    assert_eq!(map.capacity(), capacity);
    assert_eq!(map.get("one").unwrap(), Some(91));
    drop(pressure);
    assert_eq!(memory.used_bytes(), before);
    let output = map
        .publish(&schemas, schema, geometry(&map), limits())
        .unwrap();
    assert_eq!(contents(&output), BTreeMap::from([("one".into(), 91)]));
    drop((output, map));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn owned_map_publication_checks_complete_geometry_and_exact_temporary_boundary() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (schemas, schema) = dynamic();
    let mut map = table(&memory);
    map.insert("key", 123).unwrap();
    let live = geometry(&map);
    let plan = map
        .publication_plan(&schemas, schema, live, limits())
        .unwrap();
    let before = memory.used_bytes();
    for bad in [
        Geometry {
            live_bytes: 0,
            ..live
        },
        Geometry {
            live_nodes: 0,
            ..live
        },
    ] {
        assert!(matches!(
            map.publish(&schemas, schema, bad, limits()),
            Err(Error::Publication(_))
        ));
        assert_eq!(memory.used_bytes(), before);
    }
    let mut tight = limits();
    tight.max_temporary_bytes = Some(live.live_bytes + plan.workspace - 1);
    assert!(matches!(
        map.publish(&schemas, schema, live, tight),
        Err(Error::Limit {
            field: "temporary bytes",
            ..
        })
    ));
    assert_eq!(memory.used_bytes(), before);
    tight.max_temporary_bytes = Some(live.live_bytes + plan.workspace);
    let output = map.publish(&schemas, schema, live, tight).unwrap();
    assert_eq!(contents(&output)["key"], 123);
    // A prior output remains separately funded and included in the next call.
    let retained_output = output.memory_budget_retained_bytes(&memory).unwrap();
    let with_prior = Geometry {
        live_bytes: live.live_bytes + retained_output,
        live_nodes: live.live_nodes + output.retained_footprint(&schemas).unwrap().node_count,
    };
    tight.max_temporary_bytes = Some(with_prior.live_bytes + plan.workspace - 1);
    assert!(map.publish(&schemas, schema, with_prior, tight).is_err());
    assert_eq!(contents(&output)["key"], 123);
    drop((output, map));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn owned_map_publication_requires_finite_caps_and_denies_each_call_dimension() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (schemas, schema) = dynamic();
    let mut map = table(&memory);
    map.insert("x", 99).unwrap();
    let before = memory.used_bytes();
    for index in 0..7 {
        for value in [None, Some(0)] {
            let mut tight = limits();
            match index {
                0 => tight.max_output_elements = value,
                1 => tight.max_output_bytes = value,
                2 => tight.max_temporary_bytes = value,
                3 => tight.max_cloned_bytes = value,
                4 => tight.max_retained_nodes = value,
                5 => tight.max_compute_work = value,
                _ => tight.max_comparison_work = value,
            }
            assert!(
                map.publish(&schemas, schema, geometry(&map), tight)
                    .is_err(),
                "cap {index}: {value:?}"
            );
            assert_eq!(memory.used_bytes(), before);
            assert_eq!(map.get("x").unwrap(), Some(99));
        }
    }
}

#[test]
fn owned_map_publication_sort_denial_keeps_spent_work_and_releases_workspace() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (schemas, schema) = dynamic();
    let mut map = table(&memory);
    for (key, value) in [("ccc", 17), ("aaa", 42), ("bbb", 91)] {
        map.insert(key, value).unwrap();
    }
    let live = geometry(&map);
    let plan = map
        .publication_plan(&schemas, schema, live, limits())
        .unwrap();
    let before = memory.used_bytes();
    let mut tight = limits();
    tight.max_comparison_work = Some(plan.finalization);
    for _ in 0..3 {
        let used = map.work.used();
        assert!(matches!(
            map.publish(&schemas, schema, live, tight),
            Err(Error::Work(_))
        ));
        assert_eq!(memory.used_bytes(), before);
        assert!(map.work.used().0 > used.0);
        assert_eq!(map.work.used().1 - used.1, plan.finalization);
    }
    let output = map.publish(&schemas, schema, live, limits()).unwrap();
    assert_eq!(
        contents(&output),
        BTreeMap::from([("aaa".into(), 42), ("bbb".into(), 91), ("ccc".into(), 17)])
    );
}

#[test]
fn owned_map_publication_validates_schema_and_exact_or_bounded_cardinality() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let mut map = table(&memory);
    map.insert("first", u64::MAX).unwrap();
    map.insert("second", 7).unwrap();
    for cardinality in [
        CardinalitySpec::Exact(DimensionExpr::Constant(2)),
        CardinalitySpec::Dynamic {
            upper_bound: Some(DimensionExpr::Constant(2)),
        },
        CardinalitySpec::Dynamic {
            upper_bound: Some(DimensionExpr::Constant(3)),
        },
    ] {
        let (schemas, schema) = map_schema(cardinality);
        let output = map
            .publish(&schemas, schema, geometry(&map), limits())
            .unwrap();
        assert_eq!(contents(&output).len(), 2);
        let footprint = output.retained_footprint(&schemas).unwrap();
        let mut tight = limits();
        tight.max_output_bytes = Some(footprint.retained_bytes - 1);
        assert!(matches!(
            map.publish(&schemas, schema, geometry(&map), tight),
            Err(Error::Limit {
                field: "output bytes",
                ..
            })
        ));
        tight.max_output_bytes = Some(footprint.retained_bytes);
        assert!(map.publish(&schemas, schema, geometry(&map), tight).is_ok());
    }
    let before = memory.used_bytes();
    for (schemas, schema) in [
        schemas(SchemaBody::Bool),
        map_schema(CardinalitySpec::Exact(DimensionExpr::Constant(1))),
        map_schema(CardinalitySpec::Dynamic {
            upper_bound: Some(DimensionExpr::Constant(1)),
        }),
    ] {
        assert!(matches!(
            map.publish(&schemas, schema, geometry(&map), limits()),
            Err(Error::Publication(_))
        ));
        assert_eq!(memory.used_bytes(), before);
    }
}

#[test]
fn owned_map_publication_orders_long_prefixes_without_a_quadratic_map_rebuild() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (schemas, schema) = dynamic();
    let mut map = table(&memory);
    let mut expected = BTreeMap::new();
    for index in (0..128u64).rev() {
        let key = format!("{}-{index:04}", "λ\0".repeat(32));
        let value = u64::MAX - index * 17;
        map.insert(&key, value).unwrap();
        expected.insert(key, value);
    }
    let output = map
        .publish(&schemas, schema, geometry(&map), limits())
        .unwrap();
    assert_eq!(contents(&output), expected);
    let ValueData::Map(value) = output.data() else {
        unreachable!()
    };
    for adjacent in value.entries().windows(2) {
        let (ValueData::String(a), ValueData::String(b)) =
            (adjacent[0].key().data(), adjacent[1].key().data())
        else {
            unreachable!()
        };
        assert!(a.as_bytes() < b.as_bytes());
    }
    drop((output, map));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn owned_map_publication_enters_typed_native_transport_without_reviving_stale_handles() {
    use crate::native_artifact::{
        NativeValueArena, NativeValueError, NativeValueSlot, NativeWorkMeter,
    };
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let (schemas, schema) = dynamic();
    let (other, other_schema) = self::schemas(SchemaBody::Bool);
    let types = [
        NativeValueSlot {
            schema,
            key: schemas.get(schema).unwrap().key(),
        },
        NativeValueSlot {
            schema: other_schema,
            key: other.get(other_schema).unwrap().key(),
        },
    ];
    let mut native_work = NativeWorkMeter::new(1_000_000_000).unwrap();
    let mut arena = NativeValueArena::new(&types, 1024 * 1024, &memory, &mut native_work).unwrap();
    let mut map = table(&memory);
    map.insert("live", 17).unwrap();
    let first = map
        .publish(&schemas, schema, geometry(&map), limits())
        .unwrap();
    let old = arena.put(0, &first, &mut native_work).unwrap();
    assert!(matches!(
        arena.put(1, &first, &mut native_work),
        Err(NativeValueError::TypeMismatch)
    ));
    let mut foreign_work = NativeWorkMeter::new(1_000_000_000).unwrap();
    assert!(matches!(
        arena.put(0, &first, &mut foreign_work),
        Err(NativeValueError::WrongWorkLedger)
    ));
    assert_eq!(
        contents(&arena.get(old, &mut native_work).unwrap())["live"],
        17
    );
    map.insert("live", 99).unwrap();
    let next = map
        .publish(&schemas, schema, geometry(&map), limits())
        .unwrap();
    let current = arena.put(0, &next, &mut native_work).unwrap();
    assert!(matches!(
        arena.get(old, &mut native_work),
        Err(NativeValueError::StaleHandle)
    ));
    assert_eq!(
        contents(&arena.get(current, &mut native_work).unwrap())["live"],
        99
    );
    assert_eq!(contents(&first)["live"], 17);
    drop((first, next, map, arena));
    assert_eq!(memory.used_bytes(), 0);
}

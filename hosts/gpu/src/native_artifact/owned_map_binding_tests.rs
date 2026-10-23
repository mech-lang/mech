use super::*;
use mech_core::{
    ManagedMemoryBudget, SchemaId,
    snapshot::{
        EnumDraft, MapEntryDraft, SnapshotValidationContext, ValueDataDraft as Draft, ValueDraft,
    },
};
use mech_engine::{ProgramArtifact, native_artifact::lower_native_program_artifact};
use std::collections::BTreeMap;

fn artifacts() -> [ProgramArtifact; 2] {
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source(include_str!(
            "../../tests/fixtures/native/map-owner-binding-state.mec"
        ))
        .unwrap();
    let original = product.artifact().clone();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [original, decoded]
}
fn limits() -> MemoryBudgetLimits {
    MemoryBudgetLimits {
        max_output_elements: Some(1),
        max_output_bytes: Some(16 * 1024 * 1024),
        max_temporary_bytes: Some(32 * 1024 * 1024),
        max_cloned_bytes: Some(16 * 1024 * 1024),
        max_retained_nodes: Some(1_000_000),
        max_compute_work: Some(1_000_000_000),
        max_comparison_work: Some(100_000_000),
        ..Default::default()
    }
}
fn geometry(memory: &ManagedMemoryBudget) -> Geometry {
    Geometry {
        live_bytes: memory.used_bytes(),
        live_nodes: 10_000,
    }
}
fn value(
    schemas: &Arc<SchemaTable>,
    schema: SchemaId,
    data: Draft,
    memory: &ManagedMemoryBudget,
) -> Value {
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::with_shared_schemas(schemas))
    .unwrap();
    let mut claim = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut claim).unwrap()
}
fn source(
    plan: &Plan,
    region: &NativeMapOwnerRegion,
    schemas: &Arc<SchemaTable>,
    memory: &ManagedMemoryBudget,
) -> Value {
    value(
        schemas,
        plan.values()[region.tail().seed().get() as usize].schema(),
        Draft::Enum(EnumDraft {
            ordinal: 0,
            payload: Some(Box::new(Draft::Tuple(
                vec![
                    Draft::U64(7),
                    Draft::Map(
                        vec![MapEntryDraft {
                            items: vec![Draft::String("start".into()), Draft::U64(u64::MAX)]
                                .into_boxed_slice(),
                        }]
                        .into_boxed_slice(),
                    ),
                ]
                .into_boxed_slice(),
            ))),
        }),
        memory,
    )
}
fn key(
    plan: &Plan,
    site: usize,
    text: &str,
    schemas: &Arc<SchemaTable>,
    memory: &ManagedMemoryBudget,
) -> Value {
    let Action::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
        panic!("call");
    };
    value(
        schemas,
        plan.values()[operands[1].get() as usize].schema(),
        Draft::String(text.into()),
        memory,
    )
}
fn read_site(plan: &Plan, region: &NativeMapOwnerRegion, primitive: Primitive) -> usize {
    region
        .reads()
        .iter()
        .copied()
        .find(|site| {
            matches!(plan.blocks()[*site as usize].action(),
        Action::LibraryCall { template, .. } if template.primitive() == primitive)
        })
        .unwrap() as usize
}
fn contents(data: &ValueData) -> BTreeMap<String, u64> {
    let ValueData::Map(map) = data else {
        panic!("map");
    };
    map.entries()
        .iter()
        .map(|entry| {
            let (ValueData::String(key), ValueData::U64(value)) =
                (entry.key().data(), entry.value())
            else {
                panic!("entry");
            };
            (key.to_string(), *value)
        })
        .collect()
}
fn seed_contents(value: &Value) -> BTreeMap<String, u64> {
    let ValueData::Enum(state) = value.data() else {
        panic!("state");
    };
    let ValueData::Tuple(items) = state.payload().unwrap() else {
        panic!("tuple");
    };
    contents(&items[1])
}

#[test]
fn map_binding_versions_real_tables_and_preserves_seed_and_published_snapshots() {
    for artifact in artifacts() {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let regions = plan.map_owner_regions().unwrap();
        let region = &regions[0];
        let schemas = Arc::new(artifact.schemas().clone());
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(10_000_000_000, 1_000_000_000, &memory).unwrap();
        let source = source(&plan, region, &schemas, &memory);
        let (mut binding, words) = Binding::import(
            &plan,
            region,
            &schemas,
            &source,
            work.clone(),
            geometry(&memory),
            limits(),
        )
        .unwrap();
        let old = words.words()[2];
        assert_eq!(&words.words()[..2], &[0, 7]);
        assert_ne!(old, 0);
        let contains = read_site(&plan, region, Primitive::MapContainsKey);
        let get = read_site(&plan, region, Primitive::MapKeyAccess);
        let insert = region.writes()[0] as usize;
        let start = key(&plan, get, "start", &schemas, &memory);
        let fresh = key(&plan, get, "é\0🙂", &schemas, &memory);
        let mut scalar = limits();
        scalar.max_output_bytes = Some((size_of::<Value>() + size_of::<ValueData>()) as u64);
        scalar.max_cloned_bytes = Some(0);
        assert_eq!(
            binding
                .read(get, old, &start, geometry(&memory), scalar)
                .unwrap(),
            u64::MAX
        );
        let missing_key_geometry = Geometry {
            live_bytes: binding.retained_bytes().unwrap(),
            live_nodes: 10_000,
        };
        assert!(matches!(
            binding.read(get, old, &start, missing_key_geometry, limits()),
            Err(Error::Binding("incomplete key geometry"))
        ));
        assert_eq!(
            binding
                .read(contains, old, &start, geometry(&memory), limits())
                .unwrap(),
            1
        );
        assert_eq!(
            binding
                .read(get, old, &start, geometry(&memory), limits())
                .unwrap(),
            u64::MAX
        );
        assert_eq!(
            binding
                .read(contains, old, &fresh, geometry(&memory), limits())
                .unwrap(),
            0
        );
        assert!(
            binding
                .read(get, old, &fresh, geometry(&memory), limits())
                .is_err()
        );
        let next = binding
            .insert(insert, old, &fresh, 91, geometry(&memory), limits())
            .unwrap();
        assert_ne!(next, old);
        assert!(
            binding
                .read(get, old, &start, geometry(&memory), limits())
                .is_err()
        );
        let next2 = binding
            .insert(insert, next, &start, 2, geometry(&memory), limits())
            .unwrap();
        assert_eq!(
            binding
                .read(get, next2, &start, geometry(&memory), limits())
                .unwrap(),
            2
        );
        let output = binding
            .finish(
                region.tail().exit() as usize,
                next2,
                &schemas,
                geometry(&memory),
                limits(),
            )
            .unwrap();
        assert_eq!(
            contents(output.data()),
            BTreeMap::from([("start".into(), 2), ("é\0🙂".into(), 91)])
        );
        assert_eq!(
            seed_contents(&source),
            BTreeMap::from([("start".into(), u64::MAX)])
        );
        assert!(
            binding
                .read(get, next2, &start, geometry(&memory), limits())
                .is_err()
        );
        assert!(
            binding
                .finish(
                    region.tail().exit() as usize,
                    next2,
                    &schemas,
                    geometry(&memory),
                    limits()
                )
                .is_err()
        );
        assert!(binding.map.is_none());
        drop((binding, words, output, source, start, fresh, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn map_binding_rejects_foreign_references_sites_keys_and_active_work_ledgers() {
    for artifact in artifacts() {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let regions = plan.map_owner_regions().unwrap();
        let region = &regions[0];
        let schemas = Arc::new(artifact.schemas().clone());
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(10_000_000_000, 1_000_000_000, &memory).unwrap();
        let source = source(&plan, region, &schemas, &memory);
        let (mut left, a) = Binding::import(
            &plan,
            region,
            &schemas,
            &source,
            work.clone(),
            geometry(&memory),
            limits(),
        )
        .unwrap();
        let (right, b) = Binding::import(
            &plan,
            region,
            &schemas,
            &source,
            work.clone(),
            geometry(&memory),
            limits(),
        )
        .unwrap();
        let a_token = a.words()[2];
        let b_token = b.words()[2];
        assert_ne!(a_token, b_token);
        let get = read_site(&plan, region, Primitive::MapKeyAccess);
        let put = region.writes()[0] as usize;
        let key = key(&plan, get, "start", &schemas, &memory);
        assert!(
            left.read(get, b_token, &key, geometry(&memory), limits())
                .is_err()
        );
        assert!(
            right
                .read(get, a_token, &key, geometry(&memory), limits())
                .is_err()
        );
        assert!(
            left.read(put, a_token, &key, geometry(&memory), limits())
                .is_err()
        );
        assert!(
            left.insert(get, a_token, &key, 4, geometry(&memory), limits())
                .is_err()
        );
        assert!(
            left.finish(
                region.tail().initialize() as usize,
                a_token,
                &schemas,
                geometry(&memory),
                limits()
            )
            .is_err()
        );
        let foreign_memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let foreign_key = self::key(&plan, get, "start", &schemas, &foreign_memory);
        assert!(
            left.read(get, a_token, &foreign_key, geometry(&memory), limits())
                .is_err()
        );
        let foreign_work = NativeLibraryWorkBudget::new(1_000_000, 1_000_000, &memory).unwrap();
        assert!(matches!(
            foreign_work
                .with_scope(|| left.read(get, a_token, &key, geometry(&memory), limits()))
                .unwrap(),
            Err(Error::Work(_))
        ));
        assert_eq!(
            left.read(get, a_token, &key, geometry(&memory), limits())
                .unwrap(),
            u64::MAX
        );
        drop((
            left,
            right,
            a,
            b,
            key,
            foreign_key,
            source,
            work,
            foreign_work,
        ));
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(foreign_memory.used_bytes(), 0);
    }
}

#[test]
fn map_binding_import_denials_preserve_seed_and_complete_cleanup() {
    let artifact = artifacts().into_iter().next().unwrap();
    let plan = lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let regions = plan.map_owner_regions().unwrap();
    let region = &regions[0];
    let schemas = Arc::new(artifact.schemas().clone());
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = NativeLibraryWorkBudget::new(10_000_000_000, 1_000_000_000, &memory).unwrap();
    let source = source(&plan, region, &schemas, &memory);
    let before = memory.used_bytes();
    let live = geometry(&memory);
    let pressure = memory
        .reserve_capacity(memory.limit_bytes() - before)
        .unwrap();
    assert!(matches!(
        Binding::import(
            &plan,
            region,
            &schemas,
            &source,
            work.clone(),
            live,
            limits()
        ),
        Err(Error::Memory(_))
    ));
    assert_eq!(memory.used_bytes(), memory.limit_bytes());
    drop(pressure);
    assert_eq!(memory.used_bytes(), before);
    for index in 0..7 {
        let mut tight = limits();
        match index {
            0 => tight.max_output_elements = None,
            1 => tight.max_output_bytes = None,
            2 => tight.max_temporary_bytes = None,
            3 => tight.max_cloned_bytes = None,
            4 => tight.max_retained_nodes = None,
            5 => tight.max_compute_work = None,
            _ => tight.max_comparison_work = None,
        }
        assert!(
            Binding::import(&plan, region, &schemas, &source, work.clone(), live, tight).is_err()
        );
        assert_eq!(memory.used_bytes(), before);
    }
    let foreign_memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let foreign = self::source(&plan, region, &schemas, &foreign_memory);
    assert!(
        Binding::import(
            &plan,
            region,
            &schemas,
            &foreign,
            work.clone(),
            live,
            limits()
        )
        .is_err()
    );
    assert_eq!(seed_contents(&source)["start"], u64::MAX);
    let (binding, initial) = Binding::import(
        &plan,
        region,
        &schemas,
        &source,
        work.clone(),
        live,
        limits(),
    )
    .unwrap();
    drop((binding, initial, source, foreign, work));
    assert_eq!(memory.used_bytes(), 0);
    assert_eq!(foreign_memory.used_bytes(), 0);
}

#[test]
fn map_binding_denied_mutations_and_publication_keep_current_version_for_retry() {
    let artifact = artifacts().into_iter().next().unwrap();
    let plan = lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let regions = plan.map_owner_regions().unwrap();
    let region = &regions[0];
    let schemas = Arc::new(artifact.schemas().clone());
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = NativeLibraryWorkBudget::new(10_000_000_000, 1_000_000_000, &memory).unwrap();
    let source = source(&plan, region, &schemas, &memory);
    let (mut binding, words) = Binding::import(
        &plan,
        region,
        &schemas,
        &source,
        work.clone(),
        geometry(&memory),
        limits(),
    )
    .unwrap();
    let token = words.words()[2];
    let put = region.writes()[0] as usize;
    let get = read_site(&plan, region, Primitive::MapKeyAccess);
    let fresh = key(&plan, get, "new", &schemas, &memory);
    let start = key(&plan, get, "start", &schemas, &memory);
    let before = memory.used_bytes();
    let used = work.used();
    let mut tight = limits();
    tight.max_compute_work = Some(0);
    assert!(matches!(
        binding.insert(put, token, &fresh, 3, geometry(&memory), tight),
        Err(Error::Work(_))
    ));
    tight = limits();
    tight.max_cloned_bytes = Some(0);
    assert!(
        binding
            .insert(put, token, &fresh, 3, geometry(&memory), tight)
            .is_err()
    );
    assert_eq!(binding.token, token);
    assert_eq!(memory.used_bytes(), before);
    assert_eq!(
        binding
            .read(get, token, &start, geometry(&memory), limits())
            .unwrap(),
        u64::MAX
    );
    let next = binding
        .insert(put, token, &start, 2, geometry(&memory), tight)
        .unwrap();
    tight = limits();
    tight.max_output_bytes = Some(0);
    assert!(
        binding
            .finish(
                region.tail().exit() as usize,
                next,
                &schemas,
                geometry(&memory),
                tight
            )
            .is_err()
    );
    assert_eq!(binding.token, next);
    assert!(binding.map.is_some());
    assert!(work.used().0 > used.0);
    let before_exit = work.used();
    tight = limits();
    tight.max_compute_work = Some(256);
    assert!(matches!(
        binding.finish(
            region.tail().exit() as usize,
            next,
            &schemas,
            geometry(&memory),
            tight
        ),
        Err(Error::Work(_))
    ));
    assert_eq!(
        work.used().0 - before_exit.0,
        256,
        "nested publication must not renew the enclosing call allowance"
    );
    assert_eq!(binding.token, next);
    let output = binding
        .finish(
            region.tail().exit() as usize,
            next,
            &schemas,
            geometry(&memory),
            limits(),
        )
        .unwrap();
    assert_eq!(contents(output.data())["start"], 2);
    drop((binding, words, output, start, fresh, source, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn map_binding_token_exhaustion_never_wraps_or_refills_work() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = NativeLibraryWorkBudget::new(1_000_000, 1_000_000, &memory).unwrap();
    let counter = AtomicU64::new(u64::MAX - 1);
    assert_eq!(token_from(&counter, &work).unwrap(), u64::MAX);
    let issued = work.used();
    assert!(issued.0 >= 128 && issued.0 % 128 == 0);
    assert!(matches!(token_from(&counter, &work), Err(Error::Overflow)));
    assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
    assert_eq!(work.used(), (issued.0 + 128, 0));
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn map_binding_rejects_a_region_from_another_program_revision() {
    let first = artifacts().into_iter().next().unwrap();
    let plan = lower_native_program_artifact(&first, &mech_stdlib::source_catalog()).unwrap();
    let regions = plan.map_owner_regions().unwrap();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let changed = include_str!("../../tests/fixtures/native/map-owner-binding-state.mec")
        .replace("opaque", "different");
    let second = compiler.compile_canonical_source(&changed).unwrap();
    let other =
        lower_native_program_artifact(second.artifact(), &mech_stdlib::source_catalog()).unwrap();
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = NativeLibraryWorkBudget::new(10_000_000_000, 1_000_000_000, &memory).unwrap();
    let schemas = Arc::new(first.schemas().clone());
    let seed = source(&plan, &regions[0], &schemas, &memory);
    assert_ne!(plan.program_revision(), other.program_revision());
    let before = memory.used_bytes();
    assert!(matches!(
        Binding::import(
            &other,
            &regions[0],
            &schemas,
            &seed,
            work.clone(),
            geometry(&memory),
            limits()
        ),
        Err(Error::Binding("foreign program revision"))
    ));
    assert_eq!(memory.used_bytes(), before);
    drop((seed, work));
    assert_eq!(memory.used_bytes(), 0);
}

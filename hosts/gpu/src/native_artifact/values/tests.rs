use crate::native_artifact::values::*;
use mech_core::snapshot::{MapEntryDraft, SnapshotValidationContext};
use mech_core::{
    SchemaBody, SchemaDraft, SchemaTableBuilder, ValueData, ValueDataDraft, ValueDraft,
};
const TRANSPORT_LIMIT: u64 = 1024 * 1024;

#[cfg(feature = "canonical-native-program-runtime")]
#[test]
fn arena_transport_preserves_shared_account_census_and_external_roots() {
    let budget = ManagedMemoryBudget::new(16 * 1024 * 1024);
    let (text_ty, raw_a) = text(&"λ".repeat(8192));
    let (_, raw_b) = text("replacement");
    let (map_ty, raw_map) = value(
        SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
            cardinality: mech_core::CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Map(
            [("a", 1), ("b", u64::MAX)]
                .into_iter()
                .map(|(key, value)| MapEntryDraft {
                    items: vec![
                        ValueDataDraft::String(key.into()),
                        ValueDataDraft::U64(value),
                    ]
                    .into_boxed_slice(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    );
    let a = admit(&raw_a, &budget);
    let b = admit(&raw_b, &budget);
    let map = admit(&raw_map, &budget);
    assert_eq!(a.memory_budget_retained_nodes(&budget), Some(2));
    assert_eq!(map.memory_budget_retained_nodes(&budget), Some(6));
    let types = [text_ty, text_ty, map_ty];
    let mut work = NativeWorkMeter::new(100_000_000).unwrap();
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
    assert_eq!(budget.retained_snapshot_nodes(), 10);
    let before = budget.used_bytes();
    arena.put(0, &a, &mut work).unwrap();
    arena.put(1, &a, &mut work).unwrap();
    let handle = arena.put(2, &map, &mut work).unwrap();
    assert_eq!(budget.retained_snapshot_nodes(), 10);
    assert_eq!(
        budget.used_bytes(),
        before,
        "no census allocation on put/read"
    );
    assert!(matches!(
        arena.put(2, &a, &mut work),
        Err(NativeValueError::TypeMismatch)
    ));
    assert_eq!(budget.retained_snapshot_nodes(), 10);
    arena.put(0, &b, &mut work).unwrap();
    arena.retire_current_matching(1, &a, &mut work).unwrap();
    assert_eq!(budget.retained_snapshot_nodes(), 10);
    arena.retire_current(1, &mut work).unwrap();
    assert_eq!(budget.retained_snapshot_nodes(), 10);
    let taken = arena.take(handle, &mut work).unwrap();
    assert_eq!(budget.retained_snapshot_nodes(), 10);
    assert!(matches!(
        arena.get(handle, &mut work),
        Err(NativeValueError::StaleHandle)
    ));
    let mut foreign_work = NativeWorkMeter::new(1_000_000).unwrap();
    assert!(matches!(
        arena.put(0, &a, &mut foreign_work),
        Err(NativeValueError::WrongWorkLedger)
    ));
    arena.retire_current(0, &mut work).unwrap();
    assert_eq!(budget.retained_snapshot_nodes(), 10);
    drop(a);
    assert_eq!(budget.retained_snapshot_nodes(), 8);
    drop(b);
    assert_eq!(budget.retained_snapshot_nodes(), 6);
    drop(map);
    assert_eq!(
        budget.retained_snapshot_nodes(),
        6,
        "taken map remains a live caller root"
    );
    drop(taken);
    assert_eq!(budget.retained_snapshot_nodes(), 0);
    drop((arena, raw_a, raw_b, raw_map, work, foreign_work));
    assert_eq!(budget.used_bytes(), 0);
}

fn value(body: SchemaBody, data: ValueDataDraft) -> (NativeValueSlot, Value) {
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let value = ValueDraft {
        schema: id,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    (
        NativeValueSlot {
            schema: id,
            key: value.schema_key(),
        },
        value,
    )
}
fn admit(value: &Value, budget: &ManagedMemoryBudget) -> Value {
    let mut reservation = budget
        .reserve_capacity(value.memory_budget_admission_bytes(budget).unwrap())
        .unwrap();
    value.clone().into_memory_budget(&mut reservation).unwrap()
}
fn text(text: &str) -> (NativeValueSlot, Value) {
    value(SchemaBody::String, ValueDataDraft::String(text.into()))
}

#[cfg(feature = "canonical-native-program-runtime")]
#[test]
fn private_local_retirement_invalidates_only_its_exact_slot_and_ledger() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("local");
    let owned = admit(&raw, &budget);
    let types = [ty, ty];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
    let local = arena.put(0, &owned, &mut work).unwrap();
    let capture = arena.put(1, &owned, &mut work).unwrap();
    let before = budget.used_bytes();
    let mut foreign = NativeWorkMeter::new(1_000_000).unwrap();
    assert_eq!(
        arena.copy_current_prepaid(0, &foreign).unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    work.charge(NativeExecutionPhase::Setup, 128).unwrap();
    let (read, handle) = arena.copy_current_prepaid(0, &work).unwrap().unwrap();
    assert_eq!(handle, local);
    assert!(read.shares_frozen_storage(&owned));
    drop(read);
    assert_eq!(
        arena.retire_current(0, &mut foreign).unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    assert!(arena.get(local, &mut work).is_ok());
    arena.retire_current(0, &mut work).unwrap();
    assert!(arena.copy_current_prepaid(0, &work).unwrap().is_none());
    assert_eq!(
        arena.get(local, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert!(arena.get(capture, &mut work).is_ok());
    assert_eq!(
        budget.used_bytes(),
        before,
        "the live capture keeps its shared payload"
    );
    arena.retire_current(0, &mut work).unwrap();
    let replacement = arena.put(0, &owned, &mut work).unwrap();
    assert_ne!(replacement, local);
    assert_eq!(
        arena.get(local, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    drop((owned, arena));
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn strings_maps_and_full_u64_payloads_use_real_same_account_managed_slots() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let (text_ty, text_value) = text("unicode λ\0prefix");
    let (map_ty, map_value) = value(
        SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
            cardinality: mech_core::DimensionExpr::Constant(1).into(),
        },
        ValueDataDraft::Map(
            vec![MapEntryDraft {
                items: vec![
                    ValueDataDraft::String("λ".into()),
                    ValueDataDraft::U64(u64::MAX),
                ]
                .into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
    );
    let text_value = admit(&text_value, &budget);
    let map_value = admit(&map_value, &budget);
    let retained = budget.used_bytes();
    let types = [text_ty, map_ty];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    {
        let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
        assert!(budget.used_bytes() > retained);
        let before_put = budget.used_bytes();
        let a = arena.put(0, &text_value, &mut work).unwrap();
        let b = arena.put(1, &map_value, &mut work).unwrap();
        assert_eq!(
            budget.used_bytes(),
            before_put,
            "retaining admitted roots must not import or duplicate their payload"
        );
        assert!(
            matches!(arena.get(a, &mut work).unwrap().data(), ValueData::String(s) if s.as_ref() == "unicode λ\0prefix")
        );
        assert_eq!(
            arena.get(b, &mut work).unwrap().schema_key(),
            map_value.schema_key()
        );
        let ValueData::Map(counts) = arena.get(b, &mut work).unwrap().data() else {
            panic!("expected canonical map")
        };
        assert!(matches!(counts.entries()[0].value(), ValueData::U64(count) if *count == u64::MAX));
        assert_eq!(
            arena.take(b, &mut work).unwrap().schema_key(),
            map_value.schema_key()
        );
        assert_eq!(
            arena.get(b, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
    }
    assert_eq!(
        budget.used_bytes(),
        retained,
        "both realized transport arenas release their account claims"
    );
    drop(text_value);
    drop(map_value);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn late_foreign_imports_do_not_expand_arena_replacement_or_teardown_work() {
    for replace in [false, true] {
        for later in [0, 16] {
            let budget = ManagedMemoryBudget::new(1_000_000);
            let (ty, raw) = text("small");
            let owned = admit(&raw, &budget);
            let types = [ty];
            let mut work = NativeWorkMeter::new(1_000_000).unwrap();
            let mut arena =
                NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
            arena.put(0, &owned, &mut work).unwrap();
            drop(owned); // The arena owns A's last imported wrapper.
            let foreign = (0..later)
                .map(|_| {
                    let account = ManagedMemoryBudget::new(1_000_000);
                    let value = admit(&raw, &account);
                    (account, value)
                })
                .collect::<Vec<_>>();
            if replace {
                let (_, replacement) = text("other");
                let replacement = admit(&replacement, &budget);
                let retained = replacement.memory_budget_retained_bytes(&budget).unwrap();
                let before = work.used();
                arena.put(0, &replacement, &mut work).unwrap();
                assert_eq!(work.used() - before, retained * 4 + 64);
                drop(replacement);
            }
            drop(arena);
            assert_eq!(budget.used_bytes(), 0);
            for (account, value) in foreign {
                assert!(value.memory_budget_retained_bytes(&account).is_some());
                drop(value);
                assert_eq!(account.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn slot_reuse_and_other_arenas_never_revalidate_old_handles() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let (ty, first) = text("first");
    let (_, second) = text("second");
    let first = admit(&first, &budget);
    let second = admit(&second, &budget);
    let types = [ty];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let mut a = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
    let mut b = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
    let old = a.put(0, &first, &mut work).unwrap();
    let current = a.put(0, &second, &mut work).unwrap();
    let foreign = b.put(0, &first, &mut work).unwrap();
    assert_eq!(
        a.get(old, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert_eq!(
        a.get(foreign, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert_eq!(
        b.get(current, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    a.take(current, &mut work).unwrap();
    let next = a.put(0, &first, &mut work).unwrap();
    assert_ne!(next, current);
    assert_eq!(
        a.get(current, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
}

#[test]
fn wrong_types_unadmitted_and_foreign_budget_values_preserve_existing_slots() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let other_budget = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("stable");
    let good = admit(&raw, &budget);
    let foreign = admit(&raw, &other_budget);
    let (_, wrong) = value(
        SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
        ValueDataDraft::U64(42),
    );
    let wrong = admit(&wrong, &budget);
    let types = [ty];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
    let stable = arena.put(0, &good, &mut work).unwrap();
    let retained = budget.used_bytes();
    assert_eq!(
        arena.put(0, &raw, &mut work).unwrap_err(),
        NativeValueError::UnadmittedPayload
    );
    assert_eq!(
        arena.put(0, &foreign, &mut work).unwrap_err(),
        NativeValueError::UnadmittedPayload
    );
    assert_eq!(
        arena.put(0, &wrong, &mut work).unwrap_err(),
        NativeValueError::TypeMismatch
    );
    assert_eq!(
        arena.put(1, &good, &mut work).unwrap_err(),
        NativeValueError::InvalidSlot
    );
    assert_eq!(budget.used_bytes(), retained);
    assert_eq!(
        arena.get(stable, &mut work).unwrap().schema_key(),
        good.schema_key()
    );
}

#[test]
fn setup_transport_shared_memory_and_cumulative_work_denials_fail_closed() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("retained");
    let good = admit(&raw, &budget);
    let types = [ty];
    let mut denied = NativeWorkMeter::new(0).unwrap();
    let before = budget.used_bytes();
    assert!(matches!(
        NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut denied),
        Err(NativeValueError::Base(
            NativeExecutionError::WorkLimit { .. }
        ))
    ));
    assert_eq!(budget.used_bytes(), before);
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    assert!(matches!(
        NativeValueArena::new(&types, 0, &budget, &mut work),
        Err(NativeValueError::Base(
            NativeExecutionError::TransportLimit { .. }
        ))
    ));
    assert_eq!(work.used(), 0);
    let no_memory = ManagedMemoryBudget::new(0);
    assert!(NativeValueArena::new(&types, TRANSPORT_LIMIT, &no_memory, &mut work).is_err());
    assert_eq!(no_memory.used_bytes(), 0);
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
    let handle = arena.put(0, &good, &mut work).unwrap();
    work.charge(NativeExecutionPhase::Setup, work.limit() - work.used())
        .unwrap();
    assert!(matches!(
        arena.put(0, &good, &mut work),
        Err(NativeValueError::Base(
            NativeExecutionError::WorkLimit { .. }
        ))
    ));
    assert!(arena.values.values()[0].is_some());
    assert_eq!(arena.generations.values()[0], handle.generation);
    assert_eq!(
        work.used(),
        work.limit(),
        "no failed call resets cumulative authority"
    );
    let mut fresh = NativeWorkMeter::new(1_000_000).unwrap();
    assert_eq!(
        arena.put(0, &good, &mut fresh).unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    assert_eq!(
        arena.get(handle, &mut fresh).unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    assert_eq!(
        arena.take(handle, &mut fresh).unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    assert_eq!(fresh.used(), 0);
}

#[test]
fn arena_before_promotion_pins_memory_and_retains_the_same_live_handles_and_counter() {
    let memory = ManagedMemoryBudget::new(1_000_000);
    let other = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("same invocation");
    let good = admit(&raw, &memory);
    let types = [ty];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    let handle = arena.put(0, &good, &mut work).unwrap();
    let before = work.used();
    let bytes = memory.used_bytes();
    assert!(
        work.shared_library_budget(NativeExecutionPhase::Setup, 10, &other)
            .is_err()
    );
    assert_eq!(work.used(), before);
    assert_eq!(memory.used_bytes(), bytes);
    assert_eq!(other.used_bytes(), 0);
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
        .unwrap();
    assert_eq!(library.used(), (before + 64, 0));
    assert_eq!(
        arena.get(handle, &mut work).unwrap().schema_key(),
        good.schema_key()
    );
    assert_eq!(library.used(), (before + 80, 0));
    library.charge(3, 2).unwrap();
    assert_eq!(work.used(), before + 83);
    let moved = arena.take(handle, &mut work).unwrap();
    assert!(moved.shares_frozen_storage(&good));
    drop((arena, work, library, good, moved));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn promotion_before_arena_rejects_foreign_backing_before_work_or_allocation() {
    let memory = ManagedMemoryBudget::new(1_000_000);
    let other = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("already shared");
    let good = admit(&raw, &memory);
    let types = [ty];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
        .unwrap();
    let bytes = memory.used_bytes();
    assert!(matches!(
        NativeValueArena::new(&types, TRANSPORT_LIMIT, &other, &mut work),
        Err(NativeValueError::Base(
            NativeExecutionError::Descriptor { .. }
        ))
    ));
    assert_eq!(work.used(), 64);
    assert_eq!(other.used_bytes(), 0);
    assert_eq!(memory.used_bytes(), bytes);
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    let handle = arena.put(0, &good, &mut work).unwrap();
    assert_eq!(library.used().0, work.used());
    assert!(
        arena
            .get(handle, &mut work)
            .unwrap()
            .shares_frozen_storage(&good)
    );
    drop((arena, work, library, good));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn generation_exhaustion_cannot_replace_or_remove_the_live_value() {
    let budget = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("last generation");
    let good = admit(&raw, &budget);
    let types = [ty];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &budget, &mut work).unwrap();
    let mut handle = arena.put(0, &good, &mut work).unwrap();
    arena.generations.values_mut()[0] = u64::MAX;
    handle.generation = u64::MAX;
    assert_eq!(
        arena.put(0, &good, &mut work).unwrap_err(),
        NativeValueError::GenerationExhausted
    );
    assert_eq!(
        arena.take(handle, &mut work).unwrap_err(),
        NativeValueError::GenerationExhausted
    );
    assert!(arena.get(handle, &mut work).is_ok());
}

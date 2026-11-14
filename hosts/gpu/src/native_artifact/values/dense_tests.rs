//! Sparse physical owner banks keep global handles and the original admission.
use super::*;

fn map() -> NativeValueSlotMap {
    NativeValueSlotMap::from_inventory(&[false, true, true, false, true, false, false]).unwrap()
}

#[test]
fn dense_worker_inventory_is_injective_and_keeps_full_admission_and_denial_floors() {
    let slots = map();
    assert_eq!(slots.indices.as_ref(), &[u32::MAX, 0, 1, u32::MAX, 2]);
    assert_eq!((slots.physical_slots(), slots.logical_prefix()), (3, 5));
    let (ty, _) = text("types only");
    let types = [ty; 7];
    let bytes =
        crate::native_artifact::memory::NativeTypedStorage::<Option<Value>>::admission_bytes(
            types.len(),
        )
        .unwrap()
            + crate::native_artifact::memory::NativeWordStorage::admission_bytes(types.len())
                .unwrap();
    let setup = types.len() as u64 * 24 + 256;
    let raw_bytes = (types.len() * (core::mem::size_of::<Option<Value>>() + 8)) as u64;
    for dense in [false, true] {
        for short_memory in [false, true] {
            let memory = ManagedMemoryBudget::new(bytes - u64::from(short_memory));
            let mut work = NativeWorkMeter::new(setup).unwrap();
            let arena = NativeValueArena::new_with_worker_slots(
                &types,
                dense.then_some(&slots),
                raw_bytes,
                &memory,
                &mut work,
            );
            assert_eq!(work.used(), setup);
            if short_memory {
                assert!(arena.is_err());
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            } else {
                let arena = arena.unwrap();
                assert_eq!(arena.values.values().len(), if dense { 3 } else { 7 });
                assert_eq!(arena.generations.values().len(), if dense { 3 } else { 7 });
                assert_eq!(memory.used_bytes(), bytes);
                assert_eq!(arena.types.len(), if dense { 5 } else { 7 });
                drop(arena);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
        }
        let memory = ManagedMemoryBudget::new(bytes);
        let mut work = NativeWorkMeter::new(setup - 1).unwrap();
        assert!(matches!(
            NativeValueArena::new_with_worker_slots(
                &types,
                dense.then_some(&slots),
                raw_bytes,
                &memory,
                &mut work
            ),
            Err(NativeValueError::Base(
                NativeExecutionError::WorkLimit { .. }
            ))
        ));
        assert_eq!((memory.used_bytes(), work.used()), (0, 0));
        let mut work = NativeWorkMeter::new(setup).unwrap();
        assert!(
            matches!(NativeValueArena::new_with_worker_slots(&types, dense.then_some(&slots), raw_bytes - 1, &memory, &mut work), Err(NativeValueError::Base(NativeExecutionError::TransportLimit {required, limit})) if required==raw_bytes && limit==raw_bytes-1)
        );
        assert_eq!((memory.used_bytes(), work.used()), (0, 0));
    }
}

#[test]
fn dense_worker_holes_keep_empty_generation_and_method_specific_error_and_debit_order() {
    let slots = map();
    let mut results = Vec::new();
    for dense in [false, true] {
        let memory = ManagedMemoryBudget::new(1_000_000);
        let (ty, raw) = text("a retained root");
        let owned = admit(&raw, &memory);
        let types = [ty; 7];
        let mut work = NativeWorkMeter::new(1_000_000).unwrap();
        // The old compact prefix and the dense map have the SAME logical bound.
        let mut arena = if dense {
            NativeValueArena::new_with_worker_slots(
                &types,
                Some(&slots),
                TRANSPORT_LIMIT,
                &memory,
                &mut work,
            )
            .unwrap()
        } else {
            NativeValueArena::new_with_prefix(
                &types,
                slots.logical_prefix(),
                TRANSPORT_LIMIT,
                &memory,
                &mut work,
            )
            .unwrap()
        };
        let before = work.used();
        assert!(arena.native_entry_preview(3, &work).unwrap().is_none());
        assert!(arena.native_retirement_available(3, &work).unwrap());
        assert!(arena.copy_current_prepaid(3, &work).unwrap().is_none());
        assert_eq!(
            arena.copy_current(3, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(
            arena.current_handle(3, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(arena.current_retained_bytes(3, &mut work).unwrap(), None);
        arena.retire_current(3, &mut work).unwrap();
        arena.retire_current_matching(3, &owned, &mut work).unwrap();
        let mut hole = NativeValueHandle {
            domain: arena.values.domain_id(),
            slot: 3,
            generation: 0,
        };
        arena.check_virtual(hole, &mut work).unwrap();
        hole.generation = 1;
        assert_eq!(
            arena.check_virtual(hole, &mut work),
            Err(NativeValueError::StaleHandle)
        );
        assert_eq!(
            arena.get(hole, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(
            arena.take(hole, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(
            arena.copy_current_to(3, 99, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(
            arena.copy_current_to(99, 3, &mut work).unwrap_err(),
            NativeValueError::InvalidSlot
        );
        assert_eq!(
            arena.native_entry_preview(5, &work).unwrap_err(),
            NativeValueError::InvalidSlot
        );
        assert_eq!(
            arena.copy_current(5, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(
            arena.retire_current(5, &mut work),
            Err(NativeValueError::InvalidSlot)
        );
        let mut outputs = [Some(owned.clone()), Some(owned.clone())];
        assert_eq!(
            arena.publish_range(2, &mut outputs, &mut work),
            Err(NativeValueError::StaleHandle)
        );
        assert!(
            outputs
                .iter()
                .all(|v| v.as_ref().unwrap().shares_frozen_storage(&owned))
        );
        assert!(
            arena
                .native_alias_put_work(3, &owned, &work)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            arena.put(3, &raw, &mut work).unwrap_err(),
            NativeValueError::UnadmittedPayload
        );
        let (_, wrong) = value(SchemaBody::Bool, ValueDataDraft::Bool(true));
        let wrong = admit(&wrong, &memory);
        assert_eq!(
            arena.put(3, &wrong, &mut work).unwrap_err(),
            NativeValueError::TypeMismatch
        );
        let mut foreign_work = NativeWorkMeter::new(1_000_000).unwrap();
        assert_eq!(
            arena.put(3, &owned, &mut foreign_work).unwrap_err(),
            NativeValueError::WrongWorkLedger
        );
        assert_eq!(
            arena.copy_current(3, &mut foreign_work).unwrap_err(),
            NativeValueError::WrongWorkLedger
        );
        assert_eq!(foreign_work.used(), 0);
        results.push(work.used() - before);
        assert!(arena.current(3).unwrap().is_none());
        assert_eq!(arena.generation(3).unwrap(), 0);
        drop((outputs, arena, work, owned, wrong));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn dense_worker_publication_keeps_paid_prefix_and_fails_closed_for_unproved_slots() {
    let slots = map();
    let (ty, raw) = text("finite put");
    let types = [ty; 7];
    let setup = types.len() as u64 * 24 + 256;
    let probe_memory = ManagedMemoryBudget::new(1_000_000);
    let probe = admit(&raw, &probe_memory);
    let put_work = probe.memory_budget_retained_bytes(&probe_memory).unwrap() * 4 + 64;
    drop(probe);
    for allowance in [0, 31, 32, put_work - 1, put_work] {
        let mut outcomes = Vec::new();
        for dense in [false, true] {
            let memory = ManagedMemoryBudget::new(1_000_000);
            let owned = admit(&raw, &memory);
            let mut work = NativeWorkMeter::new(setup + allowance).unwrap();
            let mut arena = if dense {
                NativeValueArena::new_with_worker_slots(
                    &types,
                    Some(&slots),
                    TRANSPORT_LIMIT,
                    &memory,
                    &mut work,
                )
                .unwrap()
            } else {
                NativeValueArena::new_with_prefix(
                    &types,
                    slots.logical_prefix(),
                    TRANSPORT_LIMIT,
                    &memory,
                    &mut work,
                )
                .unwrap()
            };
            let result = arena.put(3, &owned, &mut work);
            if allowance < put_work {
                outcomes.push((result.unwrap_err(), work.used()));
                assert!(arena.current(3).unwrap().is_none());
                assert_eq!(arena.generation(3).unwrap(), 0);
            } else if dense {
                assert_eq!(result.unwrap_err(), NativeValueError::InvalidSlot);
                assert_eq!(work.used(), setup + put_work);
                assert!(arena.current(3).unwrap().is_none());
                assert_eq!(arena.generation(3).unwrap(), 0);
            } else {
                assert!(result.is_ok());
                assert_eq!(work.used(), setup + put_work);
            }
            drop((arena, work, owned));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
        if allowance < put_work {
            assert_eq!(outcomes[0], outcomes[1]);
        }
    }
}

#[test]
fn dense_worker_global_handles_aliases_tickets_virtuals_publication_and_retry_are_stable() {
    let slots = map();
    let memory = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("first");
    let (_, second) = text("second");
    let first = admit(&raw, &memory);
    let second = admit(&second, &memory);
    let types = [ty; 7];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let mut arena = NativeValueArena::new_with_worker_slots(
        &types,
        Some(&slots),
        TRANSPORT_LIMIT,
        &memory,
        &mut work,
    )
    .unwrap();
    let original = arena.put(1, &first, &mut work).unwrap();
    assert_eq!(original.slot, 1);
    let alias = arena.copy_current_to(1, 4, &mut work).unwrap();
    assert_eq!(alias.slot, 4);
    assert_eq!(arena.physical_index(4), Some(2));
    arena.retire_current(1, &mut work).unwrap();
    assert_eq!(
        arena.get(original, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert!(
        arena
            .get(alias, &mut work)
            .unwrap()
            .shares_frozen_storage(&first)
    );
    let same = arena.copy_current_to(4, 4, &mut work).unwrap();
    assert_eq!(
        arena.get(alias, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert!(
        arena
            .get(same, &mut work)
            .unwrap()
            .shares_frozen_storage(&first)
    );
    let mut foreign = NativeValueArena::new_with_worker_slots(
        &types,
        Some(&slots),
        TRANSPORT_LIMIT,
        &memory,
        &mut work,
    )
    .unwrap();
    let other = foreign.put(4, &second, &mut work).unwrap();
    assert_eq!(
        arena.get(other, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    let ticket = arena.prepare_put(2, &second, &mut work).unwrap();
    let stale_ticket = arena.prepare_put(2, &second, &mut work).unwrap();
    let installed = ticket.handle;
    arena.put_prepared(ticket, second.clone()).unwrap();
    assert_eq!(
        arena.put_prepared(stale_ticket, first.clone()),
        Err(NativeValueError::StaleHandle)
    );
    assert!(
        arena
            .get(installed, &mut work)
            .unwrap()
            .shares_frozen_storage(&second)
    );
    let virtual_value = arena
        .define_virtual(2, ty.schema, ty.key, &mut work)
        .unwrap();
    arena.check_virtual(virtual_value, &mut work).unwrap();
    assert_eq!(
        arena.get(installed, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert_eq!(
        arena.get(virtual_value, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    let retry = arena.put(2, &second, &mut work).unwrap();
    assert_eq!(
        arena.check_virtual(virtual_value, &mut work),
        Err(NativeValueError::StaleHandle)
    );
    let first_retry = arena.put(1, &first, &mut work).unwrap();
    let mut outputs = [None, None];
    arena.publish_range(1, &mut outputs, &mut work).unwrap();
    assert!(outputs[0].as_ref().unwrap().shares_frozen_storage(&first));
    assert!(outputs[1].as_ref().unwrap().shares_frozen_storage(&second));
    assert!(
        arena
            .take(retry, &mut work)
            .unwrap()
            .shares_frozen_storage(&second)
    );
    assert_eq!(
        arena.get(retry, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    let before = work.used();
    assert_eq!(
        arena.copy_current_to(1, 3, &mut work).unwrap_err(),
        NativeValueError::InvalidSlot
    );
    assert_eq!(work.used(), before + 96);
    assert!(
        arena
            .get(first_retry, &mut work)
            .unwrap()
            .shares_frozen_storage(&first)
    );
    let index = arena.physical_index(1).unwrap();
    arena.generations.values_mut()[index] = u64::MAX;
    let last = NativeValueHandle {
        generation: u64::MAX,
        ..first_retry
    };
    let before = work.used();
    assert_eq!(
        arena.put(1, &second, &mut work).unwrap_err(),
        NativeValueError::GenerationExhausted
    );
    assert_eq!(work.used(), before + 32);
    assert_eq!(
        arena.retire_current(1, &mut work),
        Err(NativeValueError::GenerationExhausted)
    );
    assert!(
        arena
            .get(last, &mut work)
            .unwrap()
            .shares_frozen_storage(&first)
    );
    drop((outputs, foreign, arena, first, second, work));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}

#[test]
fn dense_worker_omitted_mutations_keep_ticket_and_capture_checks_before_closed_failure() {
    let slots = map();
    let memory = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = text("kept root");
    let owned = admit(&raw, &memory);
    let types = [ty; 7];
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let mut arena = NativeValueArena::new_with_worker_slots(
        &types,
        Some(&slots),
        TRANSPORT_LIMIT,
        &memory,
        &mut work,
    )
    .unwrap();
    let before = work.used();
    let ticket = arena.prepare_put(3, &owned, &mut work).unwrap();
    assert!(work.used() > before);
    let paid = work.used();
    assert_eq!(
        arena.put_prepared(ticket, owned.clone()),
        Err(NativeValueError::InvalidSlot)
    );
    assert_eq!(work.used(), paid);
    assert!(arena.current(3).unwrap().is_none());
    assert_eq!(arena.generation(3).unwrap(), 0);
    let before = work.used();
    assert_eq!(
        arena
            .define_virtual(3, ty.schema, ty.key, &mut work)
            .unwrap_err(),
        NativeValueError::InvalidSlot
    );
    assert_eq!(work.used(), before + 96);
    let before = work.used();
    assert_eq!(
        arena
            .put_owned_worker_capture(3, owned.clone(), &mut work)
            .unwrap_err(),
        NativeValueError::InvalidSlot
    );
    assert_eq!(
        work.used(),
        before + owned.memory_budget_retained_bytes(&memory).unwrap() * 4 + 64
    );
    assert!(arena.current(3).unwrap().is_none());
    assert_eq!(arena.generation(3).unwrap(), 0);
    let ticket = arena.prepare_put(2, &owned, &mut work).unwrap();
    let mut foreign = NativeValueArena::new_with_worker_slots(
        &types,
        Some(&slots),
        TRANSPORT_LIMIT,
        &memory,
        &mut work,
    )
    .unwrap();
    assert_eq!(
        foreign.put_prepared(ticket, owned.clone()),
        Err(NativeValueError::StaleHandle)
    );
    let retry = arena
        .put_owned_worker_capture(2, owned.clone(), &mut work)
        .unwrap();
    assert_eq!(retry.slot, 2);
    assert!(
        arena
            .get(retry, &mut work)
            .unwrap()
            .shares_frozen_storage(&owned)
    );
    drop((foreign, arena, owned, work));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}

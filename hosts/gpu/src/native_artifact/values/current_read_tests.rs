//! Current borrowing preserves the original copy read's price and failures.
use super::*;

#[test]
fn native_runtime_consolidation_current_read_preserves_funding_identity_and_fresh_owners() {
    let memory = ManagedMemoryBudget::new(1_000_000);
    let (ty, raw) = value(SchemaBody::Id, ValueDataDraft::Id(7));
    let first = admit(&raw, &memory);
    let (_, raw_next) = value(SchemaBody::Id, ValueDataDraft::Id(11));
    let next = admit(&raw_next, &memory);
    let mut work = NativeWorkMeter::new(1_000_000).unwrap();
    let types = [ty, ty];
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    arena.put(0, &first, &mut work).unwrap();
    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
    for expected in [&first, &first] {
        let before = work.used();
        let borrowed = arena.read_current(0, &mut work).unwrap();
        assert!(std::ptr::eq(borrowed, arena.test_current_value(0).unwrap()));
        assert!(borrowed.shares_frozen_storage(expected));
        assert_eq!(work.used() - before, 32);
        let before = work.used();
        let copy = arena.copy_current(0, &mut work).unwrap();
        assert!(copy.shares_frozen_storage(expected));
        assert_eq!(work.used() - before, 32);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            retained
        );
    }
    let mut foreign = NativeWorkMeter::new(1_000_000).unwrap();
    assert_eq!(
        arena.read_current(0, &mut foreign).unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    assert_eq!(foreign.used(), 0);
    for slot in [1, 2, u32::MAX] {
        let before = work.used();
        assert_eq!(
            arena.read_current(slot, &mut work).unwrap_err(),
            NativeValueError::StaleHandle
        );
        assert_eq!(work.used() - before, 32);
    }
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 1_000_000, &memory)
        .unwrap();
    let before = work.used();
    let denied = library
        .with_call_limits(31, 0, || arena.read_current(0, &mut work))
        .unwrap();
    assert!(matches!(
        denied,
        Err(NativeValueError::Base(
            NativeExecutionError::WorkLimit { .. }
        ))
    ));
    assert_eq!(work.used(), before);
    let borrowed = library
        .with_call_limits(32, 0, || arena.read_current(0, &mut work))
        .unwrap()
        .unwrap();
    assert!(borrowed.shares_frozen_storage(&first));
    assert_eq!(work.used() - before, 32);
    arena.put(0, &next, &mut work).unwrap();
    assert!(
        arena
            .read_current(0, &mut work)
            .unwrap()
            .shares_frozen_storage(&next)
    );
    assert_eq!(first.canonical_data_draft().unwrap(), ValueDataDraft::Id(7));
    arena.retire_current(0, &mut work).unwrap();
    let before = work.used();
    assert_eq!(
        arena.read_current(0, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert_eq!(work.used() - before, 32);
    arena.put(0, &first, &mut work).unwrap();
    assert!(
        arena
            .read_current(0, &mut work)
            .unwrap()
            .shares_frozen_storage(&first)
    );
    drop((arena, first, next, work, foreign, library));
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}

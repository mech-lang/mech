//! Copy only current same-arena owners whose complete cleanup is already paid.
use super::*;

#[test]
fn copies_preserve_owners_generations_and_cleanup_after_seed_retirement() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let (ty, first) = text("original λ");
    let (_, second) = text("replacement");
    let first = admit(&first, &memory);
    let second = admit(&second, &memory);
    let alias = first.clone();
    let types = [ty; 3];
    let mut work = NativeWorkMeter::new(1024 * 1024).unwrap();
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    let source = arena.put(0, &first, &mut work).unwrap();
    let old = arena.put(1, &second, &mut work).unwrap();
    let bytes = memory.used_bytes();
    let before = work.used();
    let copy = arena.copy_current_to(0, 1, &mut work).unwrap();
    assert_eq!(work.used() - before, 96);
    assert_eq!(memory.used_bytes(), bytes);
    assert!(
        arena
            .get(source, &mut work)
            .unwrap()
            .shares_frozen_storage(&alias)
    );
    assert!(
        arena
            .get(copy, &mut work)
            .unwrap()
            .shares_frozen_storage(&alias)
    );
    assert_eq!(
        arena.get(old, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    let chained = arena.copy_current_to(1, 2, &mut work).unwrap();
    arena.put(0, &second, &mut work).unwrap();
    arena.retire_current(1, &mut work).unwrap();
    drop((first, second));
    assert!(
        arena
            .get(chained, &mut work)
            .unwrap()
            .shares_frozen_storage(&alias)
    );
    let renewed = arena.copy_current_to(2, 2, &mut work).unwrap();
    assert_eq!(
        arena.get(chained, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    assert!(
        arena
            .get(renewed, &mut work)
            .unwrap()
            .shares_frozen_storage(&alias)
    );
    drop((arena, alias, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn copy_denials_preserve_both_slots_and_recover_on_the_same_account() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let (ty, first) = text("first");
    let (_, second) = text("second");
    let (other, _) = value(
        SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64),
        ValueDataDraft::U64(7),
    );
    let first = admit(&first, &memory);
    let second = admit(&second, &memory);
    let types = [ty, ty, other, ty];
    let mut work = NativeWorkMeter::new(1024 * 1024).unwrap();
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    let source = arena.put(0, &first, &mut work).unwrap();
    let target = arena.put(1, &second, &mut work).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 1024 * 1024, &memory)
        .unwrap();
    let bytes = memory.used_bytes();
    let before = work.used();
    let denied = library
        .with_call_limits(95, 0, || arena.copy_current_to(0, 1, &mut work))
        .unwrap();
    assert!(matches!(
        denied,
        Err(NativeValueError::Base(
            NativeExecutionError::WorkLimit { .. }
        ))
    ));
    assert_eq!(
        work.used(),
        before + 64,
        "the first two debits remain spent"
    );
    assert_eq!(memory.used_bytes(), bytes);
    assert!(
        arena
            .get(source, &mut work)
            .unwrap()
            .shares_frozen_storage(&first)
    );
    assert!(
        arena
            .get(target, &mut work)
            .unwrap()
            .shares_frozen_storage(&second)
    );
    for (source, target, expected) in [
        (0, 2, NativeValueError::TypeMismatch),
        (0, 4, NativeValueError::InvalidSlot),
        (4, 1, NativeValueError::InvalidSlot),
        (3, 1, NativeValueError::StaleHandle),
    ] {
        assert_eq!(
            arena
                .copy_current_to(source, target, &mut work)
                .unwrap_err(),
            expected
        );
    }
    let mut foreign = NativeWorkMeter::new(1024 * 1024).unwrap();
    assert_eq!(
        arena.copy_current_to(0, 1, &mut foreign).unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    assert_eq!(foreign.used(), 0);
    let before = work.used();
    let copied = library
        .with_call_limits(96, 0, || arena.copy_current_to(0, 1, &mut work))
        .unwrap()
        .unwrap();
    assert_eq!(work.used(), before + 96);
    assert!(
        arena
            .get(copied, &mut work)
            .unwrap()
            .shares_frozen_storage(&first)
    );
    arena.generations.values_mut()[1] = u64::MAX;
    assert_eq!(
        arena.copy_current_to(0, 1, &mut work).unwrap_err(),
        NativeValueError::GenerationExhausted
    );
    assert_eq!(arena.generations.values()[1], u64::MAX);
    assert!(
        arena.values.values()[1]
            .as_ref()
            .unwrap()
            .shares_frozen_storage(&first)
    );
    assert_eq!(memory.used_bytes(), bytes);
    drop((arena, first, second, work, library, foreign));
    assert_eq!(memory.used_bytes(), 0);
}

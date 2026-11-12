//! Real parent-backed accounts and original put prices, not detached fuel.
use super::*;
use mech_engine::native_artifact::NativeLibraryWorkBudget;

#[derive(Debug, Eq, PartialEq)]
struct Trial {
    outcome: Result<(), NativeValueError>,
    used: u64,
    generation: u64,
    present: bool,
    batch: Option<(bool, bool, u64, u64)>,
}

fn fresh_trial(
    selected: bool,
    allowance: u64,
    active: bool,
    exhaust_memory: bool,
    peer_spend: u64,
) -> Trial {
    let memory = ManagedMemoryBudget::new(1 << 20);
    let (ty, raw) = text("unchanged immutable capture");
    let owner = admit(&raw, &memory);
    let identity = owner.frozen_storage_identity();
    let retirement = owner.memory_budget_retained_bytes(&memory).unwrap() * 4 + 32;
    let setup = 24 + 256;
    let cap = setup + allowance;
    let root = NativeLibraryWorkBudget::new(cap, 1, &memory).unwrap();
    let group = root.reserve_worker_group(cap, 1).unwrap();
    let library = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let mut work = NativeWorkMeter::from_worker_budget(library.clone(), &memory).unwrap();
    let types = [ty];
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    assert_eq!(work.used(), setup);
    let peer = (peer_spend != 0).then(|| {
        let peer = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
        peer.charge(peer_spend, 0).unwrap();
        peer
    });
    let padding = exhaust_memory.then(|| {
        memory
            .reserve_capacity(memory.limit_bytes() - memory.used_bytes())
            .unwrap()
    });
    let before = memory.used_bytes();
    OWNED_CAPTURE_BATCH_END.with(|seen| seen.set(None));
    let mut owner = Some(owner);
    let mut install = || {
        if selected {
            arena.put_owned_worker_capture(0, owner.take().unwrap(), &mut work)
        } else {
            let outcome = arena.put(0, owner.as_ref().unwrap(), &mut work);
            drop(owner.take());
            outcome
        }
    };
    let outcome = if active {
        library.with_scope(install).unwrap()
    } else {
        install()
    };
    let batch = OWNED_CAPTURE_BATCH_END.with(|seen| seen.take());
    if let Some((retained, scope, used, bytes)) = batch {
        assert!(
            !scope,
            "constructor batch must restore inactive TLS before any owner drops"
        );
        assert_eq!(used, work.used());
        assert_eq!(
            bytes, before,
            "owned input stays live until after the batch returns"
        );
        assert_eq!(retained, outcome.is_err());
    }
    if let Ok(handle) = outcome.as_ref() {
        assert_eq!(handle.slot, 0);
        assert_eq!(handle.generation, 1);
        assert!(identity.matches(arena.values.values()[0].as_ref().unwrap()));
        assert!(
            arena.values.values()[0]
                .as_ref()
                .unwrap()
                .shares_frozen_storage(&raw)
        );
    } else {
        assert!(arena.values.values()[0].is_none());
    }
    let trial = Trial {
        outcome: outcome.map(|_| ()),
        used: work.used(),
        generation: arena.generations.values()[0],
        present: arena.values.values()[0].is_some(),
        batch,
    };
    if allowance >= 32 + retirement + peer_spend {
        assert!(trial.outcome.is_ok());
    }
    drop((padding, arena, work, library, owner, peer));
    let actual = group.finish().unwrap();
    assert_eq!(actual, (trial.used + peer_spend, 0));
    assert_eq!(root.used(), actual);
    drop(root);
    assert_eq!(
        (
            memory.used_bytes(),
            memory.retained_nodes(),
            memory.retained_snapshot_nodes()
        ),
        (0, 0, 0)
    );
    trial
}

#[test]
fn owned_worker_capture_preserves_both_original_denials_and_exact_paid_prefix_without_allocation() {
    let memory = ManagedMemoryBudget::new(1 << 20);
    let (_, raw) = text("unchanged immutable capture");
    let owner = admit(&raw, &memory);
    let fee = owner.memory_budget_retained_bytes(&memory).unwrap() * 4 + 64;
    drop(owner);
    for allowance in [0, 31, 32, 33, fee - 1, fee, fee + 1] {
        for exhaust in [false, true] {
            let original = fresh_trial(false, allowance, false, exhaust, 0);
            let selected = fresh_trial(true, allowance, false, exhaust, 0);
            assert!(
                selected.batch.is_some(),
                "must exercise actual worker batch at {allowance}"
            );
            assert_eq!(
                (
                    &selected.outcome,
                    selected.used,
                    selected.generation,
                    selected.present
                ),
                (
                    &original.outcome,
                    original.used,
                    original.generation,
                    original.present
                )
            );
        }
    }
    let active = fresh_trial(true, fee, true, false, 0);
    assert!(
        active.batch.is_none(),
        "active observers retain the ordinary ordered path"
    );
    assert_eq!(active.used, 24 + 256 + fee);
}

#[derive(Clone, Copy, Debug)]
enum Refusal {
    WrongSchema,
    ForeignOwner,
    InvalidSlot,
    Occupied,
    Retired,
    ExhaustedGeneration,
}

fn refused_trial(selected: bool, refusal: Refusal) -> Trial {
    let memory = ManagedMemoryBudget::new(1 << 20);
    let foreign = ManagedMemoryBudget::new(1 << 20);
    let (ty, raw) = text("capture");
    let root = NativeLibraryWorkBudget::new(10_000_000, 1, &memory).unwrap();
    let group = root.reserve_worker_group(10_000_000, 1).unwrap();
    let library = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let mut work = NativeWorkMeter::from_worker_budget(library.clone(), &memory).unwrap();
    let types = [ty];
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    let owner = match refusal {
        Refusal::WrongSchema => {
            let (_, wrong) = value(SchemaBody::Bool, ValueDataDraft::Bool(true));
            admit(&wrong, &memory)
        }
        Refusal::ForeignOwner => admit(&raw, &foreign),
        _ => admit(&raw, &memory),
    };
    let alias = owner.clone();
    let alias_before = alias.canonical_data_draft().unwrap();
    if matches!(refusal, Refusal::Occupied | Refusal::Retired) {
        arena.put(0, &alias, &mut work).unwrap();
        if matches!(refusal, Refusal::Retired) {
            arena.retire_current(0, &mut work).unwrap();
        }
    }
    if matches!(refusal, Refusal::ExhaustedGeneration) {
        arena.generations.values_mut()[0] = u64::MAX;
    }
    let slot = if matches!(refusal, Refusal::InvalidSlot) {
        u32::MAX
    } else {
        0
    };
    OWNED_CAPTURE_BATCH_END.with(|seen| seen.set(None));
    let outcome = if selected {
        arena.put_owned_worker_capture(slot, owner, &mut work)
    } else {
        let result = arena.put(slot, &owner, &mut work);
        drop(owner);
        result
    };
    let trial = Trial {
        outcome: outcome.map(|_| ()),
        used: work.used(),
        generation: arena.generations.values()[0],
        present: arena.values.values()[0].is_some(),
        batch: OWNED_CAPTURE_BATCH_END.with(|seen| seen.take()),
    };
    assert!(
        trial.batch.is_none(),
        "{refusal:?} must retain the original checked put"
    );
    assert_eq!(alias.canonical_data_draft().unwrap(), alias_before);
    drop((arena, work, library, alias));
    assert_eq!(group.finish().unwrap(), (trial.used, 0));
    drop(root);
    assert_eq!(
        (
            memory.used_bytes(),
            memory.retained_snapshot_nodes(),
            foreign.used_bytes(),
            foreign.retained_snapshot_nodes()
        ),
        (0, 0, 0, 0)
    );
    trial
}

#[test]
fn owned_worker_capture_declines_schema_slot_owner_and_generation_misses_without_reordering_checks()
{
    for refusal in [
        Refusal::WrongSchema,
        Refusal::ForeignOwner,
        Refusal::InvalidSlot,
        Refusal::Occupied,
        Refusal::Retired,
        Refusal::ExhaustedGeneration,
    ] {
        assert_eq!(
            refused_trial(true, refusal),
            refused_trial(false, refusal),
            "{refusal:?}"
        );
    }
}

#[test]
fn owned_worker_capture_rejects_foreign_work_before_any_debit_and_stale_handles_stay_dead() {
    let memory = ManagedMemoryBudget::new(1 << 20);
    let (ty, raw) = text("capture");
    let owner = admit(&raw, &memory);
    let root = NativeLibraryWorkBudget::new(10_000_000, 1, &memory).unwrap();
    let group = root.reserve_worker_group(10_000_000, 1).unwrap();
    let first = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let second = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let mut work = NativeWorkMeter::from_worker_budget(first.clone(), &memory).unwrap();
    let mut foreign = NativeWorkMeter::from_worker_budget(second.clone(), &memory).unwrap();
    let types = [ty];
    let mut arena = NativeValueArena::new(&types, TRANSPORT_LIMIT, &memory, &mut work).unwrap();
    assert_eq!(
        arena
            .put_owned_worker_capture(0, owner.clone(), &mut foreign)
            .unwrap_err(),
        NativeValueError::WrongWorkLedger
    );
    assert_eq!(foreign.used(), 0);
    let handle = arena
        .put_owned_worker_capture(0, owner.clone(), &mut work)
        .unwrap();
    arena.retire_current(0, &mut work).unwrap();
    assert_eq!(
        arena.get(handle, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    arena.put_owned_worker_capture(0, owner, &mut work).unwrap();
    assert_eq!(
        arena.get(handle, &mut work).unwrap_err(),
        NativeValueError::StaleHandle
    );
    let used = work.used();
    drop((arena, work, foreign, first, second));
    assert_eq!(group.finish().unwrap(), (used, 0));
    drop(root);
    assert_eq!(
        (memory.used_bytes(), memory.retained_snapshot_nodes()),
        (0, 0)
    );
}

#[test]
fn owned_worker_capture_keeps_shared_peer_denial_prices_and_reconciliation() {
    let memory = ManagedMemoryBudget::new(1 << 20);
    let (_, raw) = text("unchanged immutable capture");
    let owner = admit(&raw, &memory);
    let fee = owner.memory_budget_retained_bytes(&memory).unwrap() * 4 + 64;
    drop(owner);
    for allowance in [31, 49, fee + 16, fee + 17] {
        let ordinary = fresh_trial(false, allowance, false, false, 17);
        let selected = fresh_trial(true, allowance, false, false, 17);
        assert!(selected.batch.is_some());
        assert_eq!(
            (
                &selected.outcome,
                selected.used,
                selected.generation,
                selected.present
            ),
            (
                &ordinary.outcome,
                ordinary.used,
                ordinary.generation,
                ordinary.present
            )
        );
    }
}

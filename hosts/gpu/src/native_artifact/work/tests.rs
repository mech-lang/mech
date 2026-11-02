use crate::native_artifact::work::*;

#[test]
fn worker_meter_attaches_without_root_renewal_or_promotion_and_reconciles_to_parent() {
    let memory = ManagedMemoryBudget::new(8192);
    let mut parent = NativeWorkMeter::new(1000).unwrap();
    let library = parent
        .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
        .unwrap();
    let group = library.reserve_worker_group(100, 20).unwrap();
    let held = memory.used_bytes();
    let exact = held + NativeLibraryWorkBudget::worker_account_metadata_bytes();
    assert!(exact < memory.limit_bytes());
    assert!(
        group
            .worker_loan_with_memory(&memory.with_limits(exact - 1, 100))
            .is_err()
    );
    assert_eq!(memory.used_bytes(), held);
    let capped = memory.with_limits(exact, 100);
    let loan = group.worker_loan_with_memory(&capped).unwrap();
    let worker_memory = capped.clone();
    let used = std::thread::spawn(move || {
        let local = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
        let clone = local.clone();
        let mut worker = NativeWorkMeter::from_worker_budget(local, &worker_memory).unwrap();
        assert_eq!(worker.used(), 0);
        assert_eq!(worker.limit(), 100);
        worker.charge(NativeExecutionPhase::Setup, 7).unwrap();
        clone.charge(13, 2).unwrap();
        assert_eq!(worker.used(), 20);
        assert!(worker.charge(NativeExecutionPhase::Completion, 81).is_err());
        worker.used()
    })
    .join()
    .unwrap();
    assert_eq!(used, 20);
    assert_eq!(group.finish().unwrap(), (20, 2));
    assert_eq!(parent.used(), 84);
}

#[test]
fn worker_meter_rejects_root_accounts_and_foreign_memory() {
    let memory = ManagedMemoryBudget::new(8192);
    let root = NativeLibraryWorkBudget::new(100, 20, &memory).unwrap();
    assert!(NativeWorkMeter::from_worker_budget(root.clone(), &memory).is_err());
    let group = root.reserve_worker_group(100, 20).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    assert!(NativeWorkMeter::from_worker_budget(worker, &ManagedMemoryBudget::new(8192)).is_err());
    assert_eq!(group.finish().unwrap(), (0, 0));
}

#[test]
fn closed_tail_fuel_observes_the_same_spent_account_and_declines_nested_scopes() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut work = NativeWorkMeter::new(1000).unwrap();
    work.charge(NativeExecutionPhase::Setup, 7).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
        .unwrap();
    assert_eq!(library.unscoped_remaining(), Some((929, 100)));
    library.charge(29, 3).unwrap();
    assert_eq!(library.clone().unscoped_remaining(), Some((900, 97)));
    library
        .with_scope(|| {
            assert_eq!(library.unscoped_remaining(), None);
            work.charge(NativeExecutionPhase::Completion, 10).unwrap();
        })
        .unwrap();
    assert_eq!(library.unscoped_remaining(), Some((890, 97)));
    library.restrict(500, 50).unwrap();
    assert_eq!(library.unscoped_remaining(), Some((390, 47)));
    assert_eq!(work.used(), 110);
}

#[test]
fn repeated_phases_share_one_limit_and_denial_does_not_reset_it() {
    let mut work = NativeWorkMeter::new(100).unwrap();
    work.charge(NativeExecutionPhase::Setup, 35).unwrap();
    work.charge(NativeExecutionPhase::Node(mech_core::NodeId::new(9)), 40)
        .unwrap();
    assert_eq!(work.used(), 75);
    assert_eq!(
        work.charge(NativeExecutionPhase::Completion, 26),
        Err(NativeExecutionError::WorkLimit {
            phase: NativeExecutionPhase::Completion,
            required: 101,
            limit: 100
        })
    );
    assert_eq!(work.used(), 75);
    work.charge(NativeExecutionPhase::Completion, 25).unwrap();
    assert_eq!(work.used(), 100);
}

#[test]
fn cumulative_overflow_is_a_denial_without_wrapping_the_ledger() {
    let mut work = NativeWorkMeter::new(u64::MAX).unwrap();
    work.charge(NativeExecutionPhase::Setup, u64::MAX - 1)
        .unwrap();
    assert_eq!(
        work.charge(NativeExecutionPhase::Completion, 2),
        Err(NativeExecutionError::WorkLimit {
            phase: NativeExecutionPhase::Completion,
            required: u64::MAX,
            limit: u64::MAX
        })
    );
    assert_eq!(work.used(), u64::MAX - 1);
}

#[test]
fn promotion_preserves_scalar_work_and_nested_library_work_is_immediately_visible() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut work = NativeWorkMeter::new(100).unwrap();
    let identity = work.identity();
    work.charge(NativeExecutionPhase::Setup, 7).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
        .unwrap();
    assert_eq!(work.identity(), identity);
    assert_eq!(library.used(), (71, 0));
    let backing = memory.used_bytes();
    assert!(backing > 0);
    library
        .with_scope(|| {
            library.charge(20, 3).unwrap();
            assert_eq!(work.used(), 91);
            work.charge(NativeExecutionPhase::Completion, 9).unwrap();
        })
        .unwrap();
    assert_eq!(library.used(), (100, 3));
    assert_eq!(work.used(), 100);
    assert!(matches!(
        work.charge(NativeExecutionPhase::Completion, 1),
        Err(NativeExecutionError::WorkLimit {
            required: 101,
            limit: 100,
            ..
        })
    ));
    assert!(library.charge(0, 2).is_err());
    assert_eq!(library.used(), (100, 3));
    drop(work);
    assert_eq!(memory.used_bytes(), backing);
    drop(library);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn repeated_library_access_cannot_refill_limits_or_change_memory_identity() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut work = NativeWorkMeter::new(200).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
        .unwrap();
    library.charge(5, 2).unwrap();
    let repeated = work
        .shared_library_budget(NativeExecutionPhase::Setup, 3, &memory.clone())
        .unwrap();
    assert_eq!(library.used(), (77, 2));
    assert_eq!(repeated.used(), (77, 2));
    assert!(
        work.shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
            .is_err()
    );
    assert!(
        work.shared_library_budget(
            NativeExecutionPhase::Setup,
            3,
            &ManagedMemoryBudget::new(4096)
        )
        .is_err()
    );
    assert_eq!(work.used(), 77);
    repeated.charge(1, 1).unwrap();
    assert_eq!(work.used(), 78);
    assert!(library.charge(0, 1).is_err());
    assert_eq!(library.used(), (78, 3));
}

#[test]
fn failed_repeated_promotion_preserves_existing_comparison_allowance() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut work = NativeWorkMeter::new(100).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
        .unwrap();
    work.charge(NativeExecutionPhase::Setup, 30).unwrap();
    let backing = memory.used_bytes();
    assert!(matches!(
        work.shared_library_budget(NativeExecutionPhase::Setup, 5, &memory),
        Err(NativeExecutionError::WorkLimit {
            required: 102,
            limit: 100,
            ..
        })
    ));
    assert_eq!(library.used(), (94, 0));
    assert_eq!(memory.used_bytes(), backing);
    library.charge(0, 10).unwrap();
    work.charge(NativeExecutionPhase::Completion, 6).unwrap();
    assert_eq!(library.used(), (100, 10));
}

#[test]
fn active_library_account_rejects_new_and_preexisting_independent_inline_meters() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut caller = NativeWorkMeter::new(1000).unwrap();
    let mut independent = NativeWorkMeter::new(1000).unwrap();
    let library = caller
        .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
        .unwrap();
    library
        .with_scope(|| {
            assert!(matches!(
                NativeWorkMeter::new(1000),
                Err(NativeExecutionError::Descriptor { .. })
            ));
            assert!(matches!(
                independent.charge(NativeExecutionPhase::Setup, 1),
                Err(NativeExecutionError::LibraryWork {
                    error: ResidentKernelError::InvalidInput,
                    ..
                })
            ));
            assert_eq!(independent.used(), 0);
            caller.charge(NativeExecutionPhase::Setup, 1).unwrap();
        })
        .unwrap();
    assert_eq!(library.used(), (65, 0));
    independent.charge(NativeExecutionPhase::Setup, 1).unwrap();
    assert_eq!(independent.used(), 1);
    assert!(NativeWorkMeter::new(1000).is_ok());
}

#[test]
fn promotion_denials_precede_allocation_and_never_refund_prior_work() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut denied = NativeWorkMeter::new(63).unwrap();
    assert!(matches!(
        denied.shared_library_budget(NativeExecutionPhase::Setup, 4, &memory),
        Err(NativeExecutionError::WorkLimit {
            required: 64,
            limit: 63,
            ..
        })
    ));
    assert_eq!(denied.used(), 0);
    assert_eq!(memory.used_bytes(), 0);
    let mut work = NativeWorkMeter::new(1000).unwrap();
    let empty = ManagedMemoryBudget::new(0);
    assert!(matches!(
        work.shared_library_budget(NativeExecutionPhase::Setup, 4, &empty),
        Err(NativeExecutionError::Memory { .. })
    ));
    assert_eq!(work.used(), 64);
    assert_eq!(empty.used_bytes(), 0);
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
        .unwrap();
    assert_eq!(library.used(), (128, 0));
}

#[test]
fn a_different_shared_ledger_cannot_replace_a_live_library_caller() {
    let memory = ManagedMemoryBudget::new(4096);
    let mut caller = NativeWorkMeter::new(1000).unwrap();
    let mut other = NativeWorkMeter::new(1000).unwrap();
    let caller_library = caller
        .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
        .unwrap();
    let other_library = other
        .shared_library_budget(NativeExecutionPhase::Setup, 4, &memory)
        .unwrap();
    caller_library
        .with_scope(|| {
            assert!(matches!(
                other.charge(NativeExecutionPhase::Completion, 1),
                Err(NativeExecutionError::LibraryWork {
                    error: ResidentKernelError::InvalidInput,
                    ..
                })
            ));
            assert_eq!(other_library.used(), (64, 0));
            caller.charge(NativeExecutionPhase::Completion, 1).unwrap();
        })
        .unwrap();
    assert_eq!(caller.used(), 65);
}

#[test]
fn native_ceiling_and_overflow_precede_foreign_account_and_library_caps() {
    let memory = ManagedMemoryBudget::new(8192);
    for ceiling in [100, u64::MAX] {
        let mut meter = NativeWorkMeter::new(ceiling).unwrap();
        let owner = meter
            .shared_library_budget(NativeExecutionPhase::Setup, 10, &memory)
            .unwrap();
        let foreign = NativeLibraryWorkBudget::new(1000, 10, &memory).unwrap();
        let amount = if ceiling == 100 { 37 } else { u64::MAX };
        foreign
            .with_scope(|| {
                assert_eq!(
                    meter.charge(NativeExecutionPhase::Completion, amount),
                    Err(NativeExecutionError::WorkLimit {
                        phase: NativeExecutionPhase::Completion,
                        required: if ceiling == 100 { 101 } else { u64::MAX },
                        limit: ceiling,
                    })
                );
                assert!(matches!(
                    meter.charge(NativeExecutionPhase::Completion, 1),
                    Err(NativeExecutionError::LibraryWork {
                        error: ResidentKernelError::InvalidInput,
                        ..
                    })
                ));
                assert_eq!(owner.used(), (64, 0));
                assert_eq!(foreign.used(), (0, 0));
            })
            .unwrap();
        owner.restrict(90, 10).unwrap();
        assert_eq!(
            meter.charge(NativeExecutionPhase::Completion, 27),
            Err(NativeExecutionError::WorkLimit {
                phase: NativeExecutionPhase::Completion,
                required: 91,
                limit: 90,
            })
        );
        assert_eq!(owner.used(), (64, 0));
        meter.charge(NativeExecutionPhase::Completion, 26).unwrap();
        assert_eq!(meter.used(), 90);
    }
    assert_eq!(memory.used_bytes(), 0);
}

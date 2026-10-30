use super::*;

fn root(
    compute: u64,
    comparison: u64,
) -> (NativeLibraryWorkBudget, mech_core::ManagedMemoryBudget) {
    let memory = mech_core::ManagedMemoryBudget::new(64 * 1024);
    (
        NativeLibraryWorkBudget::new(compute, comparison, &memory).unwrap(),
        memory,
    )
}

#[test]
fn worker_tokens_are_send_but_accounts_are_constructed_on_the_worker() {
    fn assert_send<T: Send>() {}
    assert_send::<NativeWorkerWorkLoan>();
    let (root, memory) = root(100, 20);
    root.charge(7, 2).unwrap();
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(80, 15).unwrap();
    let loan = group.worker_loan().unwrap();
    let used = std::thread::spawn(move || {
        let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
        worker.charge(19, 3).unwrap();
        let clone = worker.clone();
        clone.charge(11, 4).unwrap();
        assert_eq!(worker.used(), (30, 7));
        worker.used()
    })
    .join()
    .unwrap();
    assert_eq!(used, (30, 7));
    assert_eq!(root.used(), (7, 2)); // Escrow is reserved, not spent twice.
    assert_eq!(group.finish().unwrap(), (30, 7));
    assert_eq!(root.used(), (37, 9));
    assert_eq!(memory.used_bytes(), baseline);
    root.charge(63, 11).unwrap();
    assert!(root.charge(1, 0).is_err());
    drop(root);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn reservations_prevent_parent_spending_or_restricting_worker_fuel() {
    let (root, _) = root(100, 10);
    root.charge(10, 1).unwrap();
    let group = root.reserve_worker_group(80, 8).unwrap();
    assert_eq!(root.unscoped_remaining(), Some((10, 1)));
    assert!(root.charge(11, 0).is_err());
    assert!(root.charge(0, 2).is_err());
    assert!(root.restrict(89, 10).is_err());
    assert!(root.restrict(100, 8).is_err());
    assert!(root.reserve_worker_group(11, 0).is_err());
    root.charge(10, 1).unwrap();
    assert_eq!(group.finish().unwrap(), (0, 0));
    assert_eq!(root.used(), (20, 2));
    assert_eq!(root.unscoped_remaining(), Some((80, 8)));
}

#[test]
fn reservation_captures_enclosing_caps_and_denials_leave_no_claim() {
    let (root, memory) = root(100, 100);
    let baseline = memory.used_bytes();
    root.with_call_limits(7, 3, || {
        assert!(root.reserve_worker_group(8, 0).is_err());
        assert!(root.reserve_worker_group(0, 4).is_err());
        assert_eq!(memory.used_bytes(), baseline);
        let group = root.reserve_worker_group(7, 3).unwrap();
        let loan = group.worker_loan().unwrap();
        std::thread::spawn(move || {
            let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
            assert!(worker.charge(8, 0).is_err());
            worker.charge(7, 3).unwrap();
        })
        .join()
        .unwrap();
        group.finish().unwrap();
        assert!(root.charge(1, 0).is_err());
    })
    .unwrap();
    assert_eq!(root.used(), (7, 3));
    assert_eq!(memory.used_bytes(), baseline);
    root.charge(1, 0).unwrap();
}

#[test]
fn worker_call_caps_are_local_but_root_fuel_is_shared() {
    let (root, _) = root(20, 10);
    let group = root.reserve_worker_group(20, 10).unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = group.worker_loan().unwrap();
    left.with_call_limits(10, 5, || {
        left.charge(1, 1).unwrap();
        std::thread::spawn(move || {
            let right = NativeLibraryWorkBudget::from_worker_loan(right).unwrap();
            right
                .with_call_limits(10, 5, || right.charge(10, 5))
                .unwrap()
                .unwrap();
        })
        .join()
        .unwrap();
        // Another worker's ten units do not consume this call's nine remaining.
        left.charge(9, 4).unwrap();
        assert!(left.charge(1, 0).is_err());
    })
    .unwrap();
    assert_eq!(left.used(), (10, 5));
    assert_eq!(left.unscoped_remaining(), Some((0, 0)));
    drop(left);
    assert_eq!(group.finish().unwrap(), (20, 10));
    assert_eq!(root.used(), (20, 10));
}

#[test]
fn concurrent_two_dimension_debits_never_partially_spend_or_renew() {
    let (root, _) = root(28, 8);
    let group = root.reserve_worker_group(28, 8).unwrap();
    let threads = (0..8)
        .map(|_| {
            let loan = group.worker_loan().unwrap();
            std::thread::spawn(move || {
                let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
                let success = worker.charge(7, 2).is_ok();
                assert_eq!(worker.used(), if success { (7, 2) } else { (0, 0) });
                success
            })
        })
        .collect::<Vec<_>>();
    let successes = threads
        .into_iter()
        .map(|thread| usize::from(thread.join().unwrap()))
        .sum::<usize>();
    assert_eq!(successes, 4);
    assert_eq!(group.finish().unwrap(), (28, 8));
    assert!(root.reserve_worker_group(1, 0).is_err());
}

#[test]
fn a_comparison_denial_does_not_spend_compute_in_either_ledger() {
    let (root, _) = root(50, 3);
    let group = root.reserve_worker_group(50, 3).unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    left.charge(5, 3).unwrap();
    assert!(right.charge(40, 1).is_err());
    assert_eq!(right.used(), (0, 0));
    right.charge(45, 0).unwrap();
    drop((left, right));
    assert_eq!(group.finish().unwrap(), (50, 3));
}

#[test]
fn early_drop_seals_outstanding_workers_and_reconciles_successful_work() {
    let (root, memory) = root(100, 10);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(80, 8).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    worker.charge(9, 2).unwrap();
    let clone = worker.clone();
    // An alive clone is still the same live worker, so finish cannot publish.
    assert_eq!(group.finish(), Err(ResidentKernelError::InvalidInput));
    assert_eq!(root.used(), (9, 2));
    assert_eq!(worker.charge(1, 0), Err(ResidentKernelError::InvalidInput));
    assert_eq!(worker.unscoped_remaining(), None);
    root.charge(91, 8).unwrap();
    drop((worker, clone));
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
#[cfg(panic = "unwind")]
fn worker_panic_retires_local_scopes_without_refunding_work() {
    let (root, memory) = root(100, 10);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(80, 8).unwrap();
    let loan = group.worker_loan().unwrap();
    assert!(
        std::thread::spawn(move || {
            let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
            worker
                .with_call_limits(10, 3, || {
                    worker.charge(10, 3).unwrap();
                    panic!("owned worker unwind fixture");
                })
                .unwrap();
        })
        .join()
        .is_err()
    );
    assert_eq!(group.finish().unwrap(), (10, 3));
    assert_eq!(root.used(), (10, 3));
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
fn foreign_scopes_nested_groups_and_memory_rehoming_are_rejected() {
    let (foreign, other_memory) = root(100, 10);
    let (root, memory) = root(100, 10);
    assert!(
        foreign
            .with_scope(|| root.reserve_worker_group(1, 1))
            .unwrap()
            .is_err()
    );
    let group = root.reserve_worker_group(80, 8).unwrap();
    assert!(group.worker_loan_with_memory(&other_memory).is_err());
    let capped = memory.with_limits(32 * 1024, 100);
    let loan = group.worker_loan_with_memory(&capped).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
    assert_eq!(worker.memory_budget(), memory);
    assert_eq!(worker.memory_budget().limit_bytes(), 32 * 1024);
    assert_eq!(worker.memory_budget().limit_nodes(), 100);
    assert!(worker.reserve_worker_group(1, 1).is_err());
    let loan = group.worker_loan().unwrap();
    assert!(
        foreign
            .with_scope(|| NativeLibraryWorkBudget::from_worker_loan(loan))
            .unwrap()
            .is_err()
    );
    drop(worker);
    assert_eq!(group.finish().unwrap(), (0, 0));
}

#[test]
fn failed_metadata_admission_and_unused_tokens_release_all_claims() {
    let memory = mech_core::ManagedMemoryBudget::new(64 * 1024);
    let root = NativeLibraryWorkBudget::new(10, 10, &memory).unwrap();
    let original = memory.used_bytes();
    assert!(matches!(
        root.reserve_worker_group_with_memory(10, 10, &memory.with_limits(original, 100)),
        Err(NativeWorkerBudgetError::Memory(_))
    ));
    assert_eq!(root.used(), (0, 0));
    assert_eq!(root.unscoped_remaining(), Some((10, 10)));
    assert_eq!(memory.used_bytes(), original);
    let group = root.reserve_worker_group(10, 10).unwrap();
    let used = memory.used_bytes();
    assert!(matches!(
        group.worker_loan_with_memory(&memory.with_limits(used, 100)),
        Err(NativeWorkerBudgetError::Memory(_))
    ));
    assert_eq!(memory.used_bytes(), used);
    let token = group.worker_loan().unwrap();
    drop(token);
    assert_eq!(memory.used_bytes(), used);
    assert_eq!(group.finish().unwrap(), (0, 0));
    drop(root);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn multiple_reservations_and_maximum_fuel_do_not_wrap_or_double_reconcile() {
    let (root, _) = root(u64::MAX, u64::MAX);
    root.charge(1, 1).unwrap();
    let first = root
        .reserve_worker_group(u64::MAX - 2, u64::MAX - 2)
        .unwrap();
    assert!(root.reserve_worker_group(2, 0).is_err());
    let second = root.reserve_worker_group(1, 1).unwrap();
    assert!(root.charge(1, 0).is_err());
    let worker = NativeLibraryWorkBudget::from_worker_loan(second.worker_loan().unwrap()).unwrap();
    worker.charge(1, 1).unwrap();
    drop(worker);
    assert_eq!(second.finish().unwrap(), (1, 1));
    assert_eq!(root.used(), (2, 2));
    drop(first);
    assert_eq!(
        root.unscoped_remaining(),
        Some((u64::MAX - 2, u64::MAX - 2))
    );
}

#[test]
fn unscoped_debit_matches_separate_queries_with_reservations_and_tight_limits() {
    for (compute, comparison) in [(100, 60), (7, 0), (0, 7), (u64::MAX, u64::MAX)] {
        let (candidate, _) = root(compute, comparison);
        let (reference, _) = root(compute, comparison);
        for work in [&candidate, &reference] {
            work.charge(compute / 16, comparison / 16).unwrap();
        }
        let candidate_group = candidate
            .reserve_worker_group(compute / 4, comparison / 4)
            .unwrap();
        let reference_group = reference
            .reserve_worker_group(compute / 4, comparison / 4)
            .unwrap();
        for _ in 0..5 {
            let prepare = |remaining: (u64, u64)| {
                let count = (remaining.0 / 5 / 4)
                    .min(remaining.1 / 2 / 4)
                    .min(1024)
                    .max(1);
                Ok::<_, ResidentKernelError>((count * 5, count * 2, count))
            };
            let remaining = reference.unscoped_remaining().unwrap();
            let expected = prepare(remaining).and_then(|(compute, comparison, count)| {
                reference.charge(compute, comparison).map(|()| Some(count))
            });
            assert_eq!(candidate.with_unscoped_debit(prepare), expected);
            assert_eq!(candidate.used(), reference.used());
            assert_eq!(
                candidate.unscoped_remaining(),
                reference.unscoped_remaining()
            );
        }
        assert_eq!(candidate_group.finish().unwrap(), (0, 0));
        assert_eq!(reference_group.finish().unwrap(), (0, 0));
    }
}

#[test]
fn unscoped_debit_rejects_active_scopes_and_respects_root_restrictions() {
    let (work, _) = root(100, 30);
    let (foreign, _) = root(100, 30);
    let forbidden = |_| -> Result<(u64, u64, ()), ResidentKernelError> {
        panic!("a scoped transaction must not invoke its preparer")
    };
    assert_eq!(
        work.with_scope(|| work.with_unscoped_debit(forbidden))
            .unwrap(),
        Ok(None)
    );
    assert_eq!(
        foreign
            .with_scope(|| work.with_unscoped_debit(forbidden))
            .unwrap(),
        Ok(None)
    );
    work.with_call_limits(5, 2, || {
        assert_eq!(work.with_unscoped_debit(forbidden), Ok(None));
    })
    .unwrap();
    struct RootGuard(Option<(u64, u64)>);
    impl Drop for RootGuard {
        fn drop(&mut self) {
            with_control_root_limits(|root| root.set(self.0));
        }
    }
    let _guard = RootGuard(with_control_root_limits(|root| root.replace(Some((8, 3)))));
    let before = work.used();
    assert!(
        work.with_unscoped_debit(|remaining| {
            assert_eq!(remaining, (8, 3));
            Ok::<_, ResidentKernelError>((8, 4, ()))
        })
        .is_err()
    );
    assert_eq!(work.used(), before);
    assert_eq!(
        work.with_unscoped_debit(|remaining| {
            assert_eq!(remaining, (8, 3));
            Ok::<_, ResidentKernelError>((8, 3, 17))
        })
        .unwrap(),
        Some(17)
    );
    assert_eq!(work.used(), (8, 3));
    assert!(
        work.with_unscoped_debit(|remaining| {
            assert_eq!(remaining, (0, 0));
            Ok::<_, ResidentKernelError>((1, 0, ()))
        })
        .is_err()
    );
    assert_eq!(work.used(), (8, 3));
}

#[test]
fn unscoped_debit_concurrent_workers_commit_both_dimensions_once() {
    let (root, memory) = root(28, 8);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(28, 8).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let loan = group.worker_loan().unwrap();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
                barrier.wait();
                let accepted = worker
                    .with_unscoped_debit(|remaining| {
                        assert_eq!(remaining.0, remaining.1 / 2 * 7);
                        Ok::<_, ResidentKernelError>((7, 2, ()))
                    })
                    .is_ok();
                assert_eq!(worker.used(), if accepted { (7, 2) } else { (0, 0) });
                accepted
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handles
            .into_iter()
            .map(|handle| usize::from(handle.join().unwrap()))
            .sum::<usize>(),
        4
    );
    assert_eq!(group.finish().unwrap(), (28, 8));
    assert_eq!(root.used(), (28, 8));
    assert_eq!(memory.used_bytes(), baseline);
    assert!(root.charge(1, 0).is_err());
}

#[test]
fn unscoped_debit_errors_and_sealing_never_partially_commit_or_refund() {
    let (root, memory) = root(50, 3);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(50, 3).unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    left.charge(5, 3).unwrap();
    let error = ResidentKernelError::InvalidShape;
    assert_eq!(
        right.with_unscoped_debit(|remaining| -> Result<(u64, u64, ()), _> {
            assert_eq!(remaining, (45, 0));
            Err(error.clone())
        }),
        Err(error)
    );
    assert_eq!(right.used(), (0, 0));
    assert!(
        right
            .with_unscoped_debit(|_| Ok::<_, ResidentKernelError>((40, 1, ())))
            .is_err()
    );
    assert_eq!(right.used(), (0, 0));
    assert_eq!(
        right
            .with_unscoped_debit(|remaining| {
                assert_eq!(remaining, (45, 0));
                Ok::<_, ResidentKernelError>((45, 0, ()))
            })
            .unwrap(),
        Some(())
    );
    assert_eq!(right.used(), (45, 0));
    assert_eq!(group.finish(), Err(ResidentKernelError::InvalidInput));
    assert_eq!(root.used(), (50, 3));
    assert_eq!(
        right.with_unscoped_debit(|_| -> Result<(u64, u64, ()), ResidentKernelError> {
            panic!("a sealed group must not invoke its preparer")
        }),
        Ok(None)
    );
    drop((left, right));
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
fn unscoped_debit_keeps_worker_local_limits_below_shared_fuel() {
    let (root, _) = root(30, 10);
    let group = root.reserve_worker_group(30, 10).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let peer = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    worker.restrict(9, 3).unwrap();
    peer.charge(5, 1).unwrap();
    assert!(
        worker
            .with_unscoped_debit(|remaining| {
                assert_eq!(remaining, (9, 3));
                Ok::<_, ResidentKernelError>((10, 2, ()))
            })
            .is_err()
    );
    assert_eq!(worker.used(), (0, 0));
    worker
        .with_unscoped_debit(|remaining| {
            assert_eq!(remaining, (9, 3));
            Ok::<_, ResidentKernelError>((9, 3, ()))
        })
        .unwrap()
        .unwrap();
    peer.charge(16, 6).unwrap();
    drop((worker, peer));
    assert_eq!(group.finish().unwrap(), (30, 10));
    assert_eq!(root.used(), (30, 10));
}

#[test]
#[cfg(panic = "unwind")]
fn unscoped_debit_unwind_releases_the_mutex_and_keeps_original_work() {
    let (root, _) = root(30, 8);
    let group = root.reserve_worker_group(30, 8).unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    left.charge(3, 1).unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<Option<()>, ResidentKernelError> =
                left.with_unscoped_debit(|_| panic!("owned unscoped debit preparation unwind"));
        }))
        .is_err()
    );
    assert_eq!(left.used(), (3, 1));
    ESCROW_LOCK_STATS.with(|v| v.set((0, 0, 0, 0)));
    right
        .with_unscoped_debit(|remaining| {
            assert_eq!(remaining, (27, 7));
            Ok::<_, ResidentKernelError>((27, 7, ()))
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        ESCROW_LOCK_STATS.with(std::cell::Cell::get),
        (1, 0, 0, 1),
        "poison recovery must not spin or discard previous debits"
    );
    drop((left, right));
    assert_eq!(group.finish().unwrap(), (30, 8));
    assert_eq!(root.used(), (30, 8));
}

#[test]
fn escrow_spin_uncontended_debit_commits_once_without_pause_or_park() {
    let (root, memory) = root(20, 8);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(20, 8).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    ESCROW_LOCK_STATS.with(|v| v.set((0, 0, 0, 0)));
    assert_eq!(
        worker
            .with_unscoped_debit(|remaining| {
                assert_eq!(remaining, (20, 8));
                Ok::<_, ResidentKernelError>((7, 3, 41))
            })
            .unwrap(),
        Some(41)
    );
    assert_eq!(ESCROW_LOCK_STATS.with(std::cell::Cell::get), (1, 0, 0, 0));
    assert_eq!(worker.used(), (7, 3));
    drop(worker);
    assert_eq!(group.finish().unwrap(), (7, 3));
    assert_eq!(root.used(), (7, 3));
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
fn escrow_spin_is_bounded_then_uses_original_mutex_without_partial_debits() {
    use std::sync::mpsc::channel;
    use std::time::Duration;
    let (root, memory) = root(20, 8);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(20, 8).unwrap();
    let shared = group.shared.clone();
    let loan = group.worker_loan().unwrap();
    let (ready_tx, ready_rx) = channel();
    let (start_tx, start_rx) = channel();
    let (park_tx, park_rx) = channel();
    let handle = std::thread::spawn(move || {
        let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
        ready_tx.send(()).unwrap();
        start_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        ESCROW_LOCK_STATS.with(|v| v.set((0, 0, 0, 0)));
        ESCROW_PARK_NOTIFY.with(|v| *v.borrow_mut() = Some(park_tx));
        let result = worker.with_unscoped_debit(|remaining| {
            assert_eq!(remaining, (20, 8));
            Ok::<_, ResidentKernelError>((7, 3, 41))
        });
        let stats = ESCROW_LOCK_STATS.with(std::cell::Cell::get);
        (result, worker.used(), stats)
    });
    ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    // Keep the real escrow unavailable through every bounded retry. The hook
    // observes entry into the fallback, rather than depending on sleep timing.
    let held = shared.lock().unwrap();
    start_tx.send(()).unwrap();
    let parked = park_rx.recv_timeout(Duration::from_secs(2));
    assert_eq!((held.used.compute, held.used.comparison), (0, 0));
    drop(held);
    assert!(
        parked.is_ok(),
        "bounded retries must reach the blocking fallback"
    );
    let (result, used, (attempts, hints, parks, poison)) = handle.join().unwrap();
    assert_eq!(result, Ok(Some(41)));
    assert_eq!(used, (7, 3));
    assert_eq!(attempts, ESCROW_SPIN_ROUNDS);
    assert_eq!(hints, (1 << ESCROW_SPIN_ROUNDS) - 1);
    assert_eq!((parks, poison), (1, 0));
    assert_eq!(group.finish().unwrap(), (7, 3));
    assert_eq!(root.used(), (7, 3));
    drop(shared);
    assert_eq!(memory.used_bytes(), baseline);
}

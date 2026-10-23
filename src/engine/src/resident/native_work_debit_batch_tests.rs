use super::*;

fn root(compute: u64, comparison: u64) -> (NativeLibraryWorkBudget, mech_core::ManagedMemoryBudget) {
    let memory = mech_core::ManagedMemoryBudget::new(64 * 1024);
    (NativeLibraryWorkBudget::new(compute, comparison, &memory).unwrap(), memory)
}

#[test]
fn batches_keep_exact_local_call_and_shared_two_dimension_denials() {
    let (root, memory) = root(100, 10);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(100, 10).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let peer = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    peer.charge(10, 5).unwrap();
    worker.with_call_limits(9, 9, || worker.with_debit_batch(|batch| {
        batch.charge(7, 3).unwrap();
        // A local compute denial must not spend comparison in either ledger.
        assert!(matches!(batch.charge(3, 1), Err(ResidentKernelError::ResourceLimit {
            dimension: MemoryBudgetDimension::ComputeWork, required: 10, limit: 9, ..
        })));
        // The local call fits; the shared comparison allowance does not.
        assert!(matches!(batch.charge(1, 3), Err(ResidentKernelError::ResourceLimit {
            dimension: MemoryBudgetDimension::ComparisonWork, required: 11, limit: 10, ..
        })));
        batch.charge(2, 2).unwrap();
        assert!(matches!(batch.charge(1, 0), Err(ResidentKernelError::ResourceLimit {
            dimension: MemoryBudgetDimension::ComputeWork, ..
        })));
    }).unwrap()).unwrap();
    assert_eq!(worker.used(), (9, 5));
    assert_eq!(peer.used(), (10, 5));
    // No unused speculative credit remains in this worker's batch.
    peer.charge(81, 0).unwrap();
    drop((worker, peer));
    assert_eq!(group.finish().unwrap(), (100, 10));
    assert_eq!(root.used(), (100, 10));
    assert_eq!(memory.used_bytes(), baseline);
    drop(root);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn lock_batches_release_at_zero_charge_count_work_threshold_and_denial() {
    let (root, _) = root(MAX_WORK * 3, MAX_WORK * 3);
    let group = root.reserve_worker_group(MAX_WORK * 3, MAX_WORK * 3).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    // A test-only, nonblocking lock probe verifies release before leaf work.
    let shared = worker.account.borrow().worker.as_ref().unwrap().shared.clone();
    worker.with_debit_batch(|batch| {
        for _ in 0..MAX_CHARGES - 1 { batch.charge(0, 0).unwrap(); }
        assert!(shared.try_lock().is_err());
        batch.charge(0, 0).unwrap();
        assert!(shared.try_lock().is_ok());
        batch.charge(MAX_WORK - 1, 0).unwrap();
        assert!(shared.try_lock().is_err());
        batch.charge(1, 0).unwrap();
        assert!(shared.try_lock().is_ok());
        batch.charge(0, MAX_WORK).unwrap();
        assert!(shared.try_lock().is_ok());
        batch.charge(1, 0).unwrap();
        assert!(shared.try_lock().is_err());
        assert!(batch.charge(u64::MAX, 0).is_err());
        assert!(shared.try_lock().is_ok());
    }).unwrap();
    assert_eq!(worker.used(), (MAX_WORK + 1, MAX_WORK));
    drop(worker);
    assert_eq!(group.finish().unwrap(), (MAX_WORK + 1, MAX_WORK));
}

#[test]
fn nested_or_foreign_batch_reentry_is_rejected_without_changing_caps_or_spend() {
    let (root, _) = root(20, 10);
    let group = root.reserve_worker_group(20, 10).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let peer = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    worker.with_call_limits(7, 3, || worker.with_debit_batch(|batch| {
        batch.charge(2, 1).unwrap();
        assert_eq!(worker.with_debit_batch(|_| ()), Err(ResidentKernelError::InvalidInput));
        assert_eq!(peer.with_debit_batch(|_| ()), Err(ResidentKernelError::InvalidInput));
        assert_eq!(peer.charge(1, 0), Err(ResidentKernelError::InvalidInput));
        batch.charge(5, 2).unwrap();
        assert!(batch.charge(1, 0).is_err());
    }).unwrap()).unwrap();
    assert_eq!(worker.used(), (7, 3));
    assert_eq!(peer.used(), (0, 0));
    assert!(!NativeLibraryWorkBudget::has_active_scope());
    peer.charge(13, 7).unwrap();
    drop((worker, peer));
    assert_eq!(group.finish().unwrap(), (20, 10));
}

#[test]
fn ordinary_accounts_keep_their_original_charge_path_and_no_scope_leaks() {
    let (root, memory) = root(10, 3);
    root.with_call_limits(7, 2, || root.with_debit_batch(|batch| {
        assert!(matches!(&batch.mode, Mode::Ordinary(_)));
        batch.charge(7, 2).unwrap();
        assert!(batch.charge(1, 0).is_err());
    }).unwrap()).unwrap();
    assert_eq!(root.used(), (7, 2));
    assert!(!NativeLibraryWorkBudget::has_active_scope());
    root.charge(3, 1).unwrap();
    drop(root);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn concurrent_batches_cannot_overspend_either_dimension_or_hoard_unused_fuel() {
    let (root, memory) = root(1_000, 137);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(1_000, 137).unwrap();
    let start = Arc::new(std::sync::Barrier::new(4));
    let mut threads = Vec::new();
    for _ in 0..4 {
        let loan = group.worker_loan().unwrap();
        let start = start.clone();
        threads.push(std::thread::spawn(move || {
            let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
            start.wait(); // No blocking operation runs inside a batch.
            let accepted = worker.with_debit_batch(|batch| {
                let mut accepted = 0;
                while batch.charge(3, 1).is_ok() { accepted += 1; }
                accepted
            }).unwrap();
            assert_eq!(worker.used(), (accepted * 3, accepted));
            accepted
        }));
    }
    let accepted: u64 = threads.into_iter().map(|thread| thread.join().unwrap()).sum();
    assert_eq!(accepted, 137);
    assert_eq!(group.finish().unwrap(), (411, 137));
    assert_eq!(memory.used_bytes(), baseline);
    root.charge(589, 0).unwrap();
    assert_eq!(root.used(), (1_000, 137));
}

#[test]
fn zero_debit_still_rejects_a_sealed_worker_group() {
    let (root, _) = root(10, 3);
    let group = root.reserve_worker_group(10, 3).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    assert_eq!(group.finish(), Err(ResidentKernelError::InvalidInput));
    assert_eq!(worker.with_debit_batch(|batch| batch.charge(0, 0)).unwrap(),
        Err(ResidentKernelError::InvalidInput));
    assert_eq!(worker.used(), (0, 0));
    drop(worker);
    root.charge(10, 3).unwrap();
}

#[test]
#[cfg(panic = "unwind")]
fn batch_unwind_unlocks_poisoned_escrow_and_keeps_every_successful_debit() {
    let (root, memory) = root(30, 10);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(30, 10).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let peer = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        worker.with_call_limits(10, 5, || worker.with_debit_batch(|batch| {
            batch.charge(7, 3).unwrap();
            panic!("owned leaf unwind fixture");
        }).unwrap()).unwrap();
    }));
    assert!(result.is_err());
    assert!(!NativeLibraryWorkBudget::has_active_scope());
    assert_eq!(worker.used(), (7, 3));
    // Both the same account and a peer recover the mutex without refunds.
    worker.charge(3, 1).unwrap();
    peer.charge(20, 6).unwrap();
    drop((worker, peer));
    assert_eq!(group.finish().unwrap(), (30, 10));
    assert_eq!(memory.used_bytes(), baseline);
}

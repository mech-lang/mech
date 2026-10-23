use super::*;

fn root(compute: u64, comparison: u64) -> (NativeLibraryWorkBudget, mech_core::ManagedMemoryBudget) {
    let memory = mech_core::ManagedMemoryBudget::new(64 * 1024);
    (NativeLibraryWorkBudget::new(compute, comparison, &memory).unwrap(), memory)
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
    }).join().unwrap();
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
        }).join().unwrap();
        group.finish().unwrap();
        assert!(root.charge(1, 0).is_err());
    }).unwrap();
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
            right.with_call_limits(10, 5, || right.charge(10, 5)).unwrap().unwrap();
        }).join().unwrap();
        // Another worker's ten units do not consume this call's nine remaining.
        left.charge(9, 4).unwrap();
        assert!(left.charge(1, 0).is_err());
    }).unwrap();
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
    let threads = (0..8).map(|_| {
        let loan = group.worker_loan().unwrap();
        std::thread::spawn(move || {
            let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
            let success = worker.charge(7, 2).is_ok();
            assert_eq!(worker.used(), if success { (7, 2) } else { (0, 0) });
            success
        })
    }).collect::<Vec<_>>();
    let successes = threads.into_iter().map(|thread| usize::from(thread.join().unwrap())).sum::<usize>();
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
    assert!(std::thread::spawn(move || {
        let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
        worker.with_call_limits(10, 3, || {
            worker.charge(10, 3).unwrap();
            panic!("owned worker unwind fixture");
        }).unwrap();
    }).join().is_err());
    assert_eq!(group.finish().unwrap(), (10, 3));
    assert_eq!(root.used(), (10, 3));
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
fn foreign_scopes_nested_groups_and_memory_rehoming_are_rejected() {
    let (foreign, other_memory) = root(100, 10);
    let (root, memory) = root(100, 10);
    assert!(foreign.with_scope(|| root.reserve_worker_group(1, 1)).unwrap().is_err());
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
    assert!(foreign.with_scope(|| NativeLibraryWorkBudget::from_worker_loan(loan)).unwrap().is_err());
    drop(worker);
    assert_eq!(group.finish().unwrap(), (0, 0));
}

#[test]
fn failed_metadata_admission_and_unused_tokens_release_all_claims() {
    let memory = mech_core::ManagedMemoryBudget::new(64 * 1024);
    let root = NativeLibraryWorkBudget::new(10, 10, &memory).unwrap();
    let original = memory.used_bytes();
    assert!(matches!(root.reserve_worker_group_with_memory(10, 10, &memory.with_limits(original, 100)),
        Err(NativeWorkerBudgetError::Memory(_))));
    assert_eq!(root.used(), (0, 0));
    assert_eq!(root.unscoped_remaining(), Some((10, 10)));
    assert_eq!(memory.used_bytes(), original);
    let group = root.reserve_worker_group(10, 10).unwrap();
    let used = memory.used_bytes();
    assert!(matches!(group.worker_loan_with_memory(&memory.with_limits(used, 100)),
        Err(NativeWorkerBudgetError::Memory(_))));
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
    let first = root.reserve_worker_group(u64::MAX - 2, u64::MAX - 2).unwrap();
    assert!(root.reserve_worker_group(2, 0).is_err());
    let second = root.reserve_worker_group(1, 1).unwrap();
    assert!(root.charge(1, 0).is_err());
    let worker = NativeLibraryWorkBudget::from_worker_loan(second.worker_loan().unwrap()).unwrap();
    worker.charge(1, 1).unwrap();
    drop(worker);
    assert_eq!(second.finish().unwrap(), (1, 1));
    assert_eq!(root.used(), (2, 2));
    drop(first);
    assert_eq!(root.unscoped_remaining(), Some((u64::MAX - 2, u64::MAX - 2)));
}

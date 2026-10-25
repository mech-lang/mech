use super::*;

fn root(
    units: u64,
    compute: u64,
    comparison: u64,
) -> (NativeLibraryWorkBudget, mech_core::ManagedMemoryBudget) {
    let memory = mech_core::ManagedMemoryBudget::new(128 * 1024);
    (
        NativeLibraryWorkBudget::new(units * compute, units * comparison, &memory).unwrap(),
        memory,
    )
}

#[test]
fn large_paired_batches_use_local_credit_without_repeated_shared_locking() {
    let (compute, comparison) = (1 << 40, 1 << 34);
    let (root, memory) = root(512, compute, comparison);
    let baseline = memory.used_bytes();
    let group = root
        .reserve_worker_group(512 * compute, 512 * comparison)
        .unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    assert!(worker.try_charge_funded_batch(compute, comparison).unwrap());
    let held = memory.used_bytes();
    ESCROW_LOCK_STATS.with(|v| v.set((0, 0, 0, 0)));
    for _ in 0..40 {
        assert!(worker.try_charge_funded_batch(compute, comparison).unwrap());
    }
    assert_eq!(ESCROW_LOCK_STATS.with(Cell::get), (0, 0, 0, 0));
    assert_eq!(
        memory.used_bytes(),
        held,
        "credit renewal must allocate no backing"
    );
    assert_eq!(worker.used(), (41 * compute, 41 * comparison));
    assert_eq!(root.used(), (0, 0));
    drop(worker);
    let (spent, statistics) = group.finish_with_credit_statistics().unwrap();
    assert_eq!(spent, (41 * compute, 41 * comparison));
    assert_eq!(
        (statistics.claims, statistics.batches, statistics.reclaims),
        (1, 41, 1)
    );
    assert_eq!(root.used(), spent);
    assert_eq!(memory.used_bytes(), baseline);
    drop(root);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn peer_reclaims_and_original_debits_can_use_every_unspent_reserved_unit() {
    let (compute, comparison) = (1 << 40, 1 << 34);
    let (root, _) = root(64, compute, comparison);
    let group = root
        .reserve_worker_group(64 * compute, 64 * comparison)
        .unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    assert!(left.try_charge_funded_batch(compute, comparison).unwrap());
    // Left owns all initial credit. Right must recover its unused portion.
    assert!(
        right
            .try_charge_funded_batch(3 * compute, 3 * comparison)
            .unwrap()
    );
    assert!(left.try_charge_funded_batch(compute, comparison).unwrap());
    // An ordinary debit must also recover outstanding credit before denial.
    right.charge(59 * compute, 59 * comparison).unwrap();
    assert_eq!(left.used(), (2 * compute, 2 * comparison));
    assert_eq!(right.used(), (62 * compute, 62 * comparison));
    assert!(!left.try_charge_funded_batch(compute, comparison).unwrap());
    assert!(left.charge(compute, comparison).is_err());
    drop((left, right));
    let (spent, stats) = group.finish_with_credit_statistics().unwrap();
    assert_eq!(spent, (64 * compute, 64 * comparison));
    assert_eq!(stats.batches, 3);
    assert!(stats.reclaims >= 2);
    assert_eq!(root.used(), spent);
}

#[test]
fn root_foreign_call_caps_zero_prices_and_tight_headroom_keep_the_original_path() {
    let (root, _) = root(100, 1, 1);
    assert!(!root.try_charge_funded_batch(1, 1).unwrap());
    let group = root.reserve_worker_group(100, 100).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    worker.restrict(7, 7).unwrap();
    assert!(!worker.try_charge_funded_batch(2, 2).unwrap());
    assert!(!worker.try_charge_funded_batch(0, 1).unwrap());
    assert!(!worker.try_charge_funded_batch(1, u64::MAX).unwrap());
    assert_eq!(worker.used(), (0, 0));
    worker
        .with_call_limits(7, 7, || {
            assert!(!worker.try_charge_funded_batch(1, 1).unwrap());
            worker.charge(7, 7).unwrap();
        })
        .unwrap();
    assert!(worker.charge(1, 0).is_err());
    root.with_scope(|| assert!(!worker.try_charge_funded_batch(1, 1).unwrap()))
        .unwrap();
    drop(worker);
    let (spent, stats) = group.finish_with_credit_statistics().unwrap();
    assert_eq!(spent, (7, 7));
    assert_eq!((stats.claims, stats.batches), (0, 0));
}

#[test]
fn exact_remaining_queries_and_price_changes_preserve_both_cumulative_dimensions() {
    let (root, _) = root(1000, 1, 1);
    let group = root.reserve_worker_group(1000, 1000).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    assert!(worker.try_charge_funded_batch(1, 3).unwrap());
    assert_eq!(worker.unscoped_remaining(), Some((999, 997)));
    assert!(worker.try_charge_funded_batch(2, 2).unwrap());
    assert_eq!(worker.used(), (3, 5));
    assert_eq!(worker.unscoped_remaining(), Some((997, 995)));
    drop(worker);
    assert_eq!(group.finish().unwrap(), (3, 5));
    assert_eq!(root.used(), (3, 5));
}

#[test]
fn early_finish_seals_unspent_local_credit_without_refunding_successful_work() {
    let (root, memory) = root(1000, 1, 1);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(1000, 1000).unwrap();
    let worker = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    for _ in 0..3 {
        assert!(worker.try_charge_funded_batch(2, 1).unwrap());
    }
    assert!(group.finish().is_err());
    assert_eq!(root.used(), (6, 3));
    assert!(!worker.try_charge_funded_batch(2, 1).unwrap());
    assert_eq!(worker.charge(0, 0), Err(ResidentKernelError::InvalidInput));
    root.charge(994, 997).unwrap();
    drop(worker);
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
fn concurrent_credits_and_checked_debits_exhaust_exactly_one_finite_parent_allowance() {
    let (compute, comparison) = (1 << 40, 1 << 34);
    let (root, memory) = root(256, compute, comparison);
    let baseline = memory.used_bytes();
    let group = root
        .reserve_worker_group(256 * compute, 256 * comparison)
        .unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let threads = (0..8)
        .map(|_| {
            let loan = group.worker_loan().unwrap();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
                barrier.wait();
                let mut count = 0;
                for _ in 0..1000 {
                    let credited = worker.try_charge_funded_batch(compute, comparison).unwrap();
                    if !credited && worker.charge(compute, comparison).is_err() {
                        break;
                    }
                    count += 1;
                }
                assert_eq!(worker.used(), (count * compute, count * comparison));
                count
            })
        })
        .collect::<Vec<_>>();
    let accepted: u64 = threads.into_iter().map(|t| t.join().unwrap()).sum();
    assert_eq!(accepted, 256);
    let (spent, stats) = group.finish_with_credit_statistics().unwrap();
    assert_eq!(spent, (256 * compute, 256 * comparison));
    assert!(stats.batches > 0);
    assert_eq!(root.used(), spent);
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
fn group_close_races_fold_every_accepted_atomic_pair_and_seal_every_future_attempt() {
    let (root, _) = root(1_000_000, 1, 1);
    let group = root.reserve_worker_group(1_000_000, 1_000_000).unwrap();
    let loan = group.worker_loan().unwrap();
    let (ready, receive) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
        assert!(worker.try_charge_funded_batch(1, 1).unwrap());
        ready.send(()).unwrap();
        for _ in 0..100_000 {
            if !worker.try_charge_funded_batch(1, 1).unwrap() {
                break;
            }
        }
        worker.used()
    });
    receive
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    drop(group);
    let accepted = worker.join().unwrap();
    assert_eq!(root.used(), accepted);
    assert!(accepted.0 >= 1 && accepted.0 < 1_000_000);
}

#[test]
fn excess_worker_slots_retain_original_checked_execution_and_release_all_metadata() {
    let (root, memory) = root(1000, 1, 1);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(1000, 1000).unwrap();
    let mut workers = (0..SLOTS + 1)
        .map(|_| NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap())
        .collect::<Vec<_>>();
    let last = workers.pop().unwrap();
    assert!(!last.try_charge_funded_batch(1, 1).unwrap());
    last.charge(1, 1).unwrap();
    drop((last, workers));
    assert_eq!(group.finish().unwrap(), (1, 1));
    assert_eq!(memory.used_bytes(), baseline);
}

#[test]
fn comparison_denial_after_credit_reclamation_does_not_steal_remaining_compute() {
    let (root, _) = root(512, 1, 1);
    let group = root.reserve_worker_group(512, 512).unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    for _ in 0..3 {
        assert!(left.try_charge_funded_batch(8, 1).unwrap());
    }
    right.charge(480, 509).unwrap();
    assert!(!left.try_charge_funded_batch(8, 1).unwrap());
    assert!(matches!(
        left.charge(8, 1),
        Err(ResidentKernelError::ResourceLimit {
            dimension: MemoryBudgetDimension::ComparisonWork,
            required: 513,
            limit: 512,
            ..
        })
    ));
    assert_eq!(left.used(), (24, 3));
    drop((left, right));
    assert_eq!(group.finish().unwrap(), (504, 512));
    root.charge(8, 0).unwrap();
    assert_eq!(root.used(), (512, 512));
}

#[test]
#[cfg(panic = "unwind")]
fn worker_unwind_settles_pending_credit_without_refunding_the_batch() {
    let (root, memory) = root(1000, 1, 1);
    let baseline = memory.used_bytes();
    let group = root.reserve_worker_group(1000, 1000).unwrap();
    let loan = group.worker_loan().unwrap();
    assert!(
        std::thread::spawn(move || {
            let worker = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
            assert!(worker.try_charge_funded_batch(8, 3).unwrap());
            panic!("owned credited worker unwind fixture");
        })
        .join()
        .is_err()
    );
    assert_eq!(group.finish().unwrap(), (8, 3));
    assert_eq!(root.used(), (8, 3));
    assert_eq!(memory.used_bytes(), baseline);
}

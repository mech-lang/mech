use super::*;

#[test]
fn paid_cleanup_is_same_ledger_exact_nonrefundable_and_cannot_overdraw() {
    let mut work = NativeWorkMeter::new(31).unwrap();
    work.charge(NativeExecutionPhase::Setup, 7).unwrap();
    let identity = work.identity();
    let mut paid = work.try_retirement_batch(13).unwrap();
    assert_eq!(paid.meter().identity(), identity);
    assert_eq!(paid.meter().used(), 20);
    paid.pay(6).unwrap();
    assert!(paid.pay(8).is_err());
    paid.pay(7).unwrap();
    paid.finish().unwrap();
    assert!(work.try_retirement_batch(12).is_none());
    assert_eq!(work.used(), 20);
    let mut paid = work.try_retirement_batch(5).unwrap();
    paid.pay(2).unwrap();
    assert!(paid.finish().is_err());
    assert_eq!(work.used(), 25, "unconsumed paid work is never refunded");
    drop(work.try_retirement_batch(6).unwrap());
    assert_eq!(work.used(), 31);
    assert!(work.try_retirement_batch(1).is_none());
}

#[test]
fn paid_cleanup_overflow_and_finite_caps_decline_without_spending() {
    let mut inline = NativeWorkMeter::new(u64::MAX).unwrap();
    inline
        .charge(NativeExecutionPhase::Setup, u64::MAX - 1)
        .unwrap();
    assert!(inline.try_retirement_batch(2).is_none());
    assert_eq!(inline.used(), u64::MAX - 1);
    inline.try_retirement_batch(1).unwrap().pay(1).unwrap();
    assert_eq!(inline.used(), u64::MAX);

    let memory = ManagedMemoryBudget::new(8192);
    let mut work = NativeWorkMeter::new(1000).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
        .unwrap();
    let used = work.used();
    library.restrict(used + 9, 100).unwrap();
    assert!(work.try_retirement_batch(10).is_none());
    assert_eq!(work.used(), used);
    let mut paid = work.try_retirement_batch(9).unwrap();
    paid.pay(9).unwrap();
    paid.finish().unwrap();
    assert_eq!(library.used().0, used + 9);
    drop((work, library));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn paid_cleanup_declines_nested_matching_and_foreign_scopes() {
    let memory = ManagedMemoryBudget::new(8192);
    let mut work = NativeWorkMeter::new(1000).unwrap();
    let library = work
        .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
        .unwrap();
    let foreign = NativeLibraryWorkBudget::new(1000, 100, &memory).unwrap();
    let used = work.used();
    library
        .with_call_limits(5, 0, || {
            assert!(work.try_retirement_batch(1).is_none());
            assert_eq!(work.used(), used);
            work.charge(NativeExecutionPhase::Setup, 1).unwrap();
        })
        .unwrap();
    foreign
        .with_scope(|| {
            assert!(work.try_retirement_batch(1).is_none());
            assert_eq!(work.used(), used + 1);
        })
        .unwrap();
    let mut paid = work.try_retirement_batch(2).unwrap();
    paid.pay(2).unwrap();
    paid.finish().unwrap();
    assert_eq!(work.used(), used + 3);
    drop((work, library, foreign));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn paid_cleanup_keeps_parent_backed_workers_on_ordered_debits() {
    let memory = ManagedMemoryBudget::new(8192);
    let mut parent = NativeWorkMeter::new(1000).unwrap();
    let library = parent
        .shared_library_budget(NativeExecutionPhase::Setup, 100, &memory)
        .unwrap();
    let group = library.reserve_worker_group(30, 10).unwrap();
    let loan = group.worker_loan().unwrap();
    let worker_memory = memory.clone();
    let used = std::thread::spawn(move || {
        let budget = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
        let mut worker = NativeWorkMeter::from_worker_budget(budget, &worker_memory).unwrap();
        assert!(worker.try_retirement_batch(2).is_none());
        assert_eq!(worker.used(), 0);
        worker.charge(NativeExecutionPhase::Setup, 2).unwrap();
        worker.used()
    })
    .join()
    .unwrap();
    assert_eq!(used, 2);
    assert_eq!(group.finish().unwrap(), (2, 0));
    assert_eq!(parent.used(), 66);
    drop((parent, library));
    assert_eq!(memory.used_bytes(), 0);
}

use super::*;

// Independent transcription of the previous selection and its error order.
fn original(price: Price, funded: u64, remaining: (u64, u64))
    -> Result<(u64, u64, (u64, u64)), Error>
{
    let count = (remaining.0 / price.compute.max(1) / 4)
        .min(remaining.1 / price.comparison.max(1) / 4).min(BATCH).max(1);
    let funded = add(funded, count)?;
    Ok((mul(price.compute, count)?, mul(price.comparison, count)?, (count, funded)))
}

#[test]
fn refill_plan_matches_original_at_both_thresholds_and_integer_boundaries() {
    let prices = [0, 1, 2, 3, 17, u64::MAX / (4 * BATCH),
        u64::MAX / (4 * BATCH) + 1, u64::MAX / BATCH, u64::MAX / BATCH + 1, u64::MAX];
    for compute in prices {
        for comparison in prices {
            let price = Price { compute, comparison };
            let boundary = |unit: u64| {
                let mut values = vec![0, 1, 3, 4, 4095, 4096, u64::MAX];
                if let Some(at) = unit.max(1).checked_mul(4 * BATCH) {
                    values.extend([at - 1, at, at.saturating_add(1)]);
                }
                values
            };
            for compute_left in boundary(compute) {
                for comparison_left in boundary(comparison) {
                    for funded in [0, 1, u64::MAX - BATCH, u64::MAX - BATCH + 1, u64::MAX - 1, u64::MAX] {
                        let remaining = (compute_left, comparison_left);
                        let expected = original(price, funded, remaining);
                        let actual = RefillPlan::new(price, funded).debit(remaining);
                        match (actual, expected) {
                            (Ok(a), Ok(b)) => assert_eq!(a, b, "{price:?} {funded} {remaining:?}"),
                            (Err(Error::Overflow), Err(Error::Overflow)) => (),
                            (a, b) => panic!("refill differs: {price:?} {funded} {remaining:?}: {a:?} {b:?}"),
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn refill_plan_overflow_falls_back_to_a_smaller_fundable_batch() {
    for price in [Price { compute: u64::MAX, comparison: 1 },
        Price { compute: 1, comparison: u64::MAX }]
    {
        let plan = RefillPlan::new(price, 9);
        assert!(plan.full.is_none());
        assert_eq!(plan.debit((u64::MAX, u64::MAX)).unwrap(),
            (price.compute, price.comparison, (1, 10)));
    }
    let plan = RefillPlan::new(Price { compute: 1, comparison: 1 }, u64::MAX - 1);
    assert!(plan.full.is_none());
    assert_eq!(plan.debit((4, 4)).unwrap(), (1, 1, (1, u64::MAX)));
}

#[test]
fn packed_refill_commits_exact_batches_and_denies_atomically_at_root_limits() {
    for dimension in [0, 1] {
        for around in [-1i64, 0, 1] {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let work = new_work(&memory);
            let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
            let price = map.price(map.frame.capacity, map.frame.len).unwrap();
            let used = work.used();
            let mut remaining = (price.compute * 4 * BATCH, price.comparison * 4 * BATCH);
            let selected = if dimension == 0 { &mut remaining.0 } else { &mut remaining.1 };
            *selected = selected.checked_add_signed(around).unwrap();
            work.restrict(used.0 + remaining.0, used.1 + remaining.1).unwrap();
            let (compute, comparison, (count, funded)) = original(price, 0, remaining).unwrap();
            assert_eq!(map.refill().unwrap(), count);
            assert_eq!((map.funded, map.issued, map.frame.fuel), (funded, count, count));
            assert_eq!(work.used(), (used.0 + compute, used.1 + comparison));
            let before = (work.used(), map.funded, map.issued, map.frame.fuel);
            assert!(map.refill().is_err()); // No renewal of unconsumed fuel.
            assert_eq!((work.used(), map.funded, map.issued, map.frame.fuel), before);
            drop((map, work));
            assert_eq!(memory.used_bytes(), 0);
        }
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = new_work(&memory);
        let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
        let price = map.price(map.frame.capacity, map.frame.len).unwrap();
        let used = work.used();
        let remaining = if dimension == 0 { (price.compute - 1, price.comparison) }
            else { (price.compute, price.comparison - 1) };
        work.restrict(used.0 + remaining.0, used.1 + remaining.1).unwrap();
        assert!(map.refill().is_err());
        assert_eq!((work.used(), map.funded, map.issued, map.frame.fuel), (used, 0, 0, 0));
    }
}

#[test]
fn packed_refill_preserves_foreign_scope_precedence_over_price_overflow() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let foreign = new_work(&memory);
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    map.body.scalar_compute = u64::MAX;
    let before = (work.used(), map.funded, map.issued, map.frame.fuel);
    assert!(matches!(foreign.with_scope(|| map.refill()).unwrap(),
        Err(Error::Binding("packed-map entered a foreign work scope"))));
    assert_eq!((work.used(), map.funded, map.issued, map.frame.fuel), before);
    assert!(matches!(map.refill(), Err(Error::Overflow)));
    assert_eq!((work.used(), map.funded, map.issued, map.frame.fuel), before);
}

#[test]
fn packed_refill_keeps_worker_escrow_and_local_account_in_agreement() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let root = new_work(&memory);
    let before = root.used();
    let group = root.reserve_worker_group(1_000_000_000_000, 1_000_000_000).unwrap();
    let left = group.worker_loan().unwrap();
    let right = group.worker_loan().unwrap();
    let used = std::thread::scope(|scope| {
        let jobs = [left, right].map(|loan| scope.spawn(move || {
            let work = NativeLibraryWorkBudget::from_worker_loan(loan).unwrap();
            let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
            let price = map.price(map.frame.capacity, map.frame.len).unwrap();
            let before = work.used();
            assert_eq!(map.refill().unwrap(), BATCH);
            assert_eq!(work.used(), (before.0 + price.compute * BATCH,
                before.1 + price.comparison * BATCH));
            map.frame.fuel = 0;
            let before = work.used();
            work.restrict(before.0 + price.compute * 4, before.1 + price.comparison * 4).unwrap();
            assert_eq!(map.refill().unwrap(), 1);
            assert_eq!(work.used(), (before.0 + price.compute, before.1 + price.comparison));
            drop(map);
            work.used()
        }));
        jobs.map(|job| job.join().unwrap())
    });
    let spent = group.finish().unwrap();
    assert_eq!(spent, (used[0].0 + used[1].0, used[0].1 + used[1].1));
    assert_eq!(root.used(), (before.0 + spent.0, before.1 + spent.1));
    drop(root);
    assert_eq!(memory.used_bytes(), 0);
}

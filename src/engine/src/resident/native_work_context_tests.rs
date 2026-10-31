use super::*;

fn budget(compute: u64, comparison: u64) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(
        compute,
        comparison,
        &mech_core::ManagedMemoryBudget::new(1 << 20),
    )
    .unwrap()
}

fn state(b: &NativeLibraryWorkBudget) -> ((u64, u64), (u64, u64)) {
    let a = b.account.borrow();
    (
        (a.used.compute, a.used.comparison),
        (a.reserved.compute, a.reserved.comparison),
    )
}

#[test]
fn scoped_context_agrees_with_ordinary_debits_and_denied_state() {
    with_native_work_context(|context| {
        for (used, compared, reserved, reserved_comparison, cap, comparison_cap, debit, caller) in [
            (0, 0, 0, 0, 10, 10, 3, 10),
            (4, 2, 1, 1, 10, 10, 3, 10),
            (4, 2, 1, 1, 5, 10, 3, 10),
            (4, 2, 1, 1, 10, 2, 3, 10),
            (4, 2, 1, 1, 10, 10, 3, 6),
            (u64::MAX, 0, 0, 0, u64::MAX, 10, 1, u64::MAX),
            (0, u64::MAX, 0, 1, 0, u64::MAX, 1, u64::MAX),
            (1, 0, u64::MAX, 0, u64::MAX, 10, 0, u64::MAX),
        ] {
            for scoped in [false, true] {
                let left = budget(cap, comparison_cap);
                let right = budget(cap, comparison_cap);
                for b in [&left, &right] {
                    let mut a = b.account.borrow_mut();
                    a.used = ControlWork {
                        compute: used,
                        comparison: compared,
                    };
                    a.reserved = ControlWork {
                        compute: reserved,
                        comparison: reserved_comparison,
                    };
                }
                let before = state(&left);
                let actual = || left.charge_compute_with_limit_scoped(context, debit, caller);
                let expected = || right.charge_compute_with_limit(debit, caller);
                let (actual, expected) = if scoped {
                    (
                        left.with_call_limits(2, 0, actual).unwrap(),
                        right.with_call_limits(2, 0, expected).unwrap(),
                    )
                } else {
                    (actual(), expected())
                };
                assert_eq!(actual, expected);
                assert_eq!(state(&left), state(&right));
                if actual.is_err() {
                    assert_eq!(state(&left), before);
                }
            }
        }
    });
}

#[test]
fn scoped_context_reads_live_accounts_limits_and_error_precedence() {
    let b = budget(10, u64::MAX);
    let foreign = budget(10, 10);
    with_native_work_context(|context| {
        assert!(!context.has_active_scope());
        foreign
            .with_scope(|| {
                assert!(context.has_active_scope());
                assert!(matches!(
                    b.charge_compute_with_limit_scoped(context, 1, 0),
                    Err(ResidentKernelError::ResourceLimit {
                        required: 1,
                        limit: 0,
                        ..
                    })
                ));
                assert_eq!(
                    b.charge_compute_with_limit_scoped(context, 1, 10),
                    Err(ResidentKernelError::InvalidInput)
                );
            })
            .unwrap();
        assert!(!context.has_active_scope());
        b.with_call_limits(1, 0, || {
            b.charge_compute_with_limit_scoped(context, 1, 10).unwrap();
            assert!(b.charge_compute_with_limit_scoped(context, 1, 10).is_err());
        })
        .unwrap();
        b.charge_compute_with_limit_scoped(context, 1, 2).unwrap();
        {
            let mut a = b.account.borrow_mut();
            a.compute_limit = 0;
            a.used.comparison = u64::MAX;
            a.reserved.comparison = 1;
        }
        // Both reservation sums precede either exceeded-limit result.
        assert_eq!(
            b.charge_compute_with_limit_scoped(context, 1, 10),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(b.used(), (2, u64::MAX));
    });
}

#[test]
fn scoped_context_preserves_nested_observer_high_water() {
    let b = budget(100, 100);
    let demand = |compute, comparison| ResourceDemand {
        work: mech_core::WorkDemand {
            compute,
            comparison,
            ..Default::default()
        },
        ..Default::default()
    };
    with_native_work_context(|context| {
        b.with_call_limits(20, 10, || {
            observe_control_work(demand(3, 1)).unwrap();
            b.charge_compute_with_limit_scoped(context, 2, 20).unwrap();
            observe_control_work(demand(3, 1)).unwrap();
            assert_eq!(b.used(), (5, 1));
            observe_control_work(demand(4, 1)).unwrap();
            b.with_call_limits(1, 0, || {
                assert!(
                    b.charge_compute_with_limit_scoped(context, 2, u64::MAX)
                        .is_err()
                );
                b.charge_compute_with_limit_scoped(context, 1, u64::MAX)
                    .unwrap();
            })
            .unwrap();
            observe_control_work(demand(4, 1)).unwrap();
            assert_eq!(b.used(), (7, 1));
            b.charge_compute_with_limit_scoped(context, 3, u64::MAX)
                .unwrap();
        })
        .unwrap();
        assert!(!context.has_active_scope());
        assert_eq!(b.used(), (10, 1));
    });
}

#[test]
fn scoped_context_keeps_shared_worker_denial_atomic_and_recovers() {
    let root = budget(10, 5);
    let group = root.reserve_worker_group(10, 5).unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    with_native_work_context(|context| {
        right.charge(8, 0).unwrap();
        assert!(
            left.charge_compute_with_limit_scoped(context, 3, u64::MAX)
                .is_err()
        );
        assert_eq!(left.used(), (0, 0));
        left.charge_compute_with_limit_scoped(context, 2, u64::MAX)
            .unwrap();
        assert_eq!(left.used(), (2, 0));
    });
    drop((left, right));
    assert_eq!(group.finish().unwrap(), (10, 0));
    assert_eq!(root.used(), (10, 0));
}

#[test]
fn scoped_context_survives_unwind_with_caps_and_account_restored() {
    let b = budget(10, 10);
    with_native_work_context(|context| {
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            b.with_call_limits(1, 0, || {
                b.charge_compute_with_limit_scoped(context, 1, 10).unwrap();
                panic!("leave a nested call");
            })
            .unwrap();
        }));
        assert!(failed.is_err());
        assert!(!context.has_active_scope());
        assert_eq!(b.used(), (1, 0));
        b.charge_compute_with_limit_scoped(context, 2, 3).unwrap();
        assert_eq!(b.used(), (3, 0));
    });
}

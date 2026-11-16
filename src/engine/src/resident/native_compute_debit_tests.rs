//! Compute-only caller admission must preserve the ordinary debit's semantics.
use super::*;

fn budget(compute: u64, comparison: u64) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(
        compute,
        comparison,
        &mech_core::ManagedMemoryBudget::new(8192),
    )
    .unwrap()
}

// Exact former boundary: check the extra caller ceiling, then run the ordinary
// maintained debit. The new fast boundary must agree with that independent
// arithmetic path as well as the explicit error-precedence expectations below.
fn ordinary(
    budget: &NativeLibraryWorkBudget,
    compute: u64,
    limit: u64,
) -> Result<(), ResidentKernelError> {
    let error = |required| ResidentKernelError::ResourceLimit {
        phase: mech_core::ResidentResourcePhase::Admission,
        dimension: MemoryBudgetDimension::ComputeWork,
        required,
        limit,
    };
    let required = budget
        .account
        .borrow()
        .used
        .compute
        .checked_add(compute)
        .ok_or_else(|| error(u64::MAX))?;
    if required > limit {
        return Err(error(required));
    }
    CONTROL_WORK_CONTEXT.with(|context| budget.charge_in_context(context, compute, 0))
}

fn state(budget: &NativeLibraryWorkBudget) -> ((u64, u64), (u64, u64)) {
    let a = budget.account.borrow();
    (
        (a.used.compute, a.used.comparison),
        (a.reserved.compute, a.reserved.comparison),
    )
}

#[test]
fn compute_debit_matches_ordinary_arithmetic_and_preserves_denied_state() {
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
            let call = |b: &NativeLibraryWorkBudget, reference| {
                let run = || {
                    if reference {
                        ordinary(b, debit, caller)
                    } else {
                        b.charge_compute_with_limit(debit, caller)
                    }
                };
                if scoped {
                    b.with_call_limits(2, 0, run).unwrap()
                } else {
                    run()
                }
            };
            let expected = call(&right, true);
            assert_eq!(call(&left, false), expected);
            assert_eq!(state(&left), state(&right));
            if expected.is_err() {
                assert_eq!(state(&left), before);
            }
        }
    }
}

#[test]
fn compute_debit_keeps_caller_foreign_reservation_order_and_retry() {
    let current = budget(0, u64::MAX);
    let foreign = budget(100, 100);
    current.account.borrow_mut().reserved.comparison = 1;
    current.account.borrow_mut().used.comparison = u64::MAX;
    foreign
        .with_scope(|| {
            assert!(matches!(
                current.charge_compute_with_limit(1, 0),
                Err(ResidentKernelError::ResourceLimit {
                    required: 1,
                    limit: 0,
                    ..
                })
            ));
            assert_eq!(
                current.charge_compute_with_limit(1, u64::MAX),
                Err(ResidentKernelError::InvalidInput)
            );
        })
        .unwrap();
    // Both reservation additions are evaluated before either exceeded-limit
    // result is chosen. Comparison reservation overflow still wins here.
    assert_eq!(
        current.charge_compute_with_limit(1, u64::MAX),
        Err(ResidentKernelError::InvalidShape)
    );
    assert_eq!(current.used(), (0, u64::MAX));
    {
        let mut a = current.account.borrow_mut();
        a.used = ControlWork::default();
        a.reserved = ControlWork::default();
        a.compute_limit = 2;
    }
    current.charge_compute_with_limit(2, 2).unwrap();
    assert!(current.charge_compute_with_limit(1, u64::MAX).is_err());
    assert_eq!(current.used(), (2, 0));
    current.charge_compute_with_limit(0, 2).unwrap();
    current.account.borrow_mut().used.compute = u64::MAX;
    foreign
        .with_scope(|| {
            assert!(matches!(
                current.charge_compute_with_limit(1, u64::MAX),
                Err(ResidentKernelError::ResourceLimit {
                    required: u64::MAX,
                    ..
                })
            ))
        })
        .unwrap();
}

#[test]
fn compute_debit_preserves_nested_observations_and_live_relative_caps() {
    let b = budget(100, 100);
    let demand = |compute, comparison| ResourceDemand {
        work: mech_core::WorkDemand {
            compute,
            comparison,
            ..Default::default()
        },
        ..Default::default()
    };
    b.with_call_limits(20, 10, || {
        observe_control_work(demand(3, 1)).unwrap();
        b.charge_compute_with_limit(2, 20).unwrap();
        observe_control_work(demand(3, 1)).unwrap();
        assert_eq!(b.used(), (5, 1));
        observe_control_work(demand(4, 1)).unwrap();
        assert_eq!(b.used(), (6, 1));
        b.with_call_limits(1, 0, || {
            assert!(b.charge_compute_with_limit(2, u64::MAX).is_err());
            assert_eq!(b.used(), (6, 1));
            b.charge_compute_with_limit(1, u64::MAX).unwrap();
        })
        .unwrap();
        observe_control_work(demand(4, 1)).unwrap();
        assert_eq!(b.used(), (7, 1));
        assert!(b.charge_compute_with_limit(3, 9).is_err());
        b.charge_compute_with_limit(3, u64::MAX).unwrap();
    })
    .unwrap();
    assert_eq!(b.used(), (10, 1));
    assert!(!NativeLibraryWorkBudget::has_active_scope());
    b.charge_compute_with_limit(1, 11).unwrap();
}

#[test]
fn compute_debit_preserves_shared_worker_denial_and_recovery() {
    let root = budget(10, 5);
    let group = root.reserve_worker_group(10, 5).unwrap();
    let left = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    let right = NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
    right.charge(8, 0).unwrap();
    assert!(left.charge_compute_with_limit(3, u64::MAX).is_err());
    assert_eq!(left.used(), (0, 0));
    left.charge_compute_with_limit(2, u64::MAX).unwrap();
    assert_eq!(left.used(), (2, 0));
    drop((left, right));
    assert_eq!(group.finish().unwrap(), (10, 0));
    assert_eq!(root.used(), (10, 0));
}

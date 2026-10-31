use super::*;
use crate::{DimensionExpr, SchemaDraft, SchemaTableBuilder};

fn fresh(matrix: bool) -> Value {
    let body = if matrix {
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: [DimensionExpr::Constant(1), DimensionExpr::Constant(3)].into(),
        }
    } else {
        SchemaBody::String
    };
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let schemas = Arc::new(build.table);
    if matrix {
        let shape = schemas
            .get(schema)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        Value::from_unsigned_matrix_buffer(
            schema,
            shape,
            UnsignedMatrixBuffer::U8(vec![7, 8, 9]),
            schemas,
        )
        .unwrap()
    } else {
        ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::String("first admission".into()),
        }
        .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
        .unwrap()
    }
}

fn admit(value: Value, budget: &crate::ManagedMemoryBudget, independent: bool) -> Value {
    let bytes = if independent {
        value.unshared_memory_budget_admission_bytes(budget)
    } else {
        value.memory_budget_admission_bytes(budget)
    }
    .unwrap();
    let mut claim = budget.reserve_capacity(bytes).unwrap();
    let value = if independent {
        value.into_independent_memory_budget(&mut claim)
    } else {
        value.into_memory_budget(&mut claim)
    }
    .unwrap();
    assert_eq!(claim.capacity_bytes(), 0);
    value
}

#[test]
fn first_admission_reuses_exclusive_wrapper_without_changing_metadata_or_data() {
    for matrix in [false, true] {
        for independent in [false, true] {
            let value = fresh(matrix);
            let root = Arc::as_ptr(&value.root);
            let shape = Arc::as_ptr(&value.shape);
            let schemas = Arc::as_ptr(value.schemas.as_ref().unwrap());
            let identity = value.frozen_storage_identity();
            let canonical = value.canonical_data_draft().unwrap();
            let budget = crate::ManagedMemoryBudget::new(1_000_000);
            let bytes = value
                .unshared_memory_budget_admission_bytes(&budget)
                .unwrap();
            let nodes = value.memory_budget_ownership_witness().unwrap().1;
            let value = admit(value, &budget, independent);
            assert_eq!(
                Arc::as_ptr(&value.root),
                root,
                "first admission should reuse exclusive unadmitted wrapper"
            );
            assert_eq!(Arc::as_ptr(&value.shape), shape);
            assert_eq!(Arc::as_ptr(value.schemas.as_ref().unwrap()), schemas);
            assert!(Weak::ptr_eq(
                &identity.data,
                &Arc::downgrade(&value.root.data)
            ));
            assert_eq!(value.canonical_data_draft().unwrap(), canonical);
            assert_eq!(
                budget.used_bytes(),
                bytes,
                "conservative admission envelope stays unchanged"
            );
            assert_eq!(budget.retained_snapshot_nodes(), nodes);
            let mut zero = budget.reserve_capacity(0).unwrap();
            let again = value.clone().into_memory_budget(&mut zero).unwrap();
            assert!(Arc::ptr_eq(&value.root, &again.root));
            drop((value, again, zero));
            assert_eq!(budget.used_bytes(), 0);
            assert_eq!(budget.retained_nodes(), 0);
            assert_eq!(identity.data.strong_count(), 0);
        }
    }
}

#[test]
fn registration_head_retirement_preserves_weak_observers_and_recovery() {
    let raw = fresh(true);
    let expected = raw.canonical_data_draft().unwrap();
    let budget = crate::ManagedMemoryBudget::new(1_000_000);
    let owned = admit(raw.clone(), &budget, true);
    let registration = Arc::downgrade(&owned.root.memory_budget.as_ref().unwrap().registration);
    drop(owned);
    assert_eq!(registration.strong_count(), 0);
    assert!(registration.upgrade().is_none());
    assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
    assert_eq!(budget.retained_snapshot_nodes(), 0);
    assert_eq!(raw.canonical_data_draft().unwrap(), expected);
    drop(admit(raw.clone(), &budget, false));
    assert!(registration.upgrade().is_none());
    assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
    raw.root
        .data
        .with_budget_imports(|head| assert!(head.is_none()));
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
#[test]
fn registration_link_exclusivity_keeps_concurrent_imports_and_retirements_safe() {
    let raw = fresh(false);
    let start = std::sync::Barrier::new(4);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let raw = &raw;
            let start = &start;
            scope.spawn(move || {
                let budget = crate::ManagedMemoryBudget::new(1_000_000);
                let expected = raw.canonical_data_draft().unwrap();
                start.wait();
                for visit in 0..64 {
                    let owned = admit(raw.clone(), &budget, visit % 2 == 0);
                    let alias = owned.clone();
                    std::thread::yield_now();
                    drop(owned);
                    assert!(alias.memory_budget_retained_bytes(&budget).is_some());
                    assert_eq!(alias.canonical_data_draft().unwrap(), expected);
                    drop(alias);
                    assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
                    assert_eq!(budget.retained_snapshot_nodes(), 0);
                }
            });
        }
    });
    raw.root
        .data
        .with_budget_imports(|head| assert!(head.is_none()));
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
#[test]
fn exclusive_registration_retirement_recovers_poisoned_gates() {
    let raw = fresh(false);
    let budget = crate::ManagedMemoryBudget::new(1_000_000);
    let owned = admit(raw.clone(), &budget, true);
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        raw.root.data.with_budget_imports(|_| {
            owned
                .root
                .memory_budget
                .as_ref()
                .unwrap()
                .registration
                .with_links(|_| panic!("test both gate recovery paths"));
        });
    }));
    assert!(poisoned.is_err());
    drop(owned);
    assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
    drop(admit(raw.clone(), &budget, false));
    raw.root
        .data
        .with_budget_imports(|head| assert!(head.is_none()));
    assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
}

#[test]
fn first_admission_keeps_registration_failure_but_removes_wrapper_failure_point() {
    for independent in [false, true] {
        let budget = crate::ManagedMemoryBudget::new(1_000_000);
        let raw = fresh(true);
        let identity = raw.frozen_storage_identity();
        let bytes = raw.unshared_memory_budget_admission_bytes(&budget).unwrap();
        let mut claim = budget.reserve_capacity(bytes).unwrap();
        budget.inject_snapshot_import_failure_after(0);
        let rejected = if independent {
            raw.into_independent_memory_budget(&mut claim)
        } else {
            raw.into_memory_budget(&mut claim)
        };
        assert!(
            matches!(rejected, Err(crate::MemoryRuntimeError::AllocationFailed { requested, .. })
            if requested == Value::budget_registration_allocation_bytes())
        );
        assert_eq!(claim.capacity_bytes(), bytes);
        assert_eq!(budget.retained_nodes(), 0);
        assert_eq!(identity.data.strong_count(), 0);
        drop(claim);
        assert_eq!(budget.used_bytes(), 0);

        let raw = fresh(true);
        let root = Arc::as_ptr(&raw.root);
        let mut claim = budget
            .reserve_capacity(raw.unshared_memory_budget_admission_bytes(&budget).unwrap())
            .unwrap();
        budget.inject_snapshot_import_failure_after(1);
        let result = if independent {
            raw.into_independent_memory_budget(&mut claim)
        } else {
            raw.into_memory_budget(&mut claim)
        };
        let value =
            result.expect("unique first admission performs only the registration allocation");
        assert_eq!(Arc::as_ptr(&value.root), root);
        assert!(
            budget.check_snapshot_import_allocation(1, 1).is_err(),
            "removed wrapper allocation must not consume the next injected failure"
        );
        drop((value, claim));
        assert_eq!(budget.used_bytes(), 0);
        assert_eq!(budget.retained_nodes(), 0);
        let retried = admit(fresh(true), &budget, independent);
        drop(retried);
        assert_eq!(budget.used_bytes(), 0);
    }
}

#[test]
fn shared_or_weak_wrapper_retains_both_allocation_failures_and_aliases() {
    for weak_wrapper in [false, true] {
        for step in [0, 1] {
            let raw = fresh(false);
            let expected = raw.canonical_data_draft().unwrap();
            let alias = (!weak_wrapper).then(|| raw.clone());
            let weak = weak_wrapper.then(|| Arc::downgrade(&raw.root));
            let budget = crate::ManagedMemoryBudget::new(1_000_000);
            let bytes = raw.unshared_memory_budget_admission_bytes(&budget).unwrap();
            let mut claim = budget.reserve_capacity(bytes).unwrap();
            budget.inject_snapshot_import_failure_after(step);
            assert!(matches!(
                raw.into_independent_memory_budget(&mut claim),
                Err(crate::MemoryRuntimeError::AllocationFailed { .. })
            ));
            assert_eq!(claim.capacity_bytes(), bytes);
            assert_eq!(budget.retained_nodes(), 0);
            if let Some(alias) = alias {
                assert_eq!(alias.canonical_data_draft().unwrap(), expected);
                assert!(alias.root.memory_budget.is_none());
                drop(admit(alias, &budget, true));
            } else {
                assert_eq!(weak.unwrap().strong_count(), 0);
                drop(admit(fresh(false), &budget, true));
            }
            drop(claim);
            assert_eq!(budget.used_bytes(), 0);
            assert_eq!(budget.retained_nodes(), 0);
        }
    }
}

#[test]
fn unique_unadmitted_wrapper_may_share_data_with_foreign_admission() {
    let raw = fresh(true);
    let original_root = Arc::as_ptr(&raw.root);
    let identity = raw.frozen_storage_identity();
    let a = crate::ManagedMemoryBudget::new(1_000_000);
    let foreign = admit(raw.clone(), &a, true);
    assert_eq!(Arc::strong_count(&raw.root), 1);
    assert!(Arc::strong_count(&raw.root.data) > 1);
    let b = crate::ManagedMemoryBudget::new(1_000_000);
    let current = admit(raw, &b, false);
    assert_eq!(Arc::as_ptr(&current.root), original_root);
    assert!(current.shares_frozen_storage(&foreign));
    assert!(Weak::ptr_eq(
        &identity.data,
        &Arc::downgrade(&current.root.data)
    ));
    let mut zero = b.reserve_capacity(0).unwrap();
    let reused = foreign.clone().into_memory_budget(&mut zero).unwrap();
    assert!(Arc::ptr_eq(&current.root, &reused.root));
    drop(current);
    assert!(reused.memory_budget_retained_bytes(&b).is_some());
    drop((reused, zero));
    assert_eq!(b.used_bytes(), 0);
    assert_eq!(b.retained_nodes(), 0);
    assert!(foreign.memory_budget_retained_bytes(&a).is_some());
    drop(foreign);
    assert_eq!(a.used_bytes(), 0);
    assert_eq!(a.retained_nodes(), 0);
    assert_eq!(identity.data.strong_count(), 0);
}

#[test]
fn first_admission_preserves_byte_node_and_overflow_rejection_before_publication() {
    for denial in [0, 1, 2] {
        let raw = fresh(true);
        let identity = raw.frozen_storage_identity();
        let account = crate::ManagedMemoryBudget::new(1_000_000);
        let bytes = raw
            .unshared_memory_budget_admission_bytes(&account)
            .unwrap();
        let nodes = raw.memory_budget_ownership_witness().unwrap().1;
        let overflow = (denial == 2).then(|| account.retain_snapshot_nodes(u64::MAX).unwrap());
        let budget = account.with_limits(bytes, if denial == 1 { nodes - 1 } else { u64::MAX });
        let capacity = if denial == 0 { bytes - 1 } else { bytes };
        let mut claim = budget.reserve_capacity(capacity).unwrap();
        let error = raw.into_memory_budget(&mut claim).unwrap_err();
        match denial {
            0 => assert!(
                matches!(error, crate::MemoryRuntimeError::BudgetExceeded { requested, limit, .. } if requested == bytes && limit == capacity)
            ),
            1 => assert!(
                matches!(error, crate::MemoryRuntimeError::BudgetExceeded { requested, limit, .. } if requested == nodes && limit == nodes - 1)
            ),
            _ => assert!(matches!(
                error,
                crate::MemoryRuntimeError::AccountingInvariantViolation { .. }
            )),
        }
        assert_eq!(claim.capacity_bytes(), capacity);
        assert_eq!(
            account.retained_snapshot_nodes(),
            if denial == 2 { u64::MAX } else { 0 }
        );
        assert_eq!(identity.data.strong_count(), 0);
        drop((claim, overflow));
        assert_eq!(account.used_bytes(), 0);
        assert_eq!(account.retained_nodes(), 0);
        let raw = fresh(true);
        let exact = account.with_limits(bytes, nodes);
        let value = admit(raw, &exact, false);
        assert_eq!(exact.used_bytes(), bytes);
        assert_eq!(exact.retained_nodes(), nodes);
        drop(value);
        assert_eq!(account.used_bytes(), 0);
        assert_eq!(account.retained_nodes(), 0);
    }
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
#[test]
fn concurrent_unique_wrappers_deduplicate_shared_data_in_one_account() {
    // Independent fresh wrappers share immutable data; no wrapper is shared.
    // The registry's existing synchronized recheck remains authoritative.
    let raw = fresh(true);
    let sibling = Value {
        schema: raw.schema,
        schema_key: raw.schema_key,
        shape: raw.shape.clone(),
        schemas: raw.schemas.clone(),
        root: Arc::new(FrozenSnapshotStorage {
            data: raw.root.data.clone(),
            memory_budget: None,
        }),
    };
    let account = crate::ManagedMemoryBudget::new(1_000_000);
    let bytes = raw
        .unshared_memory_budget_admission_bytes(&account)
        .unwrap();
    let nodes = raw.memory_budget_ownership_witness().unwrap().1;
    let first = account.reserve_capacity(bytes).unwrap();
    let second = account.reserve_capacity(bytes).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let run = |value: Value,
               mut claim: crate::ManagedMemoryReservation,
               barrier: Arc<std::sync::Barrier>| {
        std::thread::spawn(move || {
            barrier.wait();
            let value = value.into_memory_budget(&mut claim).unwrap();
            (value, claim)
        })
    };
    let a = run(raw, first, barrier.clone());
    let b = run(sibling, second, barrier);
    let (a, first) = a.join().unwrap();
    let (b, second) = b.join().unwrap();
    assert!(Arc::ptr_eq(&a.root, &b.root));
    assert_eq!(account.retained_nodes(), nodes);
    assert_eq!(first.capacity_bytes() + second.capacity_bytes(), bytes);
    drop((a, b, first, second));
    assert_eq!(account.used_bytes(), 0);
    assert_eq!(account.retained_nodes(), 0);
}

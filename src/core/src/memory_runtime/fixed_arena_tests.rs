use super::*;
use crate::{IntegerWidth, PlannedSlotKind, ScalarMemoryKind};

fn words(count: usize) -> FixedHostArenaPlan<u64> {
    FixedHostArenaPlan::new(
        count,
        PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64)),
    )
    .unwrap()
}

#[test]
fn fixed_host_arenas_keep_exact_admission_initialization_and_retirement() {
    for count in [0, 1, 7, 8, 9, 63, 64, 65, 128, 4097] {
        let plan = words(count);
        let budget = ManagedMemoryBudget::new(64 << 20);
        let admitted = MemoryDomain::host_arena_admission_bytes((count * 8) as u64).unwrap();
        assert_eq!(plan.admission_bytes().unwrap(), admitted);
        let (mut values, _) = plan.materialize(&budget).unwrap();
        assert_eq!(budget.used_bytes(), admitted);
        assert!(values.iter().all(|n| *n == 0));
        for (i, n) in values.iter_mut().enumerate() {
            *n = i as u64;
        }
        assert!(values.iter().enumerate().all(|(i, n)| *n == i as u64));
        drop(values);
        assert_eq!(budget.used_bytes(), 0);
        if admitted > 0 {
            assert!(
                plan.materialize(&budget.with_limits(admitted - 1, u64::MAX))
                    .is_err()
            );
            assert_eq!(budget.used_bytes(), 0);
        }
    }
}

#[test]
fn fixed_host_arena_realizations_never_alias_or_reuse_live_identities() {
    let plan = words(65);
    let bytes = plan.admission_bytes().unwrap();
    let budget = ManagedMemoryBudget::new(bytes * 2);
    let (mut a, aid) = plan.materialize(&budget).unwrap();
    let (b, bid) = plan.materialize(&budget).unwrap();
    assert_ne!(aid, bid);
    assert_ne!(a.as_ptr(), b.as_ptr());
    a[0] = 7;
    assert_eq!(b[0], 0);
    assert!(plan.materialize(&budget).is_err());
    drop(a);
    let (c, cid) = plan.materialize(&budget).unwrap();
    assert_ne!(cid, aid);
    assert_ne!(cid, bid);
    assert_eq!(c[0], 0);
    drop((b, c));
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn fixed_host_arena_zero_length_still_has_a_fresh_identity() {
    let plan = words(0);
    let budget = ManagedMemoryBudget::new(0);
    let (a, aid) = plan.materialize(&budget).unwrap();
    let (b, bid) = plan.materialize(&budget).unwrap();
    assert!(a.is_empty() && b.is_empty());
    assert_ne!(aid, bid);
    assert_eq!(budget.used_bytes(), 0);
    // Zero storage retains the existing no-claim behavior even through a
    // stricter view while unrelated admitted owners remain alive.
    let budget = ManagedMemoryBudget::new(8);
    let _old = budget.reserve_capacity(8).unwrap();
    assert!(plan.materialize(&budget.with_limits(0, u64::MAX)).is_ok());
    assert_eq!(budget.used_bytes(), 8);
}

#[test]
fn fixed_host_arena_rejects_wrong_element_slots_and_overflow_before_allocation() {
    assert!(FixedHostArenaPlan::<u64>::new(1, PlannedSlotKind::CanonicalValueHandle).is_err());
    assert!(
        FixedHostArenaPlan::<Option<crate::Value>>::new(
            1,
            PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64))
        )
        .is_err()
    );
    assert!(
        FixedHostArenaPlan::<u64>::new(
            usize::MAX,
            PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64))
        )
        .is_err()
    );
}

#[test]
fn fixed_host_arena_pointer_containing_elements_are_initialized_and_destroyed() {
    let budget = ManagedMemoryBudget::new(1 << 20);
    let plan = FixedHostArenaPlan::<String>::new(4, PlannedSlotKind::StringHeader).unwrap();
    let (mut strings, _) = plan.materialize(&budget).unwrap();
    assert!(strings.iter().all(String::is_empty));
    strings[1] = "owned Rust string".into();
    assert_eq!(&strings[1], "owned Rust string");
    drop(strings);
    assert_eq!(budget.used_bytes(), 0);
    let plan =
        FixedHostArenaPlan::<Option<crate::Value>>::new(4, PlannedSlotKind::CanonicalValueHandle)
            .unwrap();
    let (values, _) = plan.materialize(&budget).unwrap();
    assert!(values.iter().all(Option::is_none));
    drop(values);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn fixed_host_arena_prefix_retains_full_admission_and_fresh_physical_ownership() {
    for count in [0, 1, 9, 65, 626] {
        let plan = words(count);
        let admitted = plan.admission_bytes().unwrap();
        for prefix in [0, count.min(1), count / 2, count] {
            let budget = ManagedMemoryBudget::new(admitted * 2);
            let (mut a, aid) = plan.materialize_prefix(prefix, &budget).unwrap();
            let (b, bid) = plan.materialize_prefix(prefix, &budget).unwrap();
            assert_eq!((a.len(), b.len()), (prefix, prefix));
            assert_ne!(aid, bid);
            assert_eq!(budget.used_bytes(), admitted * 2);
            assert_eq!(
                (budget.retained_nodes(), budget.retained_snapshot_nodes()),
                (0, 0)
            );
            assert!(a.iter().chain(b.iter()).all(|word| *word == 0));
            if prefix > 0 {
                assert_ne!(a.as_ptr(), b.as_ptr());
                a[prefix - 1] = 17;
                assert_eq!(b[prefix - 1], 0);
            }
            if admitted > 0 {
                assert!(plan.materialize_prefix(prefix, &budget).is_err());
            }
            drop(a);
            assert_eq!(budget.used_bytes(), admitted);
            let (c, cid) = plan.materialize_prefix(prefix, &budget).unwrap();
            assert_ne!(cid, aid);
            assert_ne!(cid, bid);
            assert!(c.iter().all(|word| *word == 0));
            drop((b, c));
            assert_eq!(budget.used_bytes(), 0);
            if admitted > 0 {
                let exact = budget.with_limits(admitted, u64::MAX);
                let (values, _) = plan.materialize_prefix(prefix, &exact).unwrap();
                assert_eq!(budget.used_bytes(), admitted);
                drop(values);
                let short = budget.with_limits(admitted - 1, u64::MAX);
                assert!(matches!(
                    plan.materialize_prefix(prefix, &short),
                    Err(MemoryRuntimeError::BudgetExceeded { requested, limit, .. })
                        if requested == admitted && limit == admitted - 1
                ));
                assert_eq!(budget.used_bytes(), 0);
            }
        }
        let budget = ManagedMemoryBudget::new(admitted);
        for invalid in [count + 1, usize::MAX] {
            assert!(matches!(
                plan.materialize_prefix(invalid, &budget),
                Err(MemoryRuntimeError::InvalidLayout { .. })
            ));
            assert_eq!(budget.used_bytes(), 0);
        }
    }
}

#[test]
fn fixed_host_arena_prefix_drops_only_initialized_value_owners_and_preserves_aliases() {
    use crate::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use crate::{SchemaBody, SchemaDraft, SchemaTableBuilder};
    let mut builder = SchemaTableBuilder::new();
    let handle = builder
        .insert(
            SchemaDraft {
                body: SchemaBody::String,
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = builder.finish().unwrap();
    let value = ValueDraft {
        schema: build.resolve(handle).unwrap(),
        shape_values: Box::new([]),
        data: ValueDataDraft::String("prefix alias".into()),
    }
    .finalize(&SnapshotValidationContext::new(&build.table))
    .unwrap();
    let budget = ManagedMemoryBudget::new(1 << 20);
    let value = value.admit_into_memory_budget(&budget).unwrap();
    let alias = value.clone();
    let baseline = (budget.used_bytes(), budget.retained_snapshot_nodes());
    let plan =
        FixedHostArenaPlan::<Option<crate::Value>>::new(626, PlannedSlotKind::CanonicalValueHandle)
            .unwrap();
    let full = plan.admission_bytes().unwrap();
    let (mut prefix, _) = plan.materialize_prefix(2, &budget).unwrap();
    assert_eq!(prefix.len(), 2);
    assert!(prefix.iter().all(Option::is_none));
    prefix[0] = Some(value);
    prefix[1] = Some(alias.clone());
    assert_eq!(budget.used_bytes(), baseline.0 + full);
    assert_eq!(budget.retained_snapshot_nodes(), baseline.1);
    drop(prefix);
    assert_eq!(
        (budget.used_bytes(), budget.retained_snapshot_nodes()),
        baseline
    );
    assert_eq!(
        alias.canonical_data_draft().unwrap(),
        ValueDataDraft::String("prefix alias".into())
    );
    let (empty, _) = plan.materialize_prefix(0, &budget).unwrap();
    assert!(empty.is_empty());
    assert_eq!(budget.used_bytes(), baseline.0 + full);
    drop(empty);
    assert_eq!(
        (budget.used_bytes(), budget.retained_snapshot_nodes()),
        baseline
    );
    drop(alias);
    assert_eq!((budget.used_bytes(), budget.retained_nodes()), (0, 0));
}

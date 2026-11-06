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

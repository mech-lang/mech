use super::*;
use mech_core::ManagedMemoryBudget;
use std::cell::Cell;

#[test]
fn zero_exact_short_excess_and_nonempty_backing_precede_initialization() {
    for count in [0, 1, 7] {
        let initialized = Cell::new(0);
        let values = allocate_with(
            count,
            |count| Ok(Vec::with_capacity(count)),
            |_| {
                initialized.set(initialized.get() + 1);
                3_u64
            },
        )
        .unwrap();
        assert_eq!((values.len(), values.capacity()), (count, count));
        assert_eq!(initialized.get(), count);
    }
    for capacity in [0, 1, 3] {
        let initialized = Cell::new(0);
        let result = allocate_with(
            2,
            |_| Ok(Vec::with_capacity(capacity)),
            |_| {
                initialized.set(initialized.get() + 1);
                3_u64
            },
        );
        assert!(result.is_err());
        assert_eq!(initialized.get(), 0);
    }
    let initialized = Cell::new(false);
    let mut nonempty = Vec::with_capacity(2);
    nonempty.push(1_u64);
    assert!(
        allocate_with(
            2,
            |_| Ok(nonempty),
            |_| {
                initialized.set(true);
                3_u64
            }
        )
        .is_err()
    );
    assert!(!initialized.get());
}

#[test]
fn rejected_owned_backing_drops_before_its_original_claim() {
    struct Probe<'a>(&'a Cell<u32>, &'a ManagedMemoryBudget);
    impl Drop for Probe<'_> {
        fn drop(&mut self) {
            assert_eq!(self.1.used_bytes(), 4096);
            self.0.set(self.0.get() + 1);
        }
    }
    let memory = ManagedMemoryBudget::new(4096);
    let claim = memory.reserve_capacity(4096).unwrap();
    let drops = Cell::new(0);
    let mut values = Vec::with_capacity(3);
    values.push(Probe(&drops, &memory));
    let result = allocate_with(2, |_| Ok(values), |_| panic!("must not initialize"));
    assert!(result.is_err());
    assert_eq!(drops.get(), 1);
    assert_eq!(memory.used_bytes(), 4096);
    drop(result);
    drop(claim);
    assert_eq!(memory.used_bytes(), 0);
}

//! Deterministic allocator responses to the SAME production storage guard.
use super::*;
use mech_core::ManagedMemoryBudget;
use std::cell::Cell;

#[test]
fn zero_exact_short_and_excess_capacity_are_checked_before_initialization() {
    let empty = allocate_with::<u64>(0, |_| Ok(Vec::new()), |_| panic!("zero Slot"));
    assert!(empty.unwrap().is_empty());
    for mode in ["exact", "short", "excess"] {
        let values = Vec::<u64>::with_capacity(4);
        let actual = values.capacity();
        let count = match mode {
            "exact" => actual,
            "short" => actual + 1,
            _ => actual - 1,
        };
        let initialized = Cell::new(0);
        let result = allocate_with(
            count,
            |_| Ok(values),
            |ordinal| {
                initialized.set(initialized.get() + 1);
                ordinal as u64
            },
        );
        if mode == "exact" {
            let values = result.unwrap();
            assert_eq!(values.len(), count);
            assert_eq!(values.capacity(), count);
            assert_eq!(initialized.get(), count);
        } else {
            assert!(matches!(result, Err(ResidentKernelError::InvalidShape)));
            assert_eq!(initialized.get(), 0);
        }
    }
    let mut nonempty = Vec::<u64>::with_capacity(1);
    let count = nonempty.capacity();
    nonempty.push(7);
    assert!(allocate_with(count, |_| Ok(nonempty), |_| panic!("not empty")).is_err());
}

#[test]
fn rejected_allocation_destroys_existing_contents_before_the_claim_releases() {
    struct Witness<'a> {
        memory: &'a ManagedMemoryBudget,
        observed: &'a Cell<u64>,
        drops: &'a Cell<usize>,
    }
    impl Drop for Witness<'_> {
        fn drop(&mut self) {
            self.observed.set(self.memory.used_bytes());
            self.drops.set(self.drops.get() + 1);
        }
    }
    let memory = ManagedMemoryBudget::new(4096);
    let reservation = memory.reserve_capacity(4096).unwrap();
    let observed = Cell::new(0);
    let drops = Cell::new(0);
    let initialized = Cell::new(0);
    let mut values = Vec::with_capacity(2);
    let count = values.capacity() - 1;
    // This generic destructor witness is not a constructed/published Slot.
    values.push(Witness {
        memory: &memory,
        observed: &observed,
        drops: &drops,
    });
    let result = allocate_with(
        count,
        |_| Ok(values),
        |_| {
            initialized.set(initialized.get() + 1);
            Witness {
                memory: &memory,
                observed: &observed,
                drops: &drops,
            }
        },
    );
    assert!(matches!(result, Err(ResidentKernelError::InvalidShape)));
    assert_eq!(initialized.get(), 0);
    assert_eq!(drops.get(), 1);
    assert_eq!(
        observed.get(),
        4096,
        "actual Vec contents drop before the claim"
    );
    assert_eq!(memory.used_bytes(), 4096);
    drop(reservation);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn caller_claim_exact_and_one_short_do_not_retry_the_allocator() {
    for (limit, passes) in [(4096, true), (4095, false)] {
        let memory = ManagedMemoryBudget::new(limit);
        let allocations = Cell::new(0);
        let result = memory.reserve_capacity(4096).map(|reservation| {
            let result = allocate_with::<u64>(
                0,
                |_| {
                    allocations.set(allocations.get() + 1);
                    Ok(Vec::new())
                },
                |_| unreachable!(),
            );
            assert!(result.is_ok());
            drop(result);
            assert_eq!(memory.used_bytes(), 4096);
            drop(reservation);
        });
        assert_eq!(result.is_ok(), passes);
        assert_eq!(allocations.get(), usize::from(passes));
        assert_eq!(memory.used_bytes(), 0);
    }
}

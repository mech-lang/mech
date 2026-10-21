//! Actual admitted UTF-8 cells and bounded row loans, not a language evaluator.
use super::*;
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetDimension, MemoryBudgetLimits, ResidentKernelError,
};

fn limits() -> MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_compute_work = Some(1 << 46);
    limits.max_temporary_bytes = Some(1024 * 1024);
    limits
}
fn work(memory: &ManagedMemoryBudget) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(1 << 48, 1 << 32, memory).unwrap()
}
fn store<'a>(
    work: &'a NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
) -> NativeTextLineChunks {
    super::super::prepare(1024, 128, limits, work, 0, None).unwrap()
}

#[test]
fn row_loans_use_real_global_ordinals_and_utf8_owners_until_the_loan_drops() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut owner = store(&ledger, limits());
    owner.push("discard\nA\r\n猫\ne\u{301}\nlast\r").unwrap();
    owner.retain_from(2).unwrap();
    let baseline = memory.used_bytes();
    for first in 2..=5 {
        for end in first..=5 {
            let window = owner.borrow_window(first, end).unwrap();
            assert_eq!(window.first(), first);
            assert_eq!(window.end(), end);
            assert_eq!(window.available_lines(), 4);
            assert!(window.final_facts().is_none());
            assert_eq!(window.rows().len(), (end - first) as usize);
            assert_eq!(window.line(first - 1), None);
            assert_eq!(window.line(end), None);
            for (offset, row) in window.rows().iter().enumerate() {
                let ordinal = first + offset as u64;
                let actual = owner.line(ordinal).unwrap();
                assert_eq!(*row, [actual.as_ptr() as u64, actual.len() as u64]);
                assert_eq!(window.line(ordinal), Some(actual));
            }
            assert_eq!(
                window.is_ascii(),
                (first..end).all(|ordinal| owner.line(ordinal).unwrap().is_ascii())
            );
            drop(window);
            assert_eq!(memory.used_bytes(), baseline);
        }
    }
    owner.retain_range(2, 4).unwrap();
    assert!(owner.borrow_window(1, 2).is_err());
    assert!(owner.borrow_window(2, 5).is_err());
    assert!(owner.borrow_window(0, 0).is_err());
    assert!(owner.borrow_window(4, 3).is_err());
    owner.push("\nsuffix\n").unwrap();
    let facts = owner.finish().unwrap();
    let empty = owner
        .borrow_window(facts.lines() + 1, facts.lines() + 1)
        .unwrap();
    assert!(empty.rows().is_empty());
    assert_eq!(empty.final_facts(), Some(facts));
    drop(empty);
    drop((owner, ledger));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn producer_ascii_proof_preserves_cell_qualification_and_exact_window_work() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut debits = Vec::new();
    // Equal byte lengths and the same selected cell: the producer-wide proof
    // and mixed-source inspection must debit precisely the same row/payload
    // work, with neither path renewing its existing source allowance.
    for source in ["AAAA\nB\r\r\n", "éé\nB\r\r\n"] {
        let mut owner = store(&ledger, limits());
        for fragment in source.split_inclusive('\n') {
            owner.push(fragment).unwrap();
        }
        let before = ledger.used().0;
        let window = owner.borrow_window(2, 3).unwrap();
        assert!(window.is_ascii());
        assert_eq!(window.line(2), Some("B\r"));
        debits.push(ledger.used().0 - before);
        drop(window);
        assert_eq!(owner.line(2), Some("B\r"));
        drop(owner);
    }
    assert_eq!(debits[0], debits[1]);
    drop(ledger);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn many_completed_lines_do_not_force_a_full_input_pointer_table() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut owner = super::super::prepare(64 * 1024, 128, limits(), &ledger, 0, None).unwrap();
    for _ in 0..1024 {
        owner.push("a\nb\nc\nd\n").unwrap();
    }
    let held = owner.retained_capacity_bytes();
    let window = owner.borrow_window(4093, 4097).unwrap();
    assert_eq!(window.rows().len(), 4);
    assert_eq!(window.first(), 4093);
    assert!(window.retained_capacity_bytes() - held < 1024);
    assert_eq!(window.line(4093), Some("a"));
    drop(window);
    assert_eq!(owner.retained_capacity_bytes(), held);
    owner.retain_from(4097).unwrap();
    assert_eq!(owner.line(4096), None);
    drop((owner, ledger));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn window_capacity_is_reserved_before_allocation_and_exact_limits_do_not_leak() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let ledger_bytes = memory.used_bytes();
    let mut owner = store(&ledger, limits());
    owner.push("one\ntwo\nthree\n").unwrap();
    let window = owner.borrow_window(2, 4).unwrap();
    let required = owner.peak_capacity_bytes();
    drop(window);
    drop((owner, ledger));
    assert_eq!(memory.used_bytes(), 0);
    for local in [false, true] {
        for short in [false, true] {
            let memory = ManagedMemoryBudget::new(if local {
                1024 * 1024
            } else {
                ledger_bytes + required - u64::from(short)
            });
            let ledger = work(&memory);
            let mut limits = limits();
            if local {
                limits.max_temporary_bytes = Some(required - u64::from(short));
            }
            let mut owner = store(&ledger, limits);
            owner.push("one\ntwo\nthree\n").unwrap();
            let result = owner.borrow_window(2, 4);
            assert_eq!(result.is_err(), short);
            drop(result);
            assert_eq!(owner.line(2), Some("two"));
            drop((owner, ledger));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn every_window_uses_the_same_spent_compute_allowance_and_foreign_scopes_fail() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut owner = store(&ledger, limits());
    owner.push("a\nb\n").unwrap();
    drop(owner.borrow_window(1, 3).unwrap());
    let required = ledger.used().0;
    drop((owner, ledger));
    for short in [false, true] {
        let ledger = work(&memory);
        let mut limits = limits();
        limits.max_compute_work = Some(required - u64::from(short));
        let mut owner = store(&ledger, limits);
        owner.push("a\nb\n").unwrap();
        let result = owner.borrow_window(1, 3);
        if short {
            assert!(matches!(
                &result,
                Err(InvokeError::Kernel(ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    ..
                }))
            ));
            drop(result);
        } else {
            drop(result.unwrap());
        }
        assert!(
            owner.borrow_window(1, 3).is_err(),
            "a fresh window cannot renew the producer cap"
        );
        drop((owner, ledger));
        assert_eq!(memory.used_bytes(), 0);
    }
    let ledger = work(&memory);
    let foreign = work(&memory);
    let mut owner = store(&ledger, limits());
    owner.push("a\n").unwrap();
    assert!(
        foreign
            .with_scope(|| owner.borrow_window(1, 2).map(drop))
            .unwrap()
            .is_err()
    );
    drop((owner, ledger, foreign));
    assert_eq!(memory.used_bytes(), 0);
}

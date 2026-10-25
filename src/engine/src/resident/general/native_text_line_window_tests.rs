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
            assert_eq!(window.row_count(), (end - first) as usize);
            assert_eq!(window.line(first - 1), None);
            assert_eq!(window.line(end), None);
            for (offset, row) in window
                .rows()
                .expect("mixed-source cells retain the row fallback")
                .iter()
                .enumerate()
            {
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
    assert!(empty.row_count() == 0);
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
fn cached_payload_preserves_exact_window_debits_across_fragmentation_and_retention() {
    fn check_windows(
        owner: &NativeTextLineChunks,
        ledger: &NativeLibraryWorkBudget,
        memory: &ManagedMemoryBudget,
        lines: &[&str],
        first: u64,
        end: u64,
    ) {
        let retained = memory.used_bytes();
        for start in first..=end {
            for stop in start..=end {
                let count = stop - start;
                let expected = &lines[(start - 1) as usize..(stop - 1) as usize];
                let payload = expected.iter().map(|line| line.len() as u64).sum::<u64>();
                assert_eq!(owner.window_payload_bytes(start, stop).unwrap(), payload);
                let before = ledger.used();
                assert_eq!(
                    owner.final_range_payload(start, stop),
                    (start < stop).then_some(payload)
                );
                assert_eq!(ledger.used(), before, "the selected visit prepays geometry");
                assert_eq!(
                    memory.used_bytes(),
                    retained,
                    "no row loan or cache allocation"
                );
                let before = ledger.used().0;
                let window = owner.borrow_window(start, stop).unwrap();
                let claim_bytes = count * core::mem::size_of::<[u64; 2]>() as u64
                    + core::mem::size_of::<NativeTextLineWindow<'_>>() as u64
                    + 128;
                assert_eq!(
                    ledger.used().0 - before,
                    count * 256 + 4096 + payload * 4 + claim_bytes * 8 + 128,
                    "the cached and partial-edge paths must retain the exact original charges"
                );
                assert_eq!(window.row_count(), expected.len());
                assert_eq!(
                    window.is_ascii(),
                    expected.iter().all(|line| line.is_ascii())
                );
                for (offset, line) in expected.iter().enumerate() {
                    assert_eq!(window.line(start + offset as u64), Some(*line));
                }
                drop(window);
                assert_eq!(memory.used_bytes(), retained);
            }
        }
    }

    for source in ["", "\n", "\r\n", "a\r\r\n", "é\r\n猫\nlast\r", "a\n\nb\n"] {
        let lines = source.lines().collect::<Vec<_>>();
        for split in (0..=source.len()).filter(|index| source.is_char_boundary(*index)) {
            let memory = ManagedMemoryBudget::new(1024 * 1024);
            let ledger = work(&memory);
            let mut owner = store(&ledger, limits());
            owner.push(&source[..split]).unwrap();
            owner.push(&source[split..]).unwrap();
            let facts = owner.finish().unwrap();
            assert_eq!(facts.lines(), lines.len() as u64);
            let end = facts.lines() + 1;
            check_windows(&owner, &ledger, &memory, &lines, 1, end);
            if lines.len() >= 3 {
                owner.retain_from(2).unwrap();
                check_windows(&owner, &ledger, &memory, &lines, 2, end);
                owner.retain_range(2, end - 1).unwrap();
                check_windows(&owner, &ledger, &memory, &lines, 2, end - 1);
            }
            drop((owner, ledger));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn final_range_payload_requires_eof_and_the_exact_retained_extent() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut owner = store(&ledger, limits());
    owner.push("discard\nA\r\n猫\nlast\r").unwrap();
    assert_eq!(owner.final_range_payload(2, 4), None);
    owner.finish().unwrap();
    assert_eq!(owner.final_range_payload(2, 4), Some(4));
    assert_eq!(owner.final_range_payload(4, 5), Some(5));
    owner.retain_from(2).unwrap();
    assert_eq!(owner.final_range_payload(1, 3), None);
    owner.retain_range(2, 4).unwrap();
    let before = (ledger.used(), memory.used_bytes());
    for (first, end) in [(0, 2), (2, 2), (3, 2), (2, 5), (5, 6), (1, u64::MAX)] {
        assert_eq!(owner.final_range_payload(first, end), None);
    }
    assert_eq!(owner.final_range_payload(2, 4), Some(4));
    assert_eq!((ledger.used(), memory.used_bytes()), before);
    drop((owner, ledger));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn immutable_exact_offset_backing_keeps_its_allocation() {
    // Offsets checks both capacity and length before this conversion. There
    // is no spare capacity to shrink and no second live backing allocation.
    for count in [1, 3, 1024] {
        let mut values = Vec::<u32>::new();
        values.try_reserve_exact(count).unwrap();
        assert_eq!(values.capacity(), count);
        values.extend(0..count as u32);
        let pointer = values.as_ptr();
        let boxed = values.into_boxed_slice();
        assert_eq!(boxed.as_ptr(), pointer);
        assert_eq!(boxed.len(), count);
    }
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
    assert_eq!(window.row_count(), 4);
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

#[test]
fn compact_offset_loans_cover_chunk_transitions_without_materialized_rows() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    for source in ["A\nB\r\nC\r\r\nlast\r", "\n\r\n\n", "ABC", "ABC\n", ""] {
        let reference = source.lines().collect::<Vec<_>>();
        for split in 0..=source.len() {
            let mut owner = store(&ledger, limits());
            owner.push(&source[..split]).unwrap();
            owner.push(&source[split..]).unwrap();
            owner.finish().unwrap();
            for first in 1..=reference.len() as u64 + 1 {
                for end in first..=reference.len() as u64 + 1 {
                    let window = owner.borrow_window(first, end).unwrap();
                    let (pointer, count) = window.native_rows();
                    assert_eq!(count, end - first);
                    assert_eq!(window.row_count(), count as usize);
                    if window.direct {
                        assert_eq!(pointer & 1, 1);
                        assert!(window.rows().is_none());
                        assert_eq!(
                            window.rows.capacity(),
                            0,
                            "compact loans create no row backing"
                        );
                        assert_eq!(
                            window
                                .segments
                                .iter()
                                .map(|segment| segment[4])
                                .sum::<u64>(),
                            count
                        );
                    } else {
                        assert_eq!(pointer & 1, 0);
                        assert_eq!(window.rows().unwrap().len(), count as usize);
                    }
                    for ordinal in first..end {
                        assert_eq!(
                            window.line(ordinal),
                            Some(reference[(ordinal - 1) as usize])
                        );
                    }
                }
            }
        }
    }
    let mut owner = store(&ledger, limits());
    for text in ["a\n", "b\n", "c\n"] {
        owner.push(text).unwrap();
    }
    owner.finish().unwrap();
    let window = owner.borrow_window(1, 4).unwrap();
    assert!(
        window.rows().is_some(),
        "larger physical windows retain checked fallback"
    );
    assert_eq!(window.native_rows().0 & 1, 0);
    drop(window);
    drop((owner, ledger));
    assert_eq!(memory.used_bytes(), 0);
}

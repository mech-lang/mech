//! Compare owned physical geometry to std::str::lines and retain it after its
//! directory dies. These are storage/lifetime tests, not a source evaluator.
use super::*;
use crate::resident::budget::NativeLibraryWorkBudget;
use mech_core::{ManagedMemoryBudget, TargetMemoryProfile};

fn store(
    source: &str,
    split: usize,
    memory: &ManagedMemoryBudget,
) -> (NativeTextLineChunks, NativeLibraryWorkBudget) {
    let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, memory).unwrap();
    let mut limits = TargetMemoryProfile::current_resident_cpu().unwrap().limits;
    limits.max_compute_work = Some(1 << 46);
    limits.max_temporary_bytes = Some(8 * 1024 * 1024);
    let mut chunks = super::super::prepare(1 << 20, 4096, limits, &work, 0, None).unwrap();
    chunks.push(&source[..split]).unwrap();
    chunks.push(&source[split..]).unwrap();
    chunks.finish().unwrap();
    (chunks, work)
}

fn payload(loan: &NativeTextLineJoinChunk) -> Vec<u8> {
    let w = loan.geometry();
    // SAFETY: the opaque loan retains these actual immutable allocations,
    // including after the original directory and work handle have dropped.
    let bytes = unsafe { core::slice::from_raw_parts(w[0] as *const u8, w[1] as usize) };
    if w[2] == 0 {
        assert_eq!((w[3], w[4], w[5]), (0, 1, 0));
        return bytes.to_vec();
    }
    let offsets = unsafe { core::slice::from_raw_parts(w[2] as *const u32, w[5] as usize) };
    let mut output = Vec::new();
    for index in w[3] as usize..(w[3] + w[4]) as usize {
        let start = if index == 0 {
            0
        } else {
            offsets[index - 1] as usize + 1
        };
        let lf = offsets[index] as usize;
        assert_eq!(bytes[lf], b'\n');
        let end = lf - usize::from(lf > start && bytes[lf - 1] == b'\r');
        output.extend_from_slice(&bytes[start..end]);
    }
    output
}

#[test]
fn owned_join_loans_keep_exact_ascii_ranges_and_accounts_after_directory_retirement() {
    for source in [
        "",
        "\n",
        "a\nb\r\n\nc",
        "\r\n\r\r\nend\r",
        "ab\ncd\r\nef\nlast\r",
    ] {
        let expected = source.lines().collect::<Vec<_>>();
        for split in 0..=source.len() {
            for first in 0..=expected.len() {
                for end in first..=expected.len() {
                    let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
                    let (mut chunks, work) = store(source, split, &memory);
                    let before = work.used();
                    let mut next = first as u64 + 1;
                    let mut loans = Vec::new();
                    while let Some(loan) = chunks.take_join_chunk(next, end as u64 + 1).unwrap() {
                        assert!(loan.following() > next);
                        assert!(chunks.line(next).is_none());
                        next = loan.following();
                        loans.push(loan);
                    }
                    assert_eq!(next, end as u64 + 1);
                    assert_eq!(
                        work.used(),
                        before,
                        "storage transfer grants no new allowance"
                    );
                    drop((chunks, work));
                    let mut actual = Vec::new();
                    for loan in &loans {
                        let bytes = payload(loan);
                        assert_eq!(bytes.len() as u64, loan.payload_bytes());
                        actual.extend(bytes);
                    }
                    assert_eq!(actual, expected[first..end].concat().as_bytes());
                    drop(loans);
                    assert_eq!(memory.used_bytes(), 0);
                    assert_eq!(memory.retained_nodes(), 0);
                }
            }
        }
    }
}

#[test]
fn owned_join_loans_reject_unfinished_unicode_foreign_or_discarded_ranges() {
    {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
        let mut limits = TargetMemoryProfile::current_resident_cpu().unwrap().limits;
        limits.max_compute_work = Some(1 << 46);
        limits.max_temporary_bytes = Some(8 * 1024 * 1024);
        let mut chunks = super::super::prepare(4096, 4096, limits, &work, 0, None).unwrap();
        chunks.push("a\n").unwrap();
        let before = work.used();
        assert!(chunks.take_join_chunk(1, 2).is_err());
        assert_eq!(chunks.line(1), Some("a"));
        assert_eq!(work.used(), before);
        drop((chunks, work));
        assert_eq!(memory.used_bytes(), 0);
    }
    for source in ["a\nb\n", "a\né\n"] {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let (mut chunks, work) = store(source, 1, &memory);
        let before = work.used();
        assert!(chunks.take_join_chunk(0, 1).is_err());
        assert!(chunks.take_join_chunk(3, 2).is_err());
        assert!(chunks.take_join_chunk(1, 4).is_err());
        if !source.is_ascii() {
            assert!(chunks.take_join_chunk(1, 3).is_err());
            assert_eq!(chunks.line(2), Some("é"));
        } else {
            chunks.retain_from(2).unwrap();
            let after_retention = work.used();
            assert!(chunks.take_join_chunk(1, 3).is_err());
            assert_eq!(work.used(), after_retention);
        }
        if !source.is_ascii() {
            assert_eq!(work.used(), before);
        }
        drop((chunks, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

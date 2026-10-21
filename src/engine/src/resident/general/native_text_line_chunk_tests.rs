//! Storage/ownership probes, never a Rust implementation of a Mech scanner.
use super::*;
use mech_core::{ManagedMemoryBudget, MemoryBudgetDimension, ResidentKernelError};

fn prepare(
    input_limit: u64,
    max_fragment: usize,
    limits: MemoryBudgetLimits,
    work: &NativeLibraryWorkBudget,
) -> Result<NativeTextLineChunks, InvokeError> {
    super::prepare(input_limit, max_fragment, limits, work, 0, None)
}

fn limits() -> MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_compute_work = Some(1 << 46);
    limits.max_temporary_bytes = Some(128 * 1024 * 1024);
    limits
}
fn work(memory: &ManagedMemoryBudget) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(1 << 48, 1 << 32, memory).unwrap()
}
fn finish_matches(store: &mut NativeTextLineChunks, source: &str) {
    let expected = source.lines().collect::<Vec<_>>();
    let facts = store.finish().unwrap();
    assert_eq!(facts.input_bytes(), source.len() as u64);
    assert_eq!(facts.lines(), expected.len() as u64);
    assert_eq!(
        facts.payload_bytes(),
        expected.iter().map(|line| line.len() as u64).sum::<u64>()
    );
    assert_eq!(facts.is_ascii(), source.is_ascii());
    assert_eq!(store.final_facts(), Some(facts));
    assert_eq!(store.line(0), None);
    for (index, line) in expected.iter().enumerate() {
        assert_eq!(
            store.line(index as u64 + 1),
            Some(*line),
            "{source:?}, line {index}"
        );
    }
    assert_eq!(store.line(facts.lines() + 1), None);
}

#[test]
fn every_short_utf8_split_matches_the_ordinary_lines_iterator() {
    let alphabet = ['x', '\r', '\n', 'é', '猫'];
    for length in 0..=4 {
        for mut ordinal in 0..alphabet.len().pow(length) {
            let mut source = String::new();
            for _ in 0..length {
                source.push(alphabet[ordinal % alphabet.len()]);
                ordinal /= alphabet.len();
            }
            let boundaries = source
                .char_indices()
                .map(|(index, _)| index)
                .chain([source.len()])
                .collect::<Vec<_>>();
            for &split in &boundaries {
                let memory = ManagedMemoryBudget::new(4 * 1024 * 1024);
                let work = work(&memory);
                let mut store = prepare(source.len() as u64, 32, limits(), &work).unwrap();
                store.push(&source[..split]).unwrap();
                assert!(store.final_facts().is_none());
                store.push(&source[split..]).unwrap();
                finish_matches(&mut store, &source);
                drop((store, work));
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}

#[test]
fn crlf_and_long_multibyte_lines_cross_arbitrary_fragment_boundaries() {
    for source in [
        "\r\n\r\r\n\nend\r".to_owned(),
        "é🦀".repeat(16_384) + "\r\n猫\r",
    ] {
        let memory = ManagedMemoryBudget::new(4 * 1024 * 1024);
        let work = work(&memory);
        let mut store = prepare(source.len() as u64, 4, limits(), &work).unwrap();
        for character in source.chars() {
            let mut bytes = [0; 4];
            store.push(character.encode_utf8(&mut bytes)).unwrap();
        }
        finish_matches(&mut store, &source);
        assert!(store.peak_capacity_bytes() < source.len() as u64 * 4 + 8192);
        drop((store, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn discarded_prefix_has_bounded_live_storage_independent_of_total_input() {
    let memory = ManagedMemoryBudget::new(64 * 1024);
    let work = work(&memory);
    let fragment = "abc\n".repeat(256);
    let rounds = 32 * 1024;
    let mut store = prepare(
        (fragment.len() * rounds) as u64,
        fragment.len(),
        limits(),
        &work,
    )
    .unwrap();
    for round in 0..rounds {
        store.push(&fragment).unwrap();
        let available = store.available_lines().unwrap();
        assert_eq!(available, (round as u64 + 1) * 256);
        assert_eq!(store.line(available), Some("abc"));
        store.retain_from(available + 1).unwrap();
        assert_eq!(store.line(available), None);
        assert!(store.retained_capacity_bytes() < 8192);
    }
    let facts = store.finish().unwrap();
    assert_eq!(facts.input_bytes(), 32 * 1024 * 1024);
    assert_eq!(facts.lines(), 8 * 1024 * 1024);
    assert_eq!(facts.payload_bytes(), 24 * 1024 * 1024);
    assert!(store.peak_capacity_bytes() < 16 * 1024);
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn sealed_range_keeps_real_unicode_lines_and_counts_unretained_suffix_without_backing() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = work(&memory);
    let mut store = prepare(128 * 1024, 4096, limits(), &work).unwrap();
    store.push("head\nmarker\n").unwrap();
    store.retain_from(3).unwrap();
    store.push("A\r\ne\u{301}\n猫\nend\n").unwrap();
    store.retain_range(3, 6).unwrap();
    assert_eq!(store.line(2), None);
    assert_eq!(store.line(3), Some("A"));
    assert_eq!(store.line(4), Some("e\u{301}"));
    assert_eq!(store.line(5), Some("猫"));
    assert_eq!(store.line(6), None);
    let held = store.retained_capacity_bytes();
    for _ in 0..16 {
        store.push(&"x".repeat(4096)).unwrap();
    }
    assert_eq!(store.retained_capacity_bytes(), held);
    store.push("\r").unwrap();
    store.push("\nlast\r").unwrap();
    let facts = store.finish().unwrap();
    assert_eq!(facts.lines(), 8);
    assert!(!facts.is_ascii());
    assert_eq!(store.retained_capacity_bytes(), held);
    assert_eq!(store.line(4), Some("e\u{301}"));
    assert!(
        store.retain_from(3).is_err(),
        "a sealed range cannot become open"
    );
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn empty_return_cannot_resurrect_a_dead_prefix_and_unterminated_suffix_stays_private() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = work(&memory);
    let mut store = prepare(1024, 128, limits(), &work).unwrap();
    store.push("a\nb\npartial").unwrap();
    assert_eq!(store.available_lines(), Some(2));
    assert_eq!(store.line(3), None);
    store.retain_from(3).unwrap();
    assert!(store.retain_from(2).is_err());
    assert!(store.retain_range(1, 2).is_err());
    store.retain_range(1, 1).unwrap();
    store.push("\r\nsuffix").unwrap();
    let facts = store.finish().unwrap();
    assert_eq!(facts.lines(), 4);
    assert_eq!(store.line(1), None);
    assert_eq!(store.line(3), None);
    assert!(store.push("").is_err());
    assert!(store.finish().is_err());
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn exact_and_one_short_global_and_local_capacities_include_replacement_peaks() {
    let source = "one\ntwo\n";
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let metadata = memory.used_bytes();
    let mut store = prepare(128, 128, limits(), &ledger).unwrap();
    store.push(source).unwrap();
    let peak = store.peak_capacity_bytes();
    let facts = store.finish().unwrap();
    drop((store, ledger));
    assert_eq!(memory.used_bytes(), 0);
    for local in [false, true] {
        for short in [false, true] {
            let memory = ManagedMemoryBudget::new(if local {
                1024 * 1024
            } else {
                metadata + peak - u64::from(short)
            });
            let ledger = work(&memory);
            let mut limits = limits();
            if local {
                limits.max_temporary_bytes = Some(peak - u64::from(short));
            }
            let mut store = prepare(128, 128, limits, &ledger).unwrap();
            let result = store.push(source);
            if short {
                assert!(result.is_err());
                assert_eq!(store.line(1), None);
                assert_eq!(store.available_lines(), None);
                assert!(store.final_facts().is_none());
                assert!(store.finish().is_err());
            } else {
                result.unwrap();
                assert_eq!(store.finish().unwrap(), facts);
            }
            drop((store, ledger));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn work_is_cumulative_and_failed_chunk_or_eof_has_no_final_facts() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut store = prepare(128, 128, limits(), &ledger).unwrap();
    store.push("abc\n").unwrap();
    let after_chunk = ledger.used().0;
    store.finish().unwrap();
    let required = ledger.used().0;
    assert_eq!(required - after_chunk, 512);
    drop((store, ledger));
    assert_eq!(memory.used_bytes(), 0);
    for input in ["abc\n", "abc\nX"] {
        let ledger = work(&memory);
        let mut limits = limits();
        limits.max_compute_work = Some(required - 1);
        let mut store = prepare(128, 128, limits, &ledger).unwrap();
        let result = store.push(input).and_then(|_| store.finish());
        assert!(matches!(
            result,
            Err(InvokeError::Kernel(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::ComputeWork,
                ..
            }))
        ));
        assert!(store.final_facts().is_none());
        assert!(ledger.used().0 <= required - 1);
        drop((store, ledger));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn a_foreign_work_scope_and_oversized_fragments_cannot_publish_partial_geometry() {
    for mode in 0..3 {
        let memory = ManagedMemoryBudget::new(1024 * 1024);
        let ledger = work(&memory);
        let foreign = work(&memory);
        let mut store = prepare(4, 4, limits(), &ledger).unwrap();
        let result = match mode {
            0 => foreign.with_scope(|| store.push("a\n")).unwrap(),
            1 => store.push("abcde"),
            _ => {
                store.push("abc\n").unwrap();
                store.push("x")
            }
        };
        assert!(result.is_err());
        assert_eq!(store.line(1), None);
        assert!(store.final_facts().is_none());
        drop((store, ledger, foreign));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn wrapped_chunk_directory_growth_preserves_ordinals_and_releases_every_claim() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut store = prepare(4096, 128, limits(), &ledger).unwrap();
    for ordinal in 1..=64 {
        store.push(&format!("line-{ordinal}\n")).unwrap();
        if ordinal < 8 {
            store.retain_from(ordinal + 1).unwrap();
        }
    }
    assert_eq!(store.line(7), None);
    for ordinal in 8..=64 {
        assert_eq!(
            store.line(ordinal),
            Some(format!("line-{ordinal}").as_str())
        );
    }
    let peak = store.peak_capacity_bytes();
    store.retain_from(65).unwrap();
    assert!(store.retained_capacity_bytes() < peak);
    assert_eq!(store.finish().unwrap().lines(), 64);
    drop((store, ledger));
    assert_eq!(memory.used_bytes(), 0);
}

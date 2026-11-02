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

fn directory_growth_before_new_tail(local: bool) {
    let tail = "x".repeat(32 << 10);
    let fragment = "\n".to_owned() + &tail;
    // This input bound makes page backing ineligible on every target.
    let memory = ManagedMemoryBudget::new(1 << 20);
    let control_work = work(&memory);
    let mut store = prepare(1 << 20, fragment.len(), limits(), &control_work).unwrap();
    for _ in 0..4 {
        store.push("\n").unwrap();
    }
    assert_eq!(store.chunks.len(), store.chunks.capacity());
    store.push(&fragment).unwrap();
    let global_limit = memory.used_bytes();
    let local_limit = store.retained_capacity_bytes();
    drop((store, control_work));
    assert_eq!(memory.used_bytes(), 0);

    // The final heap layout fits exactly. Directory replacement must release
    // its old allocation before acquiring this fragment's unfinished tail.
    let memory = ManagedMemoryBudget::new(if local { 1 << 20 } else { global_limit });
    let work = work(&memory);
    let mut limits = limits();
    if local {
        limits.max_temporary_bytes = Some(local_limit);
    }
    let mut store = prepare(1 << 20, fragment.len(), limits, &work).unwrap();
    for _ in 0..4 {
        store.push("\n").unwrap();
    }
    store.push(&fragment).unwrap();
    assert_eq!(store.available_lines(), Some(5));
    assert_eq!(store.line(6), None);
    finish_matches(&mut store, &("\n".repeat(5) + &tail));
    assert!(store.peak_capacity_bytes() <= local_limit);
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn directory_growth_precedes_tail_under_global_memory_limit() {
    directory_growth_before_new_tail(false);
}

#[test]
fn directory_growth_precedes_tail_under_local_temporary_limit() {
    directory_growth_before_new_tail(true);
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

fn every_utf8_split_matches(alphabet: &[char], maximum_length: u32) {
    for length in 0..=maximum_length {
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
fn every_short_utf8_split_matches_the_ordinary_lines_iterator() {
    every_utf8_split_matches(&['x', '\r', '\n', 'é', '猫'], 4);
}

#[test]
fn certified_fragments_preserve_geometry_admission_and_work_at_every_utf8_split() {
    for source in ["", "a\0b\r\n\nlast\r", "é\r\n猫\n🦎", "\n\r\r\nend\n"] {
        let boundaries = source
            .char_indices()
            .map(|(index, _)| index)
            .chain([source.len()])
            .collect::<Vec<_>>();
        for &first in &boundaries {
            for &second in boundaries.iter().filter(|&&index| index >= first) {
                let mut expected = None;
                for certified in [false, true] {
                    let memory = ManagedMemoryBudget::new(4 * 1024 * 1024);
                    let ledger = work(&memory);
                    let mut store = prepare(source.len() as u64, 64, limits(), &ledger).unwrap();
                    let mut observations = Vec::new();
                    for fragment in [
                        "",
                        &source[..first],
                        &source[first..second],
                        &source[second..],
                        "",
                    ] {
                        if certified {
                            store
                                .push_fragment(
                                    mech_core::Utf8Fragment::decode(fragment.as_bytes()).unwrap(),
                                )
                                .unwrap();
                        } else {
                            store.push(fragment).unwrap();
                        }
                        observations.push((
                            store.available_lines(),
                            ledger.used(),
                            store.retained_capacity_bytes(),
                            store.peak_capacity_bytes(),
                            memory.used_bytes(),
                        ));
                    }
                    finish_matches(&mut store, source);
                    observations.push((
                        store.available_lines(),
                        ledger.used(),
                        store.retained_capacity_bytes(),
                        store.peak_capacity_bytes(),
                        memory.used_bytes(),
                    ));
                    if let Some(expected) = &expected {
                        assert_eq!(&observations, expected, "{source:?}, {first}, {second}");
                    } else {
                        expected = Some(observations);
                    }
                    drop((store, ledger));
                    assert_eq!(memory.used_bytes(), 0);
                }
            }
        }
    }
}

#[test]
fn crlf_search_preserves_repeated_cr_and_every_utf8_fragment_split() {
    every_utf8_split_matches(&['a', '\r', '\n', 'é'], 5);
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
fn retention_search_work_does_not_rescan_every_live_chunk() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = work(&memory);
    let mut store = prepare(4096, 4, limits(), &work).unwrap();
    for _ in 0..512 {
        store.push("a\n").unwrap();
        let before = work.used().0;
        store.retain_from(1).unwrap();
        assert_eq!(work.used().0 - before, 4096);
    }
    let before = work.used().0;
    store.retain_from(513).unwrap();
    assert_eq!(work.used().0 - before, 4096 + 512 * 64);
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn final_contiguous_join_retires_payload_while_the_logical_owner_is_still_alive() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = work(&memory);
    let mut store = prepare(4096, 64, limits(), &work).unwrap();
    for _ in 0..32 {
        store.push("猫\nA\r\n").unwrap();
    }
    let facts = store.finish().unwrap();
    let before = store.retained_capacity_bytes();
    let expected = std::iter::repeat_n(["猫", "A"], 32)
        .flatten()
        .collect::<Vec<_>>()
        .join("|");
    let mut target = String::new();
    target.try_reserve_exact(expected.len()).unwrap();
    let output = memory.reserve_capacity(expected.len() as u64).unwrap();
    work.charge((expected.len() as u64 + facts.lines() + 1) * 128, 0)
        .unwrap();
    let prepaid = work.used();
    store
        .append_joined_consuming(1, facts.lines() + 1, "|", &mut target)
        .unwrap();
    assert_eq!(target, expected);
    assert!(store.retained_capacity_bytes() < before);
    assert!(store.chunks.is_empty());
    assert_eq!(store.final_facts(), Some(facts));
    assert_eq!(
        work.used(),
        prepaid,
        "join traversal does not renew or debit the producer allowance"
    );
    drop((store, work, output));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn prepaid_empty_separator_copy_matches_every_utf8_split_and_contiguous_selection() {
    for source in [
        "",
        "\n",
        "\r\n",
        "\r\r\n",
        "a\nb\r\nc",
        "é\n猫\r\nλ\n",
        "e\u{301}\n\n猫\r\r\nlast\r",
    ] {
        let rows = source.lines().collect::<Vec<_>>();
        for split in source.char_indices().map(|(i, _)| i).chain([source.len()]) {
            for first in 0..=rows.len() {
                for end in first..=rows.len() {
                    let memory = ManagedMemoryBudget::new(1024 * 1024);
                    let work = work(&memory);
                    let mut store = prepare(4096, 4096, limits(), &work).unwrap();
                    store.push(&source[..split]).unwrap();
                    store.push(&source[split..]).unwrap();
                    let facts = store.finish().unwrap();
                    assert_eq!(facts.lines(), rows.len() as u64);
                    let expected = "prefix ✓".to_owned() + &rows[first..end].concat();
                    let mut target = "prefix ✓".to_owned();
                    target
                        .try_reserve_exact(expected.len() - target.len())
                        .unwrap();
                    let output = memory.reserve_capacity(target.capacity() as u64).unwrap();
                    let prepaid = work.used();
                    let capacity = target.capacity();
                    store
                        .append_joined_consuming(first as u64 + 1, end as u64 + 1, "", &mut target)
                        .unwrap();
                    assert_eq!(
                        target, expected,
                        "{source:?} split {split}, range {first}..{end}"
                    );
                    assert_eq!(
                        target.capacity(),
                        capacity,
                        "prepaid copying cannot grow the target"
                    );
                    assert_eq!(work.used(), prepaid, "copying cannot renew source work");
                    assert!(store.chunks.is_empty());
                    drop((store, work, output));
                    assert_eq!(memory.used_bytes(), 0);
                }
            }
        }
    }
}

#[test]
fn empty_separator_copy_rejects_short_capacity_without_publishing_bytes() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = work(&memory);
    let mut store = prepare(4096, 4096, limits(), &work).unwrap();
    store.push("é\r\n猫\nlast\n").unwrap();
    store.finish().unwrap();
    let chunk = store.chunks.front().unwrap();
    let mut target = String::new();
    let prepaid = work.used();
    assert!(matches!(
        chunk.append_payload(chunk.first, chunk.end(), &mut target),
        Err(InvokeError::InvalidInput)
    ));
    assert!(target.is_empty());
    assert_eq!(work.used(), prepaid);
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn consuming_join_chunks_initialize_and_retire_before_a_following_native_transform() {
    for source in [
        "",
        "\n",
        "a\nb\r\n\nc",
        "ab\ncd\r\nef\nlast\r",
        "é\n猫\r\nλ\n",
    ] {
        for split in source.char_indices().map(|(i, _)| i).chain([source.len()]) {
            let memory = ManagedMemoryBudget::new(1024 * 1024);
            let work = work(&memory);
            let mut store = prepare(4096, 4096, limits(), &work).unwrap();
            store.push(&source[..split]).unwrap();
            store.push(&source[split..]).unwrap();
            let facts = store.finish().unwrap();
            let expected = source.lines().collect::<String>().to_ascii_uppercase();
            let mut output = Vec::new();
            output.try_reserve_exact(expected.len()).unwrap();
            let reservation = memory.reserve_capacity(output.capacity() as u64).unwrap();
            let capacity = output.capacity();
            let prepaid = work.used();
            let mut next = 1;
            loop {
                let start = output.len();
                let following = store
                    .append_joined_chunk(next, facts.lines() + 1, &mut output)
                    .unwrap();
                let Some(following) = following else {
                    break;
                };
                assert!(following > next);
                assert!(
                    store.line(next).is_none(),
                    "copied backing retires before its output is transformed"
                );
                // Tests transform only the newly initialized private byte span.
                output[start..].make_ascii_uppercase();
                next = following;
            }
            assert_eq!(output, expected.as_bytes());
            assert_eq!(output.capacity(), capacity);
            assert_eq!(work.used(), prepaid);
            assert!(store.chunks.is_empty());
            drop((store, work, reservation));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
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
    for (local, certified) in [(false, false), (false, true), (true, false), (true, true)] {
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
            let result = if certified {
                store.push_fragment(mech_core::Utf8Fragment::decode(source.as_bytes()).unwrap())
            } else {
                store.push(source)
            };
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

#[test]
fn cached_cells_preserve_random_repeated_backward_and_retired_access() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let ledger = work(&memory);
    let mut store = prepare(128, 32, limits(), &ledger).unwrap();
    for fragment in ["a\nb\r\n", "猫\n\n", "tail"] {
        store.push(fragment).unwrap();
    }
    store.finish().unwrap();
    let before = ledger.used();
    let mut cursor = usize::MAX;
    for (ordinal, chunk) in [(1, 0), (2, 0), (3, 1), (4, 1), (5, 2)] {
        assert_eq!(store.line_cached(ordinal, &mut cursor), store.line(ordinal));
        assert_eq!(cursor, chunk, "sequential reads track their actual chunk");
    }
    for initial in [0, 1, 2, usize::MAX] {
        cursor = initial;
        for ordinal in [5, 1, 1, 4, 3, 2, 5, 0, 6, u64::MAX, 3] {
            assert_eq!(store.line_cached(ordinal, &mut cursor), store.line(ordinal));
        }
    }
    assert_eq!(
        ledger.used(),
        before,
        "lookup uses the enclosing traversal debit"
    );
    store.retain_from(3).unwrap();
    for ordinal in [1, 5, 3, 4, 2, 3] {
        assert_eq!(store.line_cached(ordinal, &mut cursor), store.line(ordinal));
    }
    store.retain_range(3, 5).unwrap();
    for ordinal in [5, 3, 4, 1, 4, 3] {
        assert_eq!(store.line_cached(ordinal, &mut cursor), store.line(ordinal));
    }
    drop((store, ledger));
    assert_eq!(memory.used_bytes(), 0);
}

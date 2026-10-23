//! Exercise mapped storage through the maintained capture and consuming join.
use super::*;

fn limits() -> MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_compute_work = Some(1 << 46);
    limits.max_temporary_bytes = Some(32 << 20);
    limits
}

#[test]
fn mapped_capture_preserves_utf8_lines_and_retires_pages_during_the_real_join() {
    let memory = ManagedMemoryBudget::new(32 << 20);
    let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
    let fragment = "é猫\r\nACGT\n".repeat(4096);
    let source = fragment.repeat(64);
    let mut store = prepare(
        source.len() as u64,
        fragment.len(),
        limits(),
        &work,
        0,
        None,
    )
    .unwrap();
    for _ in 0..64 {
        store.push(&fragment).unwrap();
        assert!(matches!(
            store.chunks.back().unwrap().bytes,
            ChunkData::Pages(_)
        ));
    }
    let facts = store.finish().unwrap();
    assert_eq!(facts.lines(), 64 * 8192);
    assert_eq!(store.line(1), Some("é猫"));
    assert_eq!(store.line(facts.lines()), Some("ACGT"));
    let before = store.retained_capacity_bytes();
    assert!(before > 3 << 20);
    let expected: String = source.lines().collect();
    let output = memory.reserve_capacity(expected.len() as u64).unwrap();
    let mut target = String::new();
    target.try_reserve_exact(expected.len()).unwrap();
    work.charge((expected.len() as u64 + facts.lines() + 1) * 128, 0)
        .unwrap();
    let prepaid = work.used();
    store
        .append_joined_consuming(1, facts.lines() + 1, "", &mut target)
        .unwrap();
    assert_eq!(target, expected);
    assert!(store.retained_capacity_bytes() < 64 << 10);
    assert_eq!(work.used(), prepaid);
    assert_eq!(store.final_facts(), Some(facts));
    drop((target, output, store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_reuses_discarded_prefix_backing_and_stays_inside_the_work_bound() {
    let memory = ManagedMemoryBudget::new(8 << 20);
    let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
    let fragment = "ACGT\n".repeat(8192);
    let input_limit = (fragment.len() * 128) as u64;
    let bound = capture_work_bound(input_limit, 0, 0, 0).unwrap();
    let initial_work = work.used().0;
    let mut store = prepare(input_limit, fragment.len(), limits(), &work, 0, None).unwrap();
    let mut held = 0;
    for round in 0..128 {
        store.push(&fragment).unwrap();
        assert!(matches!(
            store.chunks.back().unwrap().bytes,
            ChunkData::Pages(_)
        ));
        store
            .retain_from(store.available_lines().unwrap() + 1)
            .unwrap();
        if round == 0 {
            held = store.retained_capacity_bytes();
        } else {
            assert_eq!(store.retained_capacity_bytes(), held);
        }
    }
    store.finish().unwrap();
    assert!(work.used().0 - initial_work <= bound);
    assert!(memory.used_bytes() < 3 << 20);
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_budget_includes_dense_offsets_before_optional_pages() {
    for local in [false, true] {
        let capacity = (2 << 20) + (128 << 10);
        let memory = ManagedMemoryBudget::new(if local { 8 << 20 } else { capacity });
        let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
        let mut limits = limits();
        if local {
            limits.max_temporary_bytes = Some(capacity);
        }
        let source = "\n".repeat(64 << 10);
        let mut store = prepare(8 << 20, source.len(), limits, &work, 0, None).unwrap();
        store.push(&source).unwrap();
        assert!(matches!(
            store.chunks.back().unwrap().bytes,
            ChunkData::Heap(_)
        ));
        assert_eq!(store.finish().unwrap().lines(), 64 << 10);
        assert_eq!(store.line(1), Some(""));
        assert_eq!(store.line(64 << 10), Some(""));
        assert!(store.peak_capacity_bytes() < 512 << 10);
        assert!(
            work.used().0 < 8 << 20,
            "a declined slab spends no speculative work"
        );
        drop((store, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn mapped_capture_budget_includes_directory_growth_before_optional_pages() {
    let source = "x".repeat((16 << 10) - 1) + "\n";
    let memory = ManagedMemoryBudget::new(8 << 20);
    let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
    let mut store = prepare(8 << 20, source.len(), limits(), &work, 0, None).unwrap();
    let fixed_memory = memory.used_bytes();
    let fixed_account = store.account.used();
    store.push(&source).unwrap();
    assert!(matches!(
        store.chunks.back().unwrap().bytes,
        ChunkData::Pages(_)
    ));
    let offsets = 4;
    let directory = (store.chunks.capacity() * core::mem::size_of::<Chunk>()) as u64 + 128;
    let slab = store.account.used() - fixed_account - offsets - directory;
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);

    // The slab and offsets fit exactly, but its mandatory directory does not.
    // Ordinary heap storage fits comfortably under the same global budget.
    let memory = ManagedMemoryBudget::new(fixed_memory + slab + offsets);
    let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
    let mut store = prepare(8 << 20, source.len(), limits(), &work, 0, None).unwrap();
    store.push(&source).unwrap();
    assert!(matches!(
        store.chunks.back().unwrap().bytes,
        ChunkData::Heap(_)
    ));
    assert_eq!(store.finish().unwrap().lines(), 1);
    assert_eq!(store.line(1), Some(&source[..source.len() - 1]));
    assert!(store.peak_capacity_bytes() < 64 << 10);
    drop((store, work));
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn mapped_capture_budget_includes_unterminated_tail_before_optional_pages() {
    for local in [false, true] {
        let capacity = (2 << 20) + (16 << 10);
        let memory = ManagedMemoryBudget::new(if local { 8 << 20 } else { capacity });
        let work = NativeLibraryWorkBudget::new(1 << 48, 1 << 32, &memory).unwrap();
        let mut limits = limits();
        if local {
            limits.max_temporary_bytes = Some(capacity);
        }
        let first = "a".repeat((16 << 10) - 1);
        let tail = "b".repeat(32 << 10);
        let source = first.clone() + "\n" + &tail;
        let mut store = prepare(8 << 20, source.len(), limits, &work, 0, None).unwrap();
        store.push(&source).unwrap();
        assert!(matches!(
            store.chunks.front().unwrap().bytes,
            ChunkData::Heap(_)
        ));
        assert_eq!(store.available_lines(), Some(1));
        assert_eq!(store.line(2), None, "unfinished tail stays private");
        assert_eq!(store.finish().unwrap().lines(), 2);
        assert_eq!(store.line(1), Some(first.as_str()));
        assert_eq!(store.line(2), Some(tail.as_str()));
        assert!(store.peak_capacity_bytes() < 128 << 10);
        drop((store, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

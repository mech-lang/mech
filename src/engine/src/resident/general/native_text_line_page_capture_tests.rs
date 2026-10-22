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

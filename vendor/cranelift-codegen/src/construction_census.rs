//! Optional host-compiler work counters. Disabled in ordinary builds.
//!
//! Counts DFG arena positions, including copied functions and compacted value
//! tables. Resident positions include retired IDs until compaction; they are
//! not semantic or register liveness, allocator capacities, or native bytes.
//! Read only after compilation workers have joined. No latency measurement
//! should use a census-enabled compiler.
use core::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// One kind of intermediate arena position.
#[derive(Clone, Copy)]
pub(crate) enum Arena {
    /// Basic blocks, including blocks not currently in the layout.
    Blocks = 0,
    /// Instructions, including removed instructions still in the arena.
    Instructions = 1,
    /// SSA entries, including aliases and retired positions.
    Values = 2,
}
struct Counter {
    created: AtomicU64,
    resident: AtomicU64,
    peak_resident: AtomicU64,
    maximum_unit: AtomicU64,
}
impl Counter {
    const fn new() -> Self {
        Self {
            created: AtomicU64::new(0),
            resident: AtomicU64::new(0),
            peak_resident: AtomicU64::new(0),
            maximum_unit: AtomicU64::new(0),
        }
    }
}
static ARENAS: [Counter; 3] = [const { Counter::new() }; 3];
static CFG_BUILDS: AtomicU64 = AtomicU64::new(0);
static DOMINATOR_BUILDS: AtomicU64 = AtomicU64::new(0);
static FULL_VERIFICATIONS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn allocate(arena: Arena, count: usize, unit_size: usize) {
    let counter = &ARENAS[arena as usize];
    counter.created.fetch_add(count as u64, Relaxed);
    let now = counter.resident.fetch_add(count as u64, Relaxed) + count as u64;
    counter.peak_resident.fetch_max(now, Relaxed);
    counter.maximum_unit.fetch_max(unit_size as u64, Relaxed);
}
pub(crate) fn release(arena: Arena, count: usize) {
    if count == 0 {
        return;
    }
    ARENAS[arena as usize]
        .resident
        .fetch_update(Relaxed, Relaxed, |n| n.checked_sub(count as u64))
        .expect("construction census released unrecorded arena entries");
}
pub(crate) fn cfg_build() {
    CFG_BUILDS.fetch_add(1, Relaxed);
}
pub(crate) fn dominator_build() {
    DOMINATOR_BUILDS.fetch_add(1, Relaxed);
}
pub(crate) fn full_verification() {
    FULL_VERIFICATIONS.fetch_add(1, Relaxed);
}

/// Arena work across all functions and temporary function copies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArenaSnapshot {
    /// Total positions created, including clone and compaction copies.
    pub created: u64,
    /// Currently retained arena positions, including retired aliases.
    pub resident: u64,
    /// Maximum simultaneously retained positions across all functions.
    pub peak_resident: u64,
    /// Maximum arena length in any one function (not layout length).
    pub maximum_unit: u64,
}
/// Diagnostic construction work. No generated-code contract is implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    /// Basic-block arena work.
    pub blocks: ArenaSnapshot,
    /// Instruction arena work.
    pub instructions: ArenaSnapshot,
    /// SSA value arena work.
    pub values: ArenaSnapshot,
    /// Complete CFG constructions; individual edge updates are not builds.
    pub cfg_builds: u64,
    /// Complete dominator constructions.
    pub dominator_builds: u64,
    /// Full verifier runs; early failure still counts as one run.
    pub full_verifications: u64,
}
/// Read counters after compilation workers have joined.
pub fn snapshot() -> Snapshot {
    let arena = |i: usize| {
        let c = &ARENAS[i];
        ArenaSnapshot {
            created: c.created.load(Relaxed),
            resident: c.resident.load(Relaxed),
            peak_resident: c.peak_resident.load(Relaxed),
            maximum_unit: c.maximum_unit.load(Relaxed),
        }
    };
    Snapshot {
        blocks: arena(0),
        instructions: arena(1),
        values: arena(2),
        cfg_builds: CFG_BUILDS.load(Relaxed),
        dominator_builds: DOMINATOR_BUILDS.load(Relaxed),
        full_verifications: FULL_VERIFICATIONS.load(Relaxed),
    }
}
/// Begin a diagnostic interval with no existing DFG entries or active workers.
/// Panics rather than silently discarding live storage from the accounting.
pub fn reset() {
    for counter in &ARENAS {
        assert_eq!(
            counter.resident.load(Relaxed),
            0,
            "census reset with live DFG entries"
        );
    }
    for counter in &ARENAS {
        counter.created.store(0, Relaxed);
        counter.peak_resident.store(0, Relaxed);
        counter.maximum_unit.store(0, Relaxed);
    }
    CFG_BUILDS.store(0, Relaxed);
    DOMINATOR_BUILDS.store(0, Relaxed);
    FULL_VERIFICATIONS.store(0, Relaxed);
}

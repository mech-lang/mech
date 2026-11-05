//! Explicit native storage layout for an admitted immutable String→U64 lookup
//! region. This module grants no source visit or selected-call permission.
//! Setup owns/pins every pointer; generated WORK borrows it synchronously.

/// Shares the maintained Engine String-matrix cell ABI. A Collection producer
/// constructs its own prices from its own proof, never another factory's fee.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::native_artifact) struct Cell {
    pub address: u64,
    pub bytes: u64,
    pub logical_output_bytes: u64,
    pub logical_cloned_bytes: u64,
    pub selection_compute_work: u64,
    pub selection_comparison_work: u64,
    pub logical_temporary_bytes: u64,
    pub logical_extra_nodes: u64,
}

/// Native table representation, independent of hashbrown's private ABI.
/// Occupied is stored last by any later mutable-owner extension.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::native_artifact) struct Slot {
    pub address: u64,
    pub bytes: u64,
    pub value: u64,
    pub hash: u64,
    pub occupied: u64,
}

/// Finite native physical preflight allowance. Logical producer/Map/builder
/// fees are committed only after the complete lookup succeeds. Failed native
/// preflight discards private output and retains the original body CFG.
#[repr(C)]
pub(in crate::native_artifact) struct Frame {
    pub slots: u64,
    pub capacity: u64,
    pub cells: u64,
    pub count: u64,
    pub rows: u64,
    pub columns: u64,
    pub output: u64,
    pub stored_bytes: u64,
    pub fuel: u64,
    pub comparison: u64,
    pub epoch: u64,
    pub issued: u64,
    pub next: u64,
}

pub(in crate::native_artifact) const HASH_OFFSET: u64 = 0xcbf29ce484222325;
pub(in crate::native_artifact) const HASH_PRIME: u64 = 0x100000001b3;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) const COMPLETE: u64 = 0;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) const COLD: u64 = 1;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) const MISSING: u64 = 2;

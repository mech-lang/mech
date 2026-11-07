//! Explicit native storage layout for an admitted immutable String→U64 lookup
//! region. This module grants no source visit or selected-call permission.
//! Setup owns/pins every pointer; generated WORK borrows it synchronously.

/// A borrowed UTF8 slice for a complete CollectionElement batch. Its logical
/// producer fees are admitted once by the exact whole-region proof, so native
/// lookup needs only the pinned address and extent. This private layout is
/// distinct from the Engine's per-selector priced String-matrix loan.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::native_artifact) struct Cell {
    pub address: u64,
    pub bytes: u64,
}

// Keep the existing conservative work/resource envelope while reducing actual
// backing. This change never obtains a speedup by discounting admission.
pub(in crate::native_artifact) const ADMITTED_CELL_BYTES: u64 = 8 * size_of::<u64>() as u64;
const _: () = assert!(size_of::<Cell>() == 2 * size_of::<u64>());

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

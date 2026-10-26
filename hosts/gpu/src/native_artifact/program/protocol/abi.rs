//! ABI35 permits a proved uniform constructor at its own collection header.
//! It reuses the existing collection helper lanes and frame layout.
//! ABI34 adds a checked lexical-integrity failure status, sealed against
//! the exact admitted visit. All ABI33 mapping representations remain.
//! ABI33 shares an owner-prepared 16-byte header and 128-byte table allowance.
//! An exhaustively proved 32-cell projection selects a separate leaf once;
//! arbitrary maps keep the full 128-cell representation. Signatures remain.
//! ABI32 stores the joined mapper's admitted table as 128 bytes: 0..127 are
//! valid outputs (including zero), and the high bit denotes a missing entry.
//! Both its scalar and vector paths read this same prepared representation.
//! ABI31 binds owned selected-source geometry and exact initialized extents
//! for direct generated transform/copy. Frame banks and signatures remain.
//! ABI30 adds paired selected-join chunk loans and completed String-map entry.
//! ABI29 discriminates borrowed compact line offsets.
//! ABI28 binds the optional owned complete-key lookup cache
//! pointer in the packed map frame. The ordinary table and helper lanes remain.
//! Entry phases, control pointer and scalar transport stay unchanged.
use crate::native_artifact::abi::NativeEntryFrame;
use crate::native_artifact::program::protocol::GlobalHelper;
use core::ffi::c_void;
pub(in crate::native_artifact) const VERSION: u64 = 35;
pub(in crate::native_artifact) const JOIN_MAP_MISSING_BYTE: u8 = 0x80;
pub(in crate::native_artifact) const PHASE_ACTIVATION: u64 = 0;
pub(in crate::native_artifact) const PHASE_TURN: u64 = 1;
pub(in crate::native_artifact) const PHASE_PREFIX: u64 = 2;
pub(in crate::native_artifact) const PHASE_CAPTURE_TAIL: u64 = 3;
pub(in crate::native_artifact) const PHASE_WORKER: u64 = 4;
pub(in crate::native_artifact) const STATUS_WORKER_RESULT: u64 = 8;
pub(in crate::native_artifact) const STATUS_ACTIVATED: u64 = 5;
pub(in crate::native_artifact) const STATUS_INPUT_PENDING: u64 = 6;
pub(in crate::native_artifact) const STATUS_INPUT_SCANNED: u64 = 7;
#[cfg(feature = "canonical-native-program")]
pub(in crate::native_artifact) const ENTRY: &str = "mech_canonical_program_entry_v1";
pub(in crate::native_artifact) const LIBRARY: u64 = GlobalHelper::Library.raw();
pub(in crate::native_artifact) const MOVE: u64 = GlobalHelper::Move.raw();
#[cfg(test)]
pub(in crate::native_artifact) const COLLECTION: u64 = GlobalHelper::Collection.raw();
pub(in crate::native_artifact) const OUTPUT_BASE: u64 = 3;
// Private compiled-region loans within the SAME turn, not new transactions.
#[cfg(any(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact) const BORROWED_STRINGS_PREPARE: u64 =
    GlobalHelper::BorrowedStringsPrepare.raw();
#[cfg(any(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact) const CLOSED_TAIL_BEGIN: u64 = GlobalHelper::ClosedTailBegin.raw();
#[cfg(test)]
pub(in crate::native_artifact) const CLOSED_TAIL_END: u64 = GlobalHelper::ClosedTailEnd.raw();
pub(in crate::native_artifact) const PREFIX_PAUSE: u64 = GlobalHelper::PrefixPause.raw();
pub(in crate::native_artifact) const PREFIX_BORROWED_RESUME: u64 =
    GlobalHelper::PrefixBorrowedResume.raw();
pub(in crate::native_artifact) const PREFIX_TAIL_RESUME: u64 = GlobalHelper::PrefixTailResume.raw();
pub(in crate::native_artifact) const PREFIX_SCANNED: u64 = GlobalHelper::PrefixScanned.raw();
// Source geometry occupies six immutable words; only this final lane is writable.
pub(in crate::native_artifact) const JOIN_MAP_INITIALIZED_WORD: usize = 6;
pub(in crate::native_artifact) const MAP_OWNER_BEGIN: u64 = GlobalHelper::MapOwnerBegin.raw();
pub(in crate::native_artifact) const MAP_OWNER_CALL: u64 = GlobalHelper::MapOwnerCall.raw();
pub(in crate::native_artifact) const MAP_OWNER_END: u64 = GlobalHelper::MapOwnerEnd.raw();
pub(in crate::native_artifact) const MAP_OWNER_SPAN: u64 = GlobalHelper::MapOwnerSpan.raw();
pub(in crate::native_artifact) const PARALLEL_RUN: u64 = GlobalHelper::ParallelRun.raw();
pub(in crate::native_artifact) const PARALLEL_TAKE: u64 = GlobalHelper::ParallelTake.raw();
pub(in crate::native_artifact) const PARALLEL_YIELD: u64 = GlobalHelper::ParallelYield.raw();
pub(in crate::native_artifact) const PARALLEL_ENTER: u64 = GlobalHelper::ParallelEnter.raw();
// Global Invocation helpers; distinct from private bound Owner operations.
pub(in crate::native_artifact) const FROZEN_MAP_COPY: u64 = GlobalHelper::FrozenMapCopy.raw();
pub(in crate::native_artifact) const FROZEN_MAP_READ: u64 = GlobalHelper::FrozenMapRead.raw();
pub(in crate::native_artifact) type Helper =
    unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64, u64, *mut u64) -> u64;
#[repr(C)]
pub(in crate::native_artifact) struct Frame {
    pub base: NativeEntryFrame,
    pub helper: Helper,
    pub result: *mut u64,
    pub arguments: *mut u64,
    pub argument_capacity: u64,
    // Immutable for each call; only the owning session selects the phase.
    pub phase: u64,
    // Constructor-private continuation facts, immutable throughout an entry.
    // These are NOT canonical line/count Values or arbitrary jump targets.
    // PHASE_WORKER reuses these lanes for proved begin site, original element
    // index and original source count; PARALLEL_ENTER checks all three against
    // the private child constructor before the first source-body visit.
    pub resume_site: u64,
    pub available_lines: u64,
    pub input_eof: u64,
}
pub(in crate::native_artifact) type Entry = unsafe extern "C" fn(*mut Frame) -> u64;

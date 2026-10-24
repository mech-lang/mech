//! Mixed-value ABI28 binds the optional owned complete-key lookup cache
//! pointer in the packed map frame. The ordinary table and helper lanes remain.
//! Entry phases, control pointer and scalar transport stay unchanged.
use super::abi::NativeEntryFrame;
use core::ffi::c_void;
pub(super) const VERSION: u64 = 28;
pub(super) const PHASE_ACTIVATION: u64 = 0;
pub(super) const PHASE_TURN: u64 = 1;
pub(super) const PHASE_PREFIX: u64 = 2;
pub(super) const PHASE_CAPTURE_TAIL: u64 = 3;
pub(super) const PHASE_WORKER: u64 = 4;
pub(super) const STATUS_WORKER_RESULT: u64 = 8;
pub(super) const STATUS_ACTIVATED: u64 = 5;
pub(super) const STATUS_INPUT_PENDING: u64 = 6;
pub(super) const STATUS_INPUT_SCANNED: u64 = 7;
#[cfg(feature = "canonical-native-program")]
pub(super) const ENTRY: &str = "mech_canonical_program_entry_v1";
pub(super) const LIBRARY: u64 = 0;
pub(super) const MOVE: u64 = 1;
pub(super) const COLLECTION: u64 = 2;
pub(super) const OUTPUT_BASE: u64 = 3;
// Private compiled-region loans within the SAME turn, not new transactions.
pub(super) const WINDOW_PREPARE: u64 = u64::MAX - 1;
pub(super) const WINDOW_FINISH: u64 = u64::MAX - 2;
pub(super) const STRING_MAP_PREPARE: u64 = u64::MAX - 3;
pub(super) const STRING_MAP_FINISH: u64 = u64::MAX - 4;
pub(super) const BORROWED_STRINGS_PREPARE: u64 = u64::MAX - 5;
pub(super) const BORROWED_STRING_FAILURE: u64 = u64::MAX - 6;
pub(super) const CLOSED_TAIL_BEGIN: u64 = u64::MAX - 7;
pub(super) const CLOSED_TAIL_END: u64 = u64::MAX - 8;
pub(super) const PREFIX_PAUSE: u64 = u64::MAX - 9;
pub(super) const PREFIX_BORROWED_RESUME: u64 = u64::MAX - 10;
pub(super) const PREFIX_TAIL_RESUME: u64 = u64::MAX - 11;
pub(super) const PREFIX_SCANNED: u64 = u64::MAX - 12;
pub(super) const MAP_OWNER_BEGIN: u64 = u64::MAX - 16;
pub(super) const MAP_OWNER_CALL: u64 = u64::MAX - 17;
pub(super) const MAP_OWNER_END: u64 = u64::MAX - 18;
pub(super) const MAP_OWNER_SPAN: u64 = u64::MAX - 19;
pub(super) const PARALLEL_RUN: u64 = u64::MAX - 20;
pub(super) const PARALLEL_TAKE: u64 = u64::MAX - 21;
pub(super) const PARALLEL_YIELD: u64 = u64::MAX - 22;
pub(super) const PARALLEL_ENTER: u64 = u64::MAX - 23;
// Global Context helpers; distinct from private bound Owner operations.
pub(super) const FROZEN_MAP_COPY: u64 = u64::MAX - 24;
pub(super) const FROZEN_MAP_READ: u64 = u64::MAX - 25;
pub(super) type Helper =
    unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64, u64, *mut u64) -> u64;
#[repr(C)]
pub(super) struct Frame {
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
pub(super) type Entry = unsafe extern "C" fn(*mut Frame) -> u64;

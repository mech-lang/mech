//! Mixed-value ABI8 carries an admitted variable-arity operand lane without
//! changing the frozen scalar-frame prefix. Older Objects fail admission.
use super::abi::NativeEntryFrame;
use core::ffi::c_void;
pub(super) const VERSION: u64 = 8;
pub(super) const PHASE_ACTIVATION: u64 = 0;
pub(super) const PHASE_TURN: u64 = 1;
pub(super) const STATUS_ACTIVATED: u64 = 5;
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
pub(super) type Helper =
    unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64, u64, *mut u64) -> u64;
#[repr(C)]
pub(super) struct Frame {
    pub base: NativeEntryFrame,
    pub helper: Helper,
    pub result: *mut u64,
    pub arguments: *mut u64,
    pub argument_capacity: u64,
    // Immutable for each call; only the owning session selects the second phase.
    pub phase: u64,
}
#[cfg(feature = "canonical-native-program")]
pub(super) type Entry = unsafe extern "C" fn(*mut Frame) -> u64;

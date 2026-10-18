//! Explicit mixed-value ABI4, preserving the frozen scalar-frame prefix.
use super::abi::NativeEntryFrame;
use core::ffi::c_void;
pub(super) const VERSION: u64 = 4;
#[cfg(feature = "canonical-native-program")]
pub(super) const ENTRY: &str = "mech_canonical_program_entry_v1";
pub(super) const LIBRARY: u64 = 0;
pub(super) const MOVE: u64 = 1;
pub(super) const OUTPUT_BASE: u64 = 2;
pub(super) type Helper = unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64, *mut u64) -> u64;
#[repr(C)]
pub(super) struct Frame {
    pub base: NativeEntryFrame,
    pub helper: Helper,
    pub result: *mut u64,
}
#[cfg(feature = "canonical-native-program")]
pub(super) type Entry = unsafe extern "C" fn(*mut Frame) -> u64;

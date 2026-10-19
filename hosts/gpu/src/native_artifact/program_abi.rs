//! Mixed-value ABI6 carries three checked leaf operands without changing the
//! frozen scalar-frame prefix. ABI5 Objects fail identity/version admission.
use super::abi::NativeEntryFrame;
use core::ffi::c_void;
pub(super) const VERSION: u64 = 6;
#[cfg(feature = "canonical-native-program")]
pub(super) const ENTRY: &str = "mech_canonical_program_entry_v1";
pub(super) const LIBRARY: u64 = 0;
pub(super) const MOVE: u64 = 1;
pub(super) const COLLECTION: u64 = 2;
pub(super) const OUTPUT_BASE: u64 = 3;
pub(super) type Helper =
    unsafe extern "C" fn(*mut c_void, u64, u64, u64, u64, u64, *mut u64) -> u64;
#[repr(C)]
pub(super) struct Frame {
    pub base: NativeEntryFrame,
    pub helper: Helper,
    pub result: *mut u64,
}
#[cfg(feature = "canonical-native-program")]
pub(super) type Entry = unsafe extern "C" fn(*mut Frame) -> u64;

//! Private borrowed UTF8 histogram storage ABI. No execution or publication authority.
pub(in crate::native_artifact) const COMPLETE: u64 = 0;
pub(in crate::native_artifact) const COLD: u64 = 1;
pub(in crate::native_artifact) const FULL: u64 = 2;
pub(in crate::native_artifact) const INVALID: u64 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct Slot {
    pub hash: u64,
    pub count: u64,
    // Zero is empty. Occupied markers are written last, after all checks.
    pub row_plus_one: u64,
}

#[repr(C)]
#[derive(Debug)]
pub(in crate::native_artifact) struct Frame {
    pub cells: u64,
    pub count: u64,
    pub slots: u64,
    pub capacity: u64,
    pub fuel: u64,
    pub comparison: u64,
    pub completed: u64,
    pub distinct: u64,
    pub payload: u64,
}

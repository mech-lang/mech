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
    pub limit: u64,
    pub fuel: u64,
    pub comparison: u64,
    pub completed: u64,
    pub distinct: u64,
    pub payload: u64,
}

/// Complete checked pass allowance, charged before a prepaid native entry.
/// Hash reads visit at most the source owner's admitted payload bound. Every
/// row makes at most `capacity` probes and compares at most its own byte length
/// per probe. Larger literal factors also cover word mixing and fixed metadata.
pub(in crate::native_artifact) fn pass_work(
    count: u64,
    source_bytes: u64,
    capacity: u64,
) -> Option<(u64, u64)> {
    let comparison = capacity.checked_mul(source_bytes)?;
    let fuel = count
        .checked_mul(64)?
        .checked_add(source_bytes.checked_mul(4)?)?
        .checked_add(capacity.checked_mul(count.checked_mul(16)?)?)?
        .checked_add(comparison.checked_mul(2)?)?;
    Some((fuel, comparison))
}

pub(in crate::native_artifact) fn bank_limit(capacity: usize) -> u64 {
    (capacity - capacity / 4) as u64
}

pub(in crate::native_artifact) fn maximum_capacity(count: usize) -> Option<usize> {
    count
        .checked_add(count / 3)?
        .checked_add(1)?
        .max(4)
        .checked_next_power_of_two()
}

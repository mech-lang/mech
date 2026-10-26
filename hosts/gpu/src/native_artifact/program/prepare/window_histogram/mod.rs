//! Private physical variants of the same base-128 histogram result.
pub(in crate::native_artifact) const SINGLE_CELLS: usize = 128;
pub(in crate::native_artifact) const BANKS: usize = 4;
pub(in crate::native_artifact) const BANKED_CELLS: usize = SINGLE_CELLS * BANKS;
pub(in crate::native_artifact) const PAIR_CELLS: usize = SINGLE_CELLS * SINGLE_CELLS;
pub(in crate::native_artifact) const BANKED_PAIR_CELLS: usize = PAIR_CELLS * BANKS;

pub(in crate::native_artifact) fn banked_cells(width: u64) -> Option<usize> {
    match width {
        1 => Some(BANKED_CELLS),
        2 => Some(BANKED_PAIR_CELLS),
        _ => None,
    }
}

pub(in crate::native_artifact) fn banked_peak(width: u64, entry_bytes: u64) -> Option<u64> {
    let physical = banked_cells(width)? as u64;
    let logical = physical / BANKS as u64;
    entry_bytes
        .checked_add(width)
        .and_then(|entry| entry.checked_mul(logical))
        .and_then(|entries| entries.checked_add(physical * size_of::<u64>() as u64))
}

/// This is a preference, never an allocation permit. The caller must reserve
/// the complete banked peak atomically before allocation, falling back to the
/// serial layout if another account user consumes the observed headroom.
/// Include the worst possible Entry/key backing while scratch is live.
/// Overflow simply declines the variant.
pub(in crate::native_artifact) fn prefer_banks(
    width: u64,
    accepted: u64,
    temporary_base: u64,
    temporary_limit: Option<u64>,
    memory_remaining: u64,
    entry_bytes: u64,
) -> bool {
    let Some(physical) = banked_cells(width) else {
        return false;
    };
    // Amortize zeroing/reduction over at least eight starts per logical cell.
    if accepted < (physical / BANKS * 8) as u64 {
        return false;
    }
    let Some(backing) = banked_peak(width, entry_bytes) else {
        return false;
    };
    backing <= memory_remaining
        && temporary_base
            .checked_add(backing)
            .is_some_and(|peak| temporary_limit.is_none_or(|limit| peak <= limit))
}

#[cfg(test)]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::prepare::window_histogram) mod tests;

//! Private physical variants of the same base-128 histogram result.
pub(super) const SINGLE_CELLS: usize = 128;
pub(super) const BANKS: usize = 4;
pub(super) const BANKED_CELLS: usize = SINGLE_CELLS * BANKS;
pub(super) const PAIR_CELLS: usize = SINGLE_CELLS * SINGLE_CELLS;
pub(super) const BANKED_PAIR_CELLS: usize = PAIR_CELLS * BANKS;

pub(super) fn banked_cells(width: u64) -> Option<usize> {
    match width {
        1 => Some(BANKED_CELLS),
        2 => Some(BANKED_PAIR_CELLS),
        _ => None,
    }
}

pub(super) fn banked_peak(width: u64, entry_bytes: u64) -> Option<u64> {
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
pub(super) fn prefer_banks(
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
mod tests {
    use super::*;

    #[test]
    fn banks_require_both_complete_peaks_and_decline_overflow() {
        let entry = 32;
        let backing = 4 * 128 * 8 + 128 * (entry + 1);
        assert!(prefer_banks(
            1,
            1024,
            17,
            Some(17 + backing),
            backing,
            entry
        ));
        assert!(!prefer_banks(
            1,
            1024,
            17,
            Some(16 + backing),
            backing,
            entry
        ));
        assert!(!prefer_banks(1, 1024, 17, None, backing - 1, entry));
        assert!(!prefer_banks(1, 1023, 17, None, backing, entry));
        for width in [0, 2, 3, u64::MAX] {
            assert!(!prefer_banks(width, 1024, 0, None, u64::MAX, entry));
        }
        assert!(!prefer_banks(1, 1024, u64::MAX, None, u64::MAX, entry));
        assert!(!prefer_banks(1, 1024, 0, None, u64::MAX, u64::MAX));
    }

    #[test]
    fn pair_banks_fund_every_counter_and_two_byte_key_before_selection() {
        let entry = 32;
        let starts = (PAIR_CELLS * 8) as u64;
        let peak = (BANKED_PAIR_CELLS * 8) as u64 + PAIR_CELLS as u64 * (entry + 2);
        assert_eq!(banked_peak(2, entry), Some(peak));
        assert!(prefer_banks(2, starts, 17, Some(peak + 17), peak, entry));
        assert!(!prefer_banks(2, starts - 1, 17, None, peak, entry));
        assert!(!prefer_banks(2, starts, 17, Some(peak + 16), peak, entry));
        assert!(!prefer_banks(2, starts, 17, None, peak - 1, entry));
        assert_eq!(banked_peak(2, u64::MAX), None);
        assert_eq!(banked_peak(3, entry), None);
    }
}

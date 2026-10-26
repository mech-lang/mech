use crate::native_artifact::program::prepare::window_histogram::*;

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

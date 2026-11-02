use crate::native_artifact::program::runtime::collections::histogram_layout::*;
#[test]
fn policy_certificate_is_finite_checked_and_never_assigns_distinct_count() {
    certify(0, 1, 0, 0).unwrap();
    if usize::BITS == 64 {
        certify(1_000_000_000, 1, 1_000_000_000, 1_048_576).unwrap();
    }
    assert!(certify(u64::MAX, 1, 0, 0).is_err());
    assert!(certify(128, usize::MAX, 0, 0).is_err());
    assert!(certify(0, 1, u64::MAX, 0).is_err());
    assert!(certify(0, 1, 0, u64::MAX).is_err());
}
#[test]
fn policy_certificate_accepts_the_last_representable_bound_and_rejects_next() {
    let (mut low, mut high) = (0, u64::MAX);
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if certify(middle, 1, 0, 0).is_ok() {
            low = middle;
        } else {
            high = middle;
        }
    }
    certify(low, 1, 0, 0).unwrap();
    assert_eq!(high, low + 1);
    assert!(certify(high, 1, 0, 0).is_err());
}

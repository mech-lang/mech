//! Allocation-free ordering of at most 32 admitted independent jobs.

/// A bounded selection sort avoids general sorting machinery and scratch
/// allocation. At most 496 key comparisons and 31 swaps are needed. The caller
/// prepays bounded scheduler work and transport before constructing this slice.
pub(super) fn order_by_key<T, K: Ord>(values: &mut [T], key: impl Fn(&T) -> K) {
    debug_assert!(values.len() <= 32);
    for first in 0..values.len() {
        let mut next = first;
        for index in first + 1..values.len() {
            if key(&values[index]) < key(&values[next]) {
                next = index;
            }
        }
        if next != first {
            values.swap(first, next);
        }
    }
}

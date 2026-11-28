//! Native implementation of the witnessed final unit-gap transform.
use super::Heap;

/// # Safety
/// The caller proved that the original phase exclusively operates on this
/// initialized, writable U32 span and consumes its gap bytes in low-first
/// order with a zero terminator. A skipped route leaves the span untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_sort_gapped_u32(
    data: *mut u32,
    length: usize,
    gaps: u64,
) -> u64 {
    if !(32..=512).contains(&length) {
        return 0;
    }
    let last = gaps
        .to_le_bytes()
        .into_iter()
        .take_while(|&gap| gap != 0)
        .last();
    if last != Some(1) {
        return 0;
    }
    // Every positive-gap insertion preserves the multiset. A final unit gap
    // inserts into the complete prefix, yielding the unique ascending sequence
    // of unsigned values. Equal integers have identical stored bytes.
    let values = unsafe { core::slice::from_raw_parts_mut(data, length) };
    if !bounded_count_sort(values) {
        values.sort_unstable();
    }
    1
}

/// The same witnessed transform, with optional invocation-owned workspace.
/// No backing is allocated here, and missing workspace retains the existing
/// implementation. Saved snapshot prefixes remain untouched.
///
/// # Safety
/// The data span has the original transform's exclusive initialized U32
/// contract. A nonnull context is the caller's live exclusive heap, whose
/// private snapshot storage is disjoint from all source array backings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_sort_gapped_u32_with_scratch(
    context: *mut Heap,
    data: *mut u32,
    length: usize,
    gaps: u64,
) -> u64 {
    if !(32..=512).contains(&length)
        || gaps
            .to_le_bytes()
            .into_iter()
            .take_while(|&gap| gap != 0)
            .last()
            != Some(1)
    {
        return 0;
    }
    // SAFETY: the caller supplies the original exclusive initialized span.
    let values = unsafe { core::slice::from_raw_parts_mut(data, length) };
    if bounded_count_sort(values) {
        return 1;
    }
    // The phase is synchronous and cannot replace or recursively use the
    // transform cache. End this temporary loan before returning to it.
    let scratch = unsafe { context.as_mut() }
        .and_then(|heap| heap.registry.as_mut())
        .and_then(|registry| registry.fixed_transforms.as_mut())
        .and_then(|cache| cache.sort_scratch(length));
    if !scratch.is_some_and(|scratch| extended_count_sort(values, scratch)) {
        values.sort_unstable();
    }
    1
}

/// Preserve the original stable low-key admission, then scatter into an
/// existing private buffer instead of reserving another large stack array.
#[inline(never)]
fn extended_count_sort(words: &mut [u32], scratch: &mut [u32]) -> bool {
    const KEYS: usize = 256;
    if !(32..=MAX_WORDS).contains(&words.len()) || scratch.len() < words.len() {
        return false;
    }
    let mut offsets = [0u16; KEYS];
    let mut previous = 0;
    let mut highest = 0;
    for &word in words.iter() {
        let low = word & LOW_MASK;
        let high = (word >> 10) as usize;
        let Some(count) = offsets.get_mut(high) else {
            return false;
        };
        if low < previous {
            return false;
        }
        *count += 1;
        previous = low;
        highest = highest.max(high);
    }
    let mut running = 0;
    for slot in &mut offsets[..=highest] {
        let count = *slot;
        *slot = running;
        running += count;
    }
    for &word in words.iter() {
        // The admitted keys own disjoint prefix-sum intervals in words.len(),
        // and the scratch view covers that complete initialized destination.
        unsafe {
            let at = offsets.get_unchecked_mut((word >> 10) as usize);
            *scratch.get_unchecked_mut(usize::from(*at)) = word;
            *at += 1;
        }
    }
    words.copy_from_slice(&scratch[..words.len()]);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matched_unit_gap_sorts_every_word_and_other_gaps_preserve_the_input() {
        let mut seed = 5u32;
        for length in [0, 1, 31, 32, 37, 128, 255, 288, 512, 513] {
            for gaps in [0, 1, 0x01_07_2e, 0x01, 0x01_00_23, 0x02_ff, 0xff_01] {
                let mut input = (0..length)
                    .map(|i| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        if i % 2 == 0 { seed } else { i as u32 % 5 }
                    })
                    .collect::<Vec<_>>();
                let mut expected = input.clone();
                let mut remaining = gaps;
                let mut final_unit = false;
                loop {
                    let gap = (remaining & 255) as usize;
                    if gap == 0 {
                        break;
                    }
                    final_unit = gap == 1;
                    for from in gap..length {
                        let key = expected[from];
                        let mut at = from;
                        while at >= gap && expected[at - gap] > key {
                            expected[at] = expected[at - gap];
                            at -= gap;
                        }
                        expected[at] = key;
                    }
                    remaining >>= 8;
                }
                let original = input.clone();
                let selected =
                    unsafe { mech_compact_sort_gapped_u32(input.as_mut_ptr(), length, gaps) };
                if (32..=512).contains(&length) && final_unit {
                    assert_eq!((selected, input), (1, expected));
                } else {
                    assert_eq!((selected, input), (0, original));
                }
            }
        }
    }
}

use core::{mem::MaybeUninit, ptr};

const MAX_WORDS: usize = 288;
const HIGH_KEYS: usize = 64;
const LOW_MASK: u32 = (1 << 10) - 1;

/// Stably group by the high key if the input already orders the low key.
/// Within a group the present order therefore equals full unsigned order.
#[inline(never)]
fn bounded_count_sort(words: &mut [u32]) -> bool {
    if !(64..=MAX_WORDS).contains(&words.len()) {
        return false;
    }
    let mut offsets = [0u16; HIGH_KEYS];
    let mut previous = 0;
    for &word in words.iter() {
        let low = word & LOW_MASK;
        let high = (word >> 10) as usize;
        let Some(count) = offsets.get_mut(high) else {
            return false;
        };
        if low < previous {
            return false;
        }
        *count += 1;
        previous = low;
    }

    let mut running = 0;
    for slot in &mut offsets {
        let count = *slot;
        *slot = running;
        running += count;
    }
    debug_assert_eq!(usize::from(running), words.len());
    let mut sorted = [MaybeUninit::<u32>::uninit(); MAX_WORDS];
    for &word in words.iter() {
        let high = (word >> 10) as usize;
        // SAFETY: the counted pass checked every high key. Each group owns
        // an exact disjoint prefix-sum interval within words.len().
        unsafe {
            let position = offsets.get_unchecked_mut(high);
            sorted.get_unchecked_mut(usize::from(*position)).write(word);
            *position += 1;
        }
    }
    // SAFETY: each destination in the prefix was initialized once and the
    // temporary stack buffer is disjoint from the exclusive source view.
    unsafe {
        ptr::copy_nonoverlapping(
            sorted.as_ptr().cast::<u32>(),
            words.as_mut_ptr(),
            words.len(),
        );
    }
    true
}

#[cfg(test)]
#[test]
fn stable_counting_matches_unsigned_order_and_preserves_rejected_inputs() {
    let mut seed = 13u32;
    for length in [32, 63, 64, 128, 257, 258, MAX_WORDS, MAX_WORDS + 1, 320] {
        for mode in 0..128 {
            let mut words: Vec<u32> = (0..length)
                .map(|i| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    ((seed >> 26) << 10) | (i / 2) as u32
                })
                .collect();
            if mode % 4 == 1 {
                words[1] = 64 << 10;
            } else if mode % 4 == 2 {
                words.swap(0, 2);
            }
            let mut expected = words.clone();
            let admitted = (64..=MAX_WORDS).contains(&length) && (mode % 4 == 0 || mode % 4 == 3);
            if admitted {
                expected.sort_unstable();
            }
            assert_eq!(bounded_count_sort(&mut words), admitted);
            assert_eq!(words, expected);
        }
    }
}

#[cfg(test)]
#[test]
fn borrowed_counting_scratch_preserves_rejections_and_canaries() {
    let mut seed = 41u32;
    for length in [31, 32, 37, 63, 64, 128, 257, 288, 289] {
        for mode in 0..128 {
            let mut words: Vec<u32> = (0..length)
                .map(|index| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    ((seed >> 24) << 10) | (index / 2) as u32
                })
                .collect();
            if mode % 4 == 1 {
                words[1] = 256 << 10;
            } else if mode % 4 == 2 {
                words.swap(0, 2);
            }
            let available = length - usize::from(mode % 4 == 3);
            let mut scratch = vec![0xfeed_face; available + 2];
            let admitted = (32..=MAX_WORDS).contains(&length) && mode % 4 == 0;
            let mut expected = words.clone();
            if admitted {
                expected.sort_unstable();
            }
            assert_eq!(
                extended_count_sort(&mut words, &mut scratch[1..=available]),
                admitted
            );
            assert_eq!(words, expected);
            assert_eq!(scratch[0], 0xfeed_face);
            assert_eq!(scratch[available + 1], 0xfeed_face);
            if !admitted {
                assert!(scratch.iter().all(|&value| value == 0xfeed_face));
            }
        }
    }
}

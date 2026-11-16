use super::{Heap, View};
use std::ptr;

/// Allocate the borrowed prefix once and finish a proved no-growth append.
///
/// # Safety
/// The heap is exclusively borrowed. The aligned six-word prefix and output
/// packets are disjoint from each other and the heap. The prefix owner is zero
/// and its initialized bytes and the source span may share read-only backing.
/// The caller proves that the exact prefix capacity holds the full result,
/// that the packed write has 1–8 bytes, and that the span has at most 15 bytes.
/// Both inputs remain valid until return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_finish_byte_appends(
    heap: *mut Heap,
    output: *mut View,
    prefix: *const View,
    packed: u64,
    packed_len: usize,
    span: *const u8,
    span_len: usize,
) -> u64 {
    // SAFETY: the caller owns the heap and lends the initialized prefix packet.
    let heap = unsafe { &mut *heap };
    let source = unsafe { prefix.read() };
    let length = source[1] as usize;
    let capacity = source[2] as usize;
    debug_assert!(source[5] == 0 && (1..=8).contains(&packed_len) && span_len < 16);
    debug_assert!(length + packed_len + span_len <= capacity);
    let slot = heap.owner_storage.filter(|_| heap.first_owner.is_null());
    let (dest, owner) = if let Some(owner) = slot {
        let Ok(layout) = super::Layout::from_size_align(capacity, 1) else {
            return 1;
        };
        // SAFETY: checked byte layout with nonzero capacity from the write.
        let data = unsafe { super::alloc(layout) };
        if data.is_null() {
            return super::ALLOCATION;
        }
        let owner = owner.as_ptr();
        // SAFETY: the loan keeps this empty slot stable until heap retirement.
        unsafe {
            owner.write(super::Owner {
                references: 1,
                slot: 0,
                data,
                capacity,
                stride: 1,
                alignment_and_mapping: 1,
            });
        }
        heap.first_owner = owner;
        let counters = &mut heap.counters;
        counters.payload_requests += 1;
        counters.requested_payload_bytes += capacity as u64;
        counters.live_payload_bytes += capacity as u64;
        counters.peak_payload_bytes = counters.peak_payload_bytes.max(counters.live_payload_bytes);
        (data, owner)
    } else {
        match heap.allocate(capacity, 1, 1, false) {
            Ok(view) => (view[0] as *mut u8, view[5] as *mut super::Owner),
            Err(status) => return status,
        }
    };
    // SAFETY: the new allocation is disjoint from immutable prefix/span
    // storage. The width decomposition writes precisely the admitted extent.
    unsafe {
        if length != 0 {
            ptr::copy_nonoverlapping(source[0] as *const u8, dest, length);
        }
        let tail = dest.add(length);
        // The first and last words cover the exact extent. Overlapping
        // writes store the same bytes where they meet.
        if packed_len >= 4 {
            let last = packed_len - 4;
            tail.cast::<u32>().write_unaligned((packed as u32).to_le());
            tail.add(last)
                .cast::<u32>()
                .write_unaligned(((packed >> (last * 8)) as u32).to_le());
        } else if packed_len >= 2 {
            let last = packed_len - 2;
            tail.cast::<u16>().write_unaligned((packed as u16).to_le());
            tail.add(last)
                .cast::<u16>()
                .write_unaligned(((packed >> (last * 8)) as u16).to_le());
        } else {
            tail.write(packed as u8);
        }
        let dest = tail.add(packed_len);
        if span_len >= 8 {
            copy_ends::<u64>(span, dest, span_len);
        } else if span_len >= 4 {
            copy_ends::<u32>(span, dest, span_len);
        } else if span_len >= 2 {
            copy_ends::<u16>(span, dest, span_len);
        } else if span_len == 1 {
            dest.write(span.read());
        }
    }
    heap.counters.initialized_bytes_copied += length as u64;
    let total = (length + packed_len + span_len) as u64;
    let result = [dest as u64, total, capacity as u64, 1, total, owner as u64];
    // SAFETY: initialization is complete and the only result owner is live.
    unsafe {
        (*owner).references = 1;
        output.write(result);
    }
    0
}

/// Copy the first and last words of a span, including a possible overlap.
///
/// # Safety
/// The source and destination ranges are valid for `bytes` and have disjoint
/// backing. A word of type `T` fits within the span and covers at least half
/// of it. Both source words are initialized and valid instances of `T`.
#[inline]
unsafe fn copy_ends<T: Copy>(span: *const u8, dest: *mut u8, bytes: usize) {
    let last = bytes - std::mem::size_of::<T>();
    // SAFETY: both ranges have bytes initialized for reading or allocated
    // for writing as appropriate. Their backing allocations are disjoint.
    // If the destination stores overlap, they write the same bytes.
    unsafe {
        dest.cast::<T>()
            .write_unaligned(span.cast::<T>().read_unaligned());
        dest.add(last)
            .cast::<T>()
            .write_unaligned(span.add(last).cast::<T>().read_unaligned());
    }
}

#[cfg(test)]
mod copy_ends_tests {
    use super::*;
    use crate::native_artifact::compact_runtime::OwnerStorage;

    #[test]
    fn all_packed_and_span_extents_preserve_bytes_prefix_and_counters() {
        let packed = 0xb382_7160_5948_3726u64;
        let packed_bytes = packed.to_le_bytes();
        let borrowed = b"immutable-prefix-and-source-span";
        for packed_len in 1..=8 {
            for span_len in 0..16 {
                for (prefix_len, extra) in [(0, 0), (3, 2), (8, 32)] {
                    for use_owner_storage in [false, true] {
                        let length = prefix_len + packed_len + span_len;
                        let capacity = length + extra;
                        let prefix = [
                            borrowed.as_ptr() as u64,
                            prefix_len as u64,
                            capacity as u64,
                            1,
                            prefix_len as u64,
                            0,
                        ];
                        let mut storage = OwnerStorage::default();
                        let mut heap = if use_owner_storage {
                            Heap::with_owner_storage(&mut storage)
                        } else {
                            Heap::new()
                        };
                        let mut output = [0; 6];
                        // The live copied span and prefix may come from the same
                        // backing. The generated result acquires a distinct allocation.
                        assert_eq!(
                            unsafe {
                                mech_compact_finish_byte_appends(
                                    &mut heap,
                                    &mut output,
                                    &prefix,
                                    packed,
                                    packed_len,
                                    borrowed.as_ptr(),
                                    span_len,
                                )
                            },
                            0
                        );
                        assert_eq!(
                            [output[1], output[2], output[3], output[4]],
                            [length as u64, capacity as u64, 1, length as u64]
                        );
                        let result = unsafe { heap.take_u8(output).unwrap() };
                        assert_ne!(result.as_ptr(), borrowed.as_ptr());
                        assert_eq!(&result[..prefix_len], &borrowed[..prefix_len]);
                        assert_eq!(
                            &result[prefix_len..prefix_len + packed_len],
                            &packed_bytes[..packed_len]
                        );
                        assert_eq!(&result[prefix_len + packed_len..], &borrowed[..span_len]);
                        assert_eq!(result.capacity(), capacity);
                        assert_eq!(heap.counters.payload_requests, 1);
                        assert_eq!(heap.counters.requested_payload_bytes, capacity as u64);
                        assert_eq!(heap.counters.initialized_bytes_copied, prefix_len as u64);
                        assert_eq!(heap.counters.zeroed_bytes, 0);
                        assert_eq!(heap.counters.live_payload_bytes, 0);
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_allocation_preserves_the_result_packet() {
        let borrowed = [0xabu8];
        let prefix = [borrowed.as_ptr() as u64, 1, usize::MAX as u64, 1, 1, 0];
        let mut storage = OwnerStorage::default();
        let mut heap = Heap::with_owner_storage(&mut storage);
        let mut output = [u64::MAX; 6];
        let status = unsafe {
            mech_compact_finish_byte_appends(
                &mut heap,
                &mut output,
                &prefix,
                1,
                1,
                borrowed.as_ptr(),
                0,
            )
        };
        assert_eq!(status, 1);
        assert_eq!(output, [u64::MAX; 6]);
        assert_eq!(heap.counters.payload_requests, 0);
        assert_eq!(borrowed, [0xab]);
    }
}

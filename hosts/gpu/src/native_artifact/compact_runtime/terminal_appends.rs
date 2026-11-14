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
        if packed_len & 8 != 0 {
            tail.cast::<u64>().write_unaligned(packed.to_le());
        }
        if packed_len & 4 != 0 {
            tail.cast::<u32>().write_unaligned((packed as u32).to_le());
        }
        if packed_len & 2 != 0 {
            let at = packed_len & 4;
            tail.add(at)
                .cast::<u16>()
                .write_unaligned(((packed >> (at * 8)) as u16).to_le());
        }
        if packed_len & 1 != 0 {
            let at = packed_len & 6;
            tail.add(at).write((packed >> (at * 8)) as u8);
        }
        let dest = tail.add(packed_len);
        if span_len & 8 != 0 {
            dest.cast::<u64>()
                .write_unaligned(span.cast::<u64>().read_unaligned());
        }
        if span_len & 4 != 0 {
            let at = span_len & 8;
            dest.add(at)
                .cast::<u32>()
                .write_unaligned(span.add(at).cast::<u32>().read_unaligned());
        }
        if span_len & 2 != 0 {
            let at = span_len & 12;
            dest.add(at)
                .cast::<u16>()
                .write_unaligned(span.add(at).cast::<u16>().read_unaligned());
        }
        if span_len & 1 != 0 {
            let at = span_len & 14;
            dest.add(at).write(span.add(at).read());
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

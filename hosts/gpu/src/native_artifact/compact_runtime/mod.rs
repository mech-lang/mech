//! Concrete diagnostic array backing. No schemas, library binding, dispatch,
//! source word banks or canonical Value identities participate in these calls.
use std::{
    alloc::{Layout, alloc, alloc_zeroed, dealloc},
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::{self, NonNull},
};
mod terminal_appends;
#[path = "zero_pages.rs"]
mod zero_pages;
pub use terminal_appends::mech_compact_finish_byte_appends;
pub const ALLOCATION: u64 = 0x19402;
pub const INVALID: u64 = 0x19403;
pub type View = [u64; 6];

#[repr(C)]
pub struct Owner {
    pub references: u64,
    slot: usize,
    data: *mut u8,
    capacity: usize,
    stride: usize,
    alignment_and_mapping: usize,
}
impl Owner {
    // A nonempty validated Layout reserves this high bit. Keeping the backing
    // flag there preserves the original six-word owner descriptor.
    const MAPPED: usize = 1usize << (usize::BITS - 1);

    fn alignment(&self) -> usize {
        self.alignment_and_mapping & !Self::MAPPED
    }

    fn is_mapped(&self) -> bool {
        self.alignment_and_mapping & Self::MAPPED != 0
    }
}
#[derive(Default, Debug)]
pub struct Counters {
    pub parallel_peak_payload_upper_bound: u64,
    pub payload_requests: u64,
    pub requested_payload_bytes: u64,
    pub initialized_bytes_copied: u64,
    pub zeroed_bytes: u64,
    #[cfg(test)]
    pub mapped_payload_requests: u64,
    #[cfg(test)]
    pub mapped_payload_bytes: u64,
    #[cfg(test)]
    pub live_mapped_payload_bytes: u64,
    pub live_payload_bytes: u64,
    pub peak_payload_bytes: u64,
}
#[derive(Default, Debug)]
pub struct ScratchAudit {
    pub allocations: u64,
    pub copies: u64,
    pub copied_bytes: u64,
    pub releases: u64,
    pub overwritten_words: u64,
    pub last_overwritten_word: u64,
}

// Diagnostic for the pinned fixture only. Both poison values fit U32 and
// cannot be a legal packed level-12 match/header.
const CACHE_WORDS: usize = 1_500_513;

/// Storage for one owner descriptor borrowed by an opaque native invocation.
pub struct OwnerStorage(MaybeUninit<Owner>);

impl Default for OwnerStorage {
    fn default() -> Self {
        Self(MaybeUninit::uninit())
    }
}

#[derive(Default)]
pub struct Heap<'storage> {
    pub parallel_peak_workers: u64,
    pub parallel_batches: u64,
    // Slot zero is inline. The vectors use zero-based indices for slots one and up.
    first_owner: *mut Owner,
    owner_storage: Option<NonNull<Owner>>,
    owner_loan: PhantomData<&'storage mut OwnerStorage>,
    owners: Vec<*mut Owner>,
    free: Vec<usize>,
    pub counters: Counters,
    pub scratch_audit: ScratchAudit,
    pub scratch_poison: Option<u32>,
}
impl Heap<'static> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'storage> Heap<'storage> {
    /// Keep the first record at a stable address owned by the caller. The heap
    /// cannot outlive that exclusive loan. Scratch payload allocation and the
    /// owner token exposed to generated code keep their existing representation.
    pub fn with_owner_storage(storage: &'storage mut OwnerStorage) -> Self {
        let mut heap = Self::default();
        heap.owner_storage = Some(NonNull::from(&mut storage.0).cast());
        heap
    }

    fn owner_at(&self, slot: usize) -> Option<*mut Owner> {
        if slot == 0 {
            Some(self.first_owner)
        } else {
            self.owners.get(slot - 1).copied()
        }
    }

    /// The pointer is a fresh or transferred owner, exclusively held here.
    unsafe fn register(&mut self, owner: *mut Owner) {
        let slot = if self.first_owner.is_null() {
            self.first_owner = owner;
            0
        } else if let Some(index) = self.free.pop() {
            self.owners[index] = owner;
            index + 1
        } else {
            self.owners.push(owner);
            self.owners.len()
        };
        unsafe { (*owner).slot = slot };
    }

    pub fn discard_private(&mut self) {
        if !self.first_owner.is_null() {
            let owner = self.first_owner;
            unsafe {
                (*owner).references = 0;
                self.release(owner)
                    .expect("owned diagnostic backing cleanup");
            }
        }
        for index in 0..self.owners.len() {
            let owner = self.owners[index];
            if !owner.is_null() {
                unsafe {
                    (*owner).references = 0;
                    self.release(owner)
                        .expect("owned diagnostic backing cleanup");
                }
            }
        }
        assert_eq!(self.counters.live_payload_bytes, 0);
        #[cfg(test)]
        assert_eq!(self.counters.live_mapped_payload_bytes, 0);
    }
    fn allocate(
        &mut self,
        capacity: usize,
        stride: usize,
        alignment: usize,
        zeroed: bool,
    ) -> Result<View, u64> {
        if stride == 0 || alignment == 0 || !alignment.is_power_of_two() {
            return Err(INVALID);
        }
        let bytes = capacity.checked_mul(stride).ok_or(1u64)?;
        let layout = Layout::from_size_align(bytes, alignment).map_err(|_| 1u64)?;
        if bytes == 0 {
            return Ok([alignment as u64, 0, 0, 1, 0, 0]);
        }
        debug_assert_eq!(alignment & Owner::MAPPED, 0);
        // Fresh page mappings provide the same initialized zero words.
        // Byte outputs keep their original allocator for direct Vec transfer.
        let mapping = if zeroed && stride != 1 {
            zero_pages::reserve(layout)?
        } else {
            None
        };
        let mapped = mapping.is_some();
        let data = if let Some(mapping) = mapping {
            mapping.as_ptr()
        } else {
            // SAFETY: checked nonzero layout. The selected allocator operation
            // establishes the original initialization contract.
            unsafe {
                if zeroed {
                    alloc_zeroed(layout)
                } else {
                    alloc(layout)
                }
            }
        };
        if data.is_null() {
            return Err(ALLOCATION);
        }
        if stride == 4 && capacity == CACHE_WORDS && zeroed {
            self.scratch_audit.allocations += 1;
            if let Some(poison) = self.scratch_poison {
                // The original allocator and its initialization/failure
                // behavior have completed. This separate diagnostic supplies
                // alternate initialized bit patterns before native use.
                for index in 0..capacity {
                    unsafe { data.cast::<u32>().add(index).write(poison) };
                }
            }
        }
        let owner = Owner {
            references: 0,
            slot: 0,
            data,
            capacity,
            stride,
            alignment_and_mapping: if mapped {
                alignment | Owner::MAPPED
            } else {
                alignment
            },
        };
        let owner = if self.first_owner.is_null()
            && let Some(storage) = self.owner_storage
        {
            // SAFETY: the heap holds the exclusive storage loan. A free first
            // slot means that no live token points into the loan.
            unsafe { storage.as_ptr().write(owner) };
            storage.as_ptr()
        } else {
            Box::into_raw(Box::new(owner))
        };
        // SAFETY: this fresh owner is transferred to the scope of this heap.
        unsafe { self.register(owner) };
        self.counters.payload_requests += 1;
        self.counters.requested_payload_bytes += bytes as u64;
        self.counters.live_payload_bytes += bytes as u64;
        self.counters.peak_payload_bytes = self
            .counters
            .peak_payload_bytes
            .max(self.counters.live_payload_bytes);
        if zeroed {
            self.counters.zeroed_bytes += bytes as u64
        }
        #[cfg(test)]
        if mapped {
            self.counters.mapped_payload_requests += 1;
            self.counters.mapped_payload_bytes += bytes as u64;
            self.counters.live_mapped_payload_bytes += bytes as u64;
        }
        Ok([data as u64, 0, capacity as u64, 1, 0, owner as u64])
    }
    unsafe fn release(&mut self, owner: *mut Owner) -> Result<(), u64> {
        if owner.is_null() {
            return Ok(());
        }
        // SAFETY: only the owning native transaction supplies concrete owner
        // pointers, and a zero refcount is required before physical release.
        let record = unsafe { &*owner };
        if record.references != 0 || self.owner_at(record.slot) != Some(owner) {
            return Err(INVALID);
        }
        let bytes = record.capacity.checked_mul(record.stride).ok_or(INVALID)?;
        let layout = Layout::from_size_align(bytes, record.alignment()).map_err(|_| INVALID)?;
        let slot = record.slot;
        let data = record.data;
        if record.stride == 4 && record.capacity == CACHE_WORDS && record.is_mapped() {
            self.scratch_audit.releases += 1;
            if let Some(poison) = self.scratch_poison {
                // Every original word was initialized by the mapping and the
                // diagnostic fill, so untouched words can be inspected here.
                let words = unsafe { std::slice::from_raw_parts(data.cast::<u32>(), CACHE_WORDS) };
                for (index, &word) in words.iter().enumerate() {
                    if word != poison {
                        self.scratch_audit.overwritten_words += 1;
                        self.scratch_audit.last_overwritten_word = (index + 1) as u64;
                    }
                }
            }
        }
        if record.is_mapped() {
            // SAFETY: this exact mapping is live, exclusive and registered.
            unsafe { zero_pages::release(data, bytes)? };
            #[cfg(test)]
            {
                self.counters.live_mapped_payload_bytes -= bytes as u64;
            }
        } else {
            // SAFETY: this pointer retains its original allocator layout.
            unsafe { dealloc(data, layout) };
        }
        // SAFETY: this owner belongs to the heap and its payload is retired.
        unsafe { self.retire_record(owner) };
        if slot == 0 {
            self.first_owner = ptr::null_mut();
        } else {
            let index = slot - 1;
            self.owners[index] = ptr::null_mut();
            self.free.push(index);
        }
        self.counters.live_payload_bytes -= bytes as u64;
        Ok(())
    }
    unsafe fn retire_record(&self, owner: *mut Owner) {
        if self
            .owner_storage
            .is_some_and(|storage| storage.as_ptr() == owner)
        {
            // SAFETY: the current registered record was initialized in this
            // slot. End its lifetime while retaining the storage loan.
            unsafe { owner.drop_in_place() };
        } else {
            // SAFETY: all other records are allocated as Box<Owner>.
            unsafe { drop(Box::from_raw(owner)) };
        }
    }

    fn writable(
        &mut self,
        source: View,
        consume: bool,
        minimum: usize,
        stride: usize,
        alignment: usize,
    ) -> Result<View, u64> {
        let length = usize::try_from(source[1]).map_err(|_| INVALID)?;
        let capacity = usize::try_from(source[2]).map_err(|_| INVALID)?;
        if length > capacity || minimum < length {
            return Err(INVALID);
        }
        let owner = source[5] as *mut Owner;
        if consume && !owner.is_null() {
            // SAFETY: current private owner came from this live transaction;
            // native last-use permits replacing this source, not a live alias.
            let record = unsafe { &*owner };
            if self.owner_at(record.slot) != Some(owner)
                || record.stride != stride
                || record.alignment() != alignment
                || record.data as u64 != source[0]
                || record.capacity != capacity
            {
                return Err(INVALID);
            }
            if record.references == 1 && minimum <= capacity {
                return Ok(source);
            }
        }
        let mut result = self.allocate(capacity.max(minimum), stride, alignment, false)?;
        let bytes = length.checked_mul(stride).ok_or(1u64)?;
        if bytes != 0 {
            // SAFETY: source's actual initialized prefix is valid for this
            // synchronous call; fresh destination has at least that capacity.
            unsafe {
                ptr::copy_nonoverlapping(source[0] as *const u8, result[0] as *mut u8, bytes)
            };
        }
        if stride == 4 && capacity == CACHE_WORDS && bytes != 0 {
            self.scratch_audit.copies += 1;
            self.scratch_audit.copied_bytes += bytes as u64;
        }
        self.counters.initialized_bytes_copied += bytes as u64;
        result[1] = source[1];
        result[3] = source[3];
        result[4] = source[4];
        Ok(result)
    }
}
impl Drop for Heap<'_> {
    fn drop(&mut self) {
        // A failed private transaction publishes no roots. Generated values
        // and views cannot escape this lifetime; discard all remaining scratch.
        self.discard_private();
    }
}

/// # Safety
/// Current owning Heap and a disjoint six-word output lane outlive the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_allocate(
    context: *mut Heap,
    capacity: u64,
    stride: u64,
    alignment: u64,
    zeroed: u64,
    output: *mut u64,
) -> u64 {
    if context.is_null() || output.is_null() {
        return INVALID;
    }
    let result = unsafe { &mut *context }.allocate(
        capacity as usize,
        stride as usize,
        alignment as usize,
        zeroed != 0,
    );
    match result {
        Ok(view) => {
            unsafe { ptr::copy_nonoverlapping(view.as_ptr(), output, 6) };
            0
        }
        Err(status) => status,
    }
}
/// # Safety
/// Both lanes and the current concrete backing owners belong to this live
/// transaction. `consume` additionally requires the compiler's last-use proof.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_writable(
    context: *mut Heap,
    source: *const u64,
    consume: u64,
    minimum: u64,
    stride: u64,
    alignment: u64,
    output: *mut u64,
) -> u64 {
    if context.is_null() || source.is_null() || output.is_null() {
        return INVALID;
    }
    let view = unsafe { ptr::read(source.cast::<View>()) };
    let result = unsafe { &mut *context }.writable(
        view,
        consume != 0,
        minimum as usize,
        stride as usize,
        alignment as usize,
    );
    match result {
        Ok(view) => {
            unsafe { ptr::copy_nonoverlapping(view.as_ptr(), output, 6) };
            0
        }
        Err(status) => status,
    }
}
/// # Safety
/// The generated final release has decremented this current owner's count to0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_free(context: *mut Heap, owner: *mut Owner) -> u64 {
    if context.is_null() {
        return INVALID;
    }
    match unsafe { (&mut *context).release(owner) } {
        Ok(()) => 0,
        Err(status) => status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_owner_storage_survives_heap_moves_and_failure_retries() {
        let mut storage = OwnerStorage::default();
        let address = storage.0.as_mut_ptr();
        let mut heap = Heap::with_owner_storage(&mut storage);
        let mut output = heap.allocate(13, 1, 1, false).unwrap();
        let owner = output[5] as *mut Owner;
        assert_eq!(owner, address);
        let scratch = heap.allocate(4, 8, 8, false).unwrap();
        assert_ne!(scratch[5] as *mut Owner, owner);
        unsafe {
            (output[0] as *mut u8).write(17);
            (*owner).references = 2;
        }
        output[1] = 1;
        output[4] = 1;
        let mut moved = heap;
        assert_eq!(unsafe { moved.take_u8(output) }, Err(INVALID));
        unsafe { (*owner).references = 1 };
        let published = unsafe { moved.take_u8(output) }.unwrap();
        assert_eq!(published, [17]);
        assert_eq!(published.capacity(), 13);
        let retry = moved.allocate(3, 2, 2, false).unwrap();
        assert_eq!(retry[5] as *mut Owner, address);
        moved.discard_private();
        assert_eq!(moved.counters.live_payload_bytes, 0);
        assert_eq!(published, [17]);
    }

    #[test]
    fn inline_owner_transfers_with_live_tail_and_reuses_its_slot() {
        let mut heap = Heap::new();
        let mut result = heap.allocate(13, 1, 1, false).unwrap();
        let owner = result[5] as *mut Owner;
        assert_eq!(heap.owner_at(0), Some(owner));
        assert_eq!(heap.owners.capacity(), 0);
        assert_eq!(heap.free.capacity(), 0);
        let scratch = heap.allocate(4, 8, 8, false).unwrap();
        let scratch_owner = scratch[5] as *mut Owner;
        assert_eq!(heap.owner_at(1), Some(scratch_owner));
        unsafe {
            (result[0] as *mut u8).write(17);
            (*owner).references = 2;
        }
        result[1] = 1;
        result[4] = 1;
        assert_eq!(unsafe { heap.take_u8(result) }, Err(INVALID));
        assert_eq!(heap.owner_at(0), Some(owner));
        unsafe { (*owner).references = 1 };
        let published = unsafe { heap.take_u8(result) }.unwrap();
        assert_eq!(published, [17]);
        assert_eq!(published.capacity(), 13);
        assert_eq!(heap.owner_at(0), Some(ptr::null_mut()));
        assert_eq!(heap.owner_at(1), Some(scratch_owner));
        assert_eq!(heap.free.capacity(), 0);
        let next = heap.allocate(9, 2, 2, false).unwrap();
        assert_eq!(heap.owner_at(0), Some(next[5] as *mut Owner));
        heap.discard_private();
        assert_eq!(heap.counters.live_payload_bytes, 0);
        assert_eq!(heap.owner_at(0), Some(ptr::null_mut()));
        assert_eq!(heap.owner_at(1), Some(ptr::null_mut()));
        assert_eq!(published, [17]);
    }

    #[test]
    fn live_alias_copies_initialized_prefix_and_keeps_capacity() {
        let mut heap = Heap::new();
        let mut original = heap.allocate(305257, 8, 8, false).unwrap();
        unsafe {
            (original[0] as *mut u64).write(17);
            (original[0] as *mut u64).add(1).write(29);
            (*(original[5] as *mut Owner)).references = 2;
        }
        original[1] = 2;
        original[4] = 2;
        let copy = heap.writable(original, true, 2, 8, 8).unwrap();
        assert_ne!(copy[0], original[0]);
        assert_eq!(copy[2], 305257);
        assert_eq!(heap.counters.initialized_bytes_copied, 16);
        unsafe {
            (copy[0] as *mut u64).write(99);
            assert_eq!((original[0] as *const u64).read(), 17);
        }
        let before = heap.counters.payload_requests;
        unsafe {
            (*(original[5] as *mut Owner)).references = 1;
        }
        let reused = heap.writable(original, true, 2, 8, 8).unwrap();
        assert_eq!(reused, original);
        assert_eq!(heap.counters.payload_requests, before);
    }
    #[test]
    fn real_cache_and_tree_are_initialized_but_path_reserve_is_not() {
        let mut heap = Heap::new();
        let tree = heap.allocate(262144, 2, 2, true).unwrap();
        let cache = heap.allocate(1500513, 4, 4, true).unwrap();
        let nodes = heap.allocate(305257, 8, 8, false).unwrap();
        unsafe {
            assert_eq!((tree[0] as *const u16).add(262143).read(), 0);
            assert_eq!((cache[0] as *const u32).add(1500512).read(), 0);
        }
        assert_eq!(heap.counters.zeroed_bytes, 262144 * 2 + 1500513 * 4);
        assert_eq!(nodes[1], 0);
        assert_eq!(nodes[2], 305257);
        assert_eq!(heap.counters.initialized_bytes_copied, 0);
    }
    #[test]
    fn clear_keeps_private_capacity_and_copies_no_elements() {
        let mut heap = Heap::new();
        let mut source = heap.allocate(305257, 8, 8, false).unwrap();
        unsafe {
            (*(source[5] as *mut Owner)).references = 1;
        }
        // A clear result has an empty initialized prefix. No unread bytes are
        // exposed and no old element is copied into a new backing.
        source[1] = 0;
        source[3] = 1;
        source[4] = 0;
        let requests = heap.counters.payload_requests;
        assert_eq!(heap.writable(source, true, 0, 8, 8).unwrap(), source);
        assert_eq!(heap.counters.payload_requests, requests);
        unsafe {
            (*(source[5] as *mut Owner)).references = 2;
        }
        let other = heap.writable(source, true, 0, 8, 8).unwrap();
        assert_ne!(other[0], source[0]);
        assert_eq!(other[2], 305257);
        assert_eq!(heap.counters.initialized_bytes_copied, 0);
    }
    #[test]
    fn rejected_physical_size_keeps_source_and_result_lane_and_recovers() {
        let mut heap = Heap::new();
        let mut source = heap.allocate(13, 1, 1, false).unwrap();
        source[1] = 2;
        source[4] = 2;
        unsafe {
            (source[0] as *mut u8).write(17);
            (source[0] as *mut u8).add(1).write(29);
            (*(source[5] as *mut Owner)).references = 2;
        }
        let before = source;
        let mut output = [0x5555555555555555u64; 6];
        let expected = output;
        // usize::MAX cannot describe a Rust allocation. This rejects without
        // attempting an enormous allocation or changing current owned storage.
        let status = unsafe {
            mech_compact_writable(
                &mut heap,
                source.as_ptr(),
                0,
                u64::MAX,
                1,
                1,
                output.as_mut_ptr(),
            )
        };
        assert_eq!(status, 1);
        assert_eq!(source, before);
        assert_eq!(output, expected);
        unsafe {
            assert_eq!((source[0] as *const u8).read(), 17);
            assert_eq!((source[0] as *const u8).add(1).read(), 29);
            assert_eq!((*(source[5] as *const Owner)).references, 2);
        }
        let status = unsafe {
            mech_compact_writable(&mut heap, source.as_ptr(), 0, 3, 1, 1, output.as_mut_ptr())
        };
        assert_eq!(status, 0);
        assert_eq!(output[1], 2);
        assert_eq!(output[2], 13);
        assert_ne!(output[0], source[0]);
        assert_eq!(heap.counters.initialized_bytes_copied, 2);
    }
}

impl Heap<'_> {
    /// Transfer the sole final U8 allocation without a payload copy. All view
    /// aliases are counted by the generated typed source-value lifetime code.
    ///
    /// # Safety
    /// The view must originate from this heap in the exact generated entry.
    #[inline]
    pub unsafe fn take_u8(&mut self, view: View) -> Result<Vec<u8>, u64> {
        let [pointer, length, capacity, rows, columns, owner] = view;
        if rows != 1 || columns != length || length > capacity || owner == 0 {
            return Err(INVALID);
        }
        let owner = owner as *mut Owner;
        // SAFETY: final views are returned by the exact linked native body. A
        // current heap slot, exact backing geometry and sole reference are all
        // checked before removing that owner and constructing the Vec.
        let record = unsafe { &*owner };
        if self.owner_at(record.slot) != Some(owner)
            || record.references != 1
            || record.stride != 1
            || record.alignment_and_mapping != 1
            || record.data as u64 != pointer
            || record.capacity as u64 != capacity
        {
            return Err(INVALID);
        }
        let slot = record.slot;
        if slot == 0 {
            self.first_owner = ptr::null_mut();
        } else if slot == self.owners.len() {
            self.owners.pop();
        } else {
            let index = slot - 1;
            self.owners[index] = ptr::null_mut();
            self.free.push(index);
        }
        self.counters.live_payload_bytes -= capacity;
        // SAFETY: the validated record is sole and belongs to the current heap.
        unsafe { self.retire_record(owner) };
        // Allocation uses this exact U8 layout through the pinned global
        // allocator. The Vec becomes its sole owner and retains full capacity.
        let result =
            unsafe { Vec::from_raw_parts(pointer as *mut u8, length as usize, capacity as usize) };
        Ok(result)
    }
}

#[cfg(all(test, target_os = "macos"))]
mod mapped_tests {
    use super::*;

    #[test]
    fn mapped_numeric_zeros_remain_readable_and_live_aliases_keep_their_words() {
        assert_eq!(
            std::mem::size_of::<Owner>(),
            6 * std::mem::size_of::<usize>()
        );
        let mut heap = Heap::new();
        let mut source = heap.allocate(262144, 4, 4, true).unwrap();
        let bytes = 262144 * 4;
        assert_eq!(heap.counters.mapped_payload_requests, 1);
        assert_eq!(heap.counters.mapped_payload_bytes, bytes);
        let old = unsafe { std::slice::from_raw_parts_mut(source[0] as *mut u32, 262144) };
        assert!(old.iter().all(|value| *value == 0));
        old[0] = 17;
        old[262143] = 29;
        source[1] = 262144;
        source[4] = 262144;
        unsafe { (*(source[5] as *mut Owner)).references = 2 };
        let copied = heap.writable(source, true, 262144, 4, 4).unwrap();
        assert_ne!(source[0], copied[0]);
        assert_eq!(copied[2], source[2]);
        unsafe { (copied[0] as *mut u32).write(99) };
        assert_eq!(old[0], 17);
        assert_eq!(old[262143], 29);
        assert_eq!(heap.counters.initialized_bytes_copied, bytes);
        assert_eq!(heap.counters.mapped_payload_requests, 1);
        heap.discard_private();
        assert_eq!(heap.counters.live_mapped_payload_bytes, 0);
    }

    #[test]
    fn failed_mapping_keeps_prior_backing_and_recovers_on_retry() {
        let mut heap = Heap::new();
        let prior = heap.allocate(262144, 4, 4, true).unwrap();
        unsafe { (prior[0] as *mut u32).write(73) };
        let requests = heap.counters.payload_requests;
        zero_pages::DENY.with(|deny| deny.set(true));
        assert_eq!(heap.allocate(1500513, 4, 4, true), Err(ALLOCATION));
        zero_pages::DENY.with(|deny| deny.set(false));
        assert_eq!(heap.counters.payload_requests, requests);
        assert_eq!(unsafe { (prior[0] as *const u32).read() }, 73);
        let retry = heap.allocate(1500513, 4, 4, true).unwrap();
        assert_eq!(unsafe { (retry[0] as *const u32).add(1500512).read() }, 0);
        heap.discard_private();
        assert_eq!(heap.counters.live_payload_bytes, 0);
        assert_eq!(heap.counters.live_mapped_payload_bytes, 0);
    }

    #[test]
    fn byte_output_retains_the_original_allocator_and_transfer() {
        let mut heap = Heap::new();
        let mut output = heap.allocate(131072, 1, 1, true).unwrap();
        assert_eq!(heap.counters.mapped_payload_requests, 0);
        unsafe {
            (output[0] as *mut u8).write(42);
            (*(output[5] as *mut Owner)).references = 1;
        }
        output[1] = 1;
        output[4] = 1;
        let published = unsafe { heap.take_u8(output) }.unwrap();
        assert_eq!(published, [42]);
        assert_eq!(published.capacity(), 131072);
        assert_eq!(heap.counters.live_payload_bytes, 0);
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn deny_test_mapping(denied: bool) {
    zero_pages::DENY.with(|deny| deny.set(denied));
}

/// Concrete native thread entry and storage transfer. Captures are immutable
/// typed word packets with zero borrowed owner lanes. Workers execute the
/// generated function, own disjoint heaps, and finish before this call returns.
/// No source operation, schema, tag, post or scoring algorithm is known here.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeStatus {
    pub status: u64,
    pub origin: u64,
}
struct Completed {
    heap: Heap<'static>,
    status: NativeStatus,
}
// SAFETY: after its native call returns, this job exclusively owns every live
// allocation in this heap. Borrowed captures have no owner records here. The
// scoped join transfers this exclusive ownership to its invoking thread.
unsafe impl Send for Completed {}
impl Heap<'_> {
    fn adopt(&mut self, other: &mut Heap<'static>) {
        debug_assert!(
            other.owner_storage.is_none(),
            "workers use owned descriptors"
        );
        for owner in std::iter::once(other.first_owner).chain(other.owners.iter().copied()) {
            if owner.is_null() {
                continue;
            }
            // SAFETY: the joined worker transfers each live concrete allocation
            // once. No other job or thread can subsequently touch its owner.
            unsafe { self.register(owner) };
        }
        self.counters.payload_requests += other.counters.payload_requests;
        self.counters.requested_payload_bytes += other.counters.requested_payload_bytes;
        self.counters.initialized_bytes_copied += other.counters.initialized_bytes_copied;
        self.counters.zeroed_bytes += other.counters.zeroed_bytes;
        #[cfg(test)]
        {
            self.counters.mapped_payload_requests += other.counters.mapped_payload_requests;
            self.counters.mapped_payload_bytes += other.counters.mapped_payload_bytes;
            self.counters.live_mapped_payload_bytes += other.counters.live_mapped_payload_bytes;
            other.counters.live_mapped_payload_bytes = 0;
        }
        self.counters.live_payload_bytes += other.counters.live_payload_bytes;
        self.counters.peak_payload_bytes = self
            .counters
            .peak_payload_bytes
            .max(self.counters.live_payload_bytes);
        other.first_owner = ptr::null_mut();
        other.owners.clear();
        other.free.clear();
        other.counters.live_payload_bytes = 0;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_parallel(
    context: *mut Heap,
    function: usize,
    input: *const u64,
    input_words: usize,
    output_words: usize,
    count: usize,
    output: *mut u64,
) -> NativeStatus {
    let invalid = NativeStatus {
        status: INVALID,
        origin: u64::MAX,
    };
    if context.is_null()
        || function == 0
        || input.is_null()
        || output.is_null()
        || count == 0
        || count > 8
        || input_words == 0
        || output_words == 0
    {
        return invalid;
    }
    let Some(input_length) = count.checked_mul(input_words) else {
        return invalid;
    };
    let Some(output_length) = count.checked_mul(output_words) else {
        return invalid;
    };
    if input_length > isize::MAX as usize / 8 || output_length > isize::MAX as usize / 8 {
        return invalid;
    }
    // SAFETY: compiler-owned typed entry and bounded disjoint stack packets.
    let call: unsafe extern "C" fn(*mut Heap, *const u64, *mut u64) -> NativeStatus =
        unsafe { std::mem::transmute(function) };
    let packets = unsafe { std::slice::from_raw_parts(input, input_length) };
    let output_address = output as usize;
    let mut failed = false;
    let mut jobs = std::thread::scope(|scope| {
        let mut threads = Vec::with_capacity(count);
        for ordinal in 0..count {
            let packet = &packets[ordinal * input_words..(ordinal + 1) * input_words];
            let destination = output_address + ordinal * output_words * 8;
            match std::thread::Builder::new().spawn_scoped(scope, move || {
                let mut heap = Heap::new();
                // SAFETY: each generated worker receives immutable captures,
                // one private heap and one nonoverlapping native output packet.
                let status = unsafe { call(&mut heap, packet.as_ptr(), destination as *mut u64) };
                Completed { heap, status }
            }) {
                Ok(thread) => threads.push(thread),
                Err(_) => {
                    failed = true;
                    break;
                }
            }
        }
        let mut completed = Vec::with_capacity(threads.len());
        for thread in threads {
            match thread.join() {
                Ok(job) => completed.push(job),
                Err(_) => failed = true,
            }
        }
        completed
    });
    // SAFETY: every spawned worker has joined. Only this thread uses context.
    let parent = unsafe { &mut *context };
    parent.parallel_batches += 1;
    parent.parallel_peak_workers = parent.parallel_peak_workers.max(jobs.len() as u64);
    let status = if failed {
        NativeStatus {
            status: ALLOCATION,
            origin: u64::MAX,
        }
    } else {
        jobs.iter()
            .find_map(|job| (job.status.status != 0).then_some(job.status))
            .unwrap_or(NativeStatus {
                status: 0,
                origin: 0,
            })
    };
    if status.status != 0 {
        for job in &mut jobs {
            job.heap.discard_private();
        }
        return status;
    }
    let upper = parent.counters.live_payload_bytes
        + jobs
            .iter()
            .map(|job| job.heap.counters.peak_payload_bytes)
            .sum::<u64>();
    parent.counters.parallel_peak_payload_upper_bound =
        parent.counters.parallel_peak_payload_upper_bound.max(upper);
    for job in &mut jobs {
        parent.adopt(&mut job.heap);
    }
    status
}

#[cfg(test)]
mod parallel_tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn refused_mapping_preserves_existing_storage_and_recovers() {
        let mut heap = Heap::new();
        let existing = heap.allocate(3, 8, 8, false).unwrap();
        deny_test_mapping(true);
        let denied = heap.allocate(32768, 8, 8, true);
        deny_test_mapping(false);
        assert_eq!(denied.unwrap_err(), ALLOCATION);
        assert_eq!(heap.counters.live_payload_bytes, 24);
        assert_eq!(heap.first_owner, existing[5] as *mut Owner);
        heap.allocate(32768, 8, 8, true).unwrap();
        heap.discard_private();
    }
    unsafe extern "C" fn copy_borrow(
        context: *mut Heap,
        input: *const u64,
        output: *mut u64,
    ) -> NativeStatus {
        // SAFETY: the test passes four initialized words per packet.
        let packet = unsafe { std::slice::from_raw_parts(input, 4) };
        let heap = unsafe { &mut *context };
        let mut view = heap.allocate(32768, 8, 8, true).unwrap();
        view[1] = 3;
        view[4] = 3;
        // The native writer initializes its own storage; the shared input is
        // an immutable borrow and never appears in a worker owner registry.
        for i in 0..3 {
            unsafe {
                (view[0] as *mut u64)
                    .add(i)
                    .write((packet[0] as *const u64).add(i).read() + packet[1]);
            }
        }
        unsafe {
            (*(view[5] as *mut Owner)).references = 1;
        }
        if packet[2] != 0 {
            return NativeStatus {
                status: packet[2],
                origin: packet[3],
            };
        }
        unsafe {
            ptr::copy_nonoverlapping(view.as_ptr(), output, 6);
        }
        NativeStatus {
            status: 0,
            origin: 0,
        }
    }
    #[test]
    fn joined_workers_transfer_owned_mappings_and_preserve_borrowed_inputs() {
        for count in [1, 2, 4, 8] {
            let mut parent = Heap::new();
            let existing = parent.allocate(3, 8, 8, false).unwrap();
            let borrowed = [11u64, 19, 31];
            let input = (0..count)
                .flat_map(|i| [borrowed.as_ptr() as u64, i as u64, 0, 0])
                .collect::<Vec<_>>();
            let mut output = vec![[0u64; 6]; count];
            let status = unsafe {
                mech_compact_parallel(
                    &mut parent,
                    copy_borrow as *const () as usize,
                    input.as_ptr(),
                    4,
                    6,
                    count,
                    output.as_mut_ptr().cast(),
                )
            };
            assert_eq!(status.status, 0);
            assert_eq!(parent.parallel_peak_workers, count as u64);
            assert_eq!(parent.parallel_batches, 1);
            assert_eq!(
                parent.counters.live_payload_bytes,
                24 + count as u64 * 32768 * 8
            );
            assert_eq!(parent.counters.payload_requests, 1 + count as u64);
            assert_eq!(parent.first_owner, existing[5] as *mut Owner);
            for (i, view) in output.iter().enumerate() {
                assert!(parent.owners.contains(&(view[5] as *mut Owner)));
                let values = unsafe { std::slice::from_raw_parts(view[0] as *const u64, 3) };
                assert_eq!(values, borrowed.map(|v| v + i as u64));
                assert_eq!(unsafe { (*(view[5] as *const Owner)).references }, 1);
            }
            assert_eq!(borrowed, [11, 19, 31]);
            #[cfg(target_os = "macos")]
            assert_eq!(
                parent.counters.live_mapped_payload_bytes,
                count as u64 * 32768 * 8
            );
            parent.discard_private();
            assert_eq!(parent.counters.live_payload_bytes, 0);
        }
    }
    #[test]
    fn failed_workers_report_first_source_ordinal_without_adopting_partial_outputs() {
        let mut parent = Heap::new();
        let existing = parent.allocate(3, 8, 8, false).unwrap();
        let borrowed = [11u64, 19, 31];
        let input = (0..4)
            .flat_map(|i| {
                [
                    borrowed.as_ptr() as u64,
                    i,
                    if i == 1 || i == 3 { 71 + i } else { 0 },
                    100 + i,
                ]
            })
            .collect::<Vec<_>>();
        let mut output = [[0u64; 6]; 4];
        let status = unsafe {
            mech_compact_parallel(
                &mut parent,
                copy_borrow as *const () as usize,
                input.as_ptr(),
                4,
                6,
                4,
                output.as_mut_ptr().cast(),
            )
        };
        assert_eq!(status.status, 72);
        assert_eq!(status.origin, 101);
        assert_eq!(parent.counters.live_payload_bytes, 24);
        assert_eq!(parent.first_owner, existing[5] as *mut Owner);
        assert!(parent.owners.iter().all(|p| p.is_null()));
        parent.discard_private();
    }
    #[test]
    fn invalid_parallel_packet_geometry_rejects_before_any_worker_runs() {
        let mut parent = Heap::new();
        let input = [0u64; 4];
        let mut output = [0u64; 6];
        for (input_words, output_words, count) in [
            (0, 6, 4),
            (4, 0, 4),
            (4, 6, 0),
            (4, 6, 9),
            (usize::MAX, 6, 4),
            (4, usize::MAX, 4),
        ] {
            let status = unsafe {
                mech_compact_parallel(
                    &mut parent,
                    copy_borrow as *const () as usize,
                    input.as_ptr(),
                    input_words,
                    output_words,
                    count,
                    output.as_mut_ptr(),
                )
            };
            assert_eq!(status.status, INVALID);
            assert_eq!(parent.parallel_batches, 0);
            assert_eq!(parent.counters.payload_requests, 0);
        }
    }
}

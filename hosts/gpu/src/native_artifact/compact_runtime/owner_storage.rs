//! Invocation-owned stable descriptors; payload initialization is unchanged.
use super::{MaybeUninit, Owner};
use std::ptr;

pub(super) const RECORDS: usize = match option_env!("MECH_INLINE_OWNER_RECORDS") {
    Some(count) => match count.as_bytes() {
        [b'1'] => 1,
        [b'8'] => 8,
        [b'1', b'2'] => 12,
        [b'1', b'6'] => 16,
        _ => panic!("inline owner record count must be 1, 8, 12 or 16"),
    },
    None => 1,
};

/// Borrowed by exactly one live Heap. Moving the Heap does not move records.
/// The first record keeps the original single-record storage representation.
#[repr(C)]
pub struct OwnerStorage {
    pub(super) records: [MaybeUninit<Owner>; RECORDS],
}

impl Default for OwnerStorage {
    fn default() -> Self {
        Self {
            records: [const { MaybeUninit::uninit() }; RECORDS],
        }
    }
}

impl OwnerStorage {
    /// The heap's existing vacant logical slot is also the arena index.
    /// Unchanged registration immediately consumes that same slot. Distinct
    /// live owner records never share a cell, including after heap moves.
    ///
    /// # Safety
    /// storage must be an exclusive, stable loan for this heap. slot must be
    /// vacant and immediately consumed by registration before another store.
    pub(super) unsafe fn store(
        storage: *mut Self,
        record: Owner,
        slot: usize,
    ) -> Result<*mut Owner, Owner> {
        if slot >= RECORDS {
            return Err(record);
        }
        // SAFETY: the caller retains the exclusive, stable storage loan.
        let first = unsafe { ptr::addr_of_mut!((*storage).records).cast::<Owner>() };
        // SAFETY: the bound above keeps this pointer within the array.
        let destination = unsafe { first.add(slot) };
        // SAFETY: the caller guarantees this cell has no live owner.
        unsafe { destination.write(record) };
        Ok(destination)
    }

    /// # Safety
    /// storage is the heap's exclusive loan. record is live, registered in that
    /// heap, and its payload has already been transferred or released.
    pub(super) unsafe fn retire(storage: *mut Self, record: *mut Owner) -> bool {
        // SAFETY: the caller retains the exclusive, stable storage loan.
        let first = unsafe { ptr::addr_of_mut!((*storage).records).cast::<Owner>() };
        let offset = (record as usize).wrapping_sub(first as usize);
        if offset % size_of::<Owner>() != 0 || offset / size_of::<Owner>() >= RECORDS {
            return false;
        }
        // The caller has validated the owner against the current registered
        // logical slot before retirement. Its cell remains part of the loan.
        unsafe { record.drop_in_place() };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_artifact::compact_runtime::Heap;

    #[test]
    fn descriptors_survive_heap_moves_arbitrary_release_and_fallback() {
        assert_eq!(std::mem::offset_of!(OwnerStorage, records), 0);
        assert_eq!(size_of::<OwnerStorage>(), RECORDS * size_of::<Owner>());
        let mut storage = OwnerStorage::default();
        for _ in 0..32 {
            let mut heap = Heap::with_owner_storage(&mut storage);
            let mut views = Vec::new();
            // More simultaneous live descriptors than the inline capacity.
            for index in 0..48 {
                let view = heap.allocate(4, 8, 8, true).unwrap();
                unsafe { (view[0] as *mut u64).write(index as u64 + 100) };
                views.push(view);
            }
            let mut moved = heap;
            for index in (0..48).step_by(2) {
                unsafe { moved.release(views[index][5] as *mut Owner).unwrap() };
            }
            for index in (1..48).step_by(2) {
                assert_eq!(
                    unsafe { (views[index][0] as *const u64).read() },
                    index as u64 + 100
                );
                let fresh = moved.allocate(4, 8, 8, true).unwrap();
                assert_eq!(unsafe { (fresh[0] as *const u64).read() }, 0);
            }
            moved.discard_private();
            assert_eq!(moved.counters.live_payload_bytes, 0);
        }
    }
}

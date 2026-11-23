//! Fixed size classes for the invocation's small numeric allocations.
use super::Owner;
use std::{
    alloc::{Layout, alloc, dealloc},
    mem::{align_of, size_of},
    ptr::NonNull,
};

const CLASSES: [(usize, u8); 4] = [(128, 8), (256, 4), (1536, 6), (2560, 4)];
const HEADER: usize = size_of::<Owner>();
const STARTS: [usize; CLASSES.len() + 1] = {
    let mut bases = [0; CLASSES.len() + 1];
    let mut at = 0;
    while at < CLASSES.len() {
        let (bytes, slots) = CLASSES[at];
        bases[at + 1] = bases[at] + bytes * slots as usize;
        at += 1;
    }
    bases
};
const STORAGE_BYTES: usize = STARTS[CLASSES.len()];

#[derive(Default)]
pub(super) struct SmallScratch {
    storage: Option<NonNull<u8>>,
    occupied: [u8; CLASSES.len()],
}

impl SmallScratch {
    fn layout() -> Layout {
        Layout::from_size_align(STORAGE_BYTES, align_of::<Owner>()).unwrap()
    }

    pub(super) fn acquire(&mut self, requested: Layout) -> Option<NonNull<Owner>> {
        if requested.align() > align_of::<Owner>() {
            return None;
        }
        let class = size_class(requested.size())?;
        let (stride, slots) = CLASSES[class];
        let available = (u8::MAX >> (u8::BITS - u32::from(slots))) & !self.occupied[class];
        if available == 0 {
            return None;
        }
        let storage = match self.storage {
            Some(storage) => storage,
            None => {
                // SAFETY: the fixed layout is nonempty. Every slot remains
                // inside the exclusively owned invocation backing.
                let storage = NonNull::new(unsafe { alloc(Self::layout()) })?;
                self.storage = Some(storage);
                storage
            }
        };
        let slot = available.trailing_zeros() as usize;
        self.occupied[class] |= 1 << slot;
        let offset = STARTS[class] + slot * stride;
        // SAFETY: the selected slot covers the whole header and payload, is
        // naturally aligned, and is now owned by this allocation only.
        Some(unsafe { storage.add(offset) }.cast())
    }

    pub(super) fn contains(&self, owner: *mut Owner) -> bool {
        self.storage.is_some_and(|storage| {
            owner.addr().wrapping_sub(storage.as_ptr().addr()) < STORAGE_BYTES
        })
    }

    /// The heap checked the registered owner and its exact layout.
    pub(super) fn release(&mut self, owner: *mut Owner, payload_bytes: usize) {
        let storage = self.storage.expect("live scratch has backing");
        let class = size_class(payload_bytes).expect("known scratch payload layout");
        let stride = CLASSES[class].0;
        let offset = owner.addr() - storage.as_ptr().addr() - STARTS[class];
        let slot = offset / stride;
        let mask = 1 << slot;
        debug_assert_eq!(offset % stride, 0);
        debug_assert_ne!(self.occupied[class] & mask, 0);
        self.occupied[class] &= !mask;
    }
}

fn size_class(payload: usize) -> Option<usize> {
    let size = HEADER.checked_add(payload)?;
    CLASSES.iter().position(|&(stride, _)| size <= stride)
}

impl Drop for SmallScratch {
    fn drop(&mut self) {
        debug_assert_eq!(self.occupied, [0; CLASSES.len()]);
        if let Some(storage) = self.storage {
            // SAFETY: the heap has retired each owner before this field drops.
            unsafe { dealloc(storage.as_ptr(), Self::layout()) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_slots_are_disjoint_and_return_only_after_release() {
        let mut scratch = SmallScratch::default();
        let mut occupied = Vec::new();
        for (bytes, number) in CLASSES {
            let payload = bytes - HEADER;
            let layout = Layout::from_size_align(payload, align_of::<Owner>()).unwrap();
            for _ in 0..number {
                let owner = scratch.acquire(layout).unwrap();
                let at = owner.as_ptr().addr();
                assert!(occupied.iter().all(|&(other, _, size)| at + bytes <= other || other + size <= at));
                occupied.push((at, payload, bytes));
            }
            assert!(scratch.acquire(layout).is_none());
        }
        for (address, payload, _) in occupied {
            let loan = std::ptr::without_provenance_mut::<Owner>(address);
            scratch.release(loan, payload);
            let layout = Layout::from_size_align(payload, align_of::<Owner>()).unwrap();
            let replacement = scratch.acquire(layout).unwrap();
            scratch.release(replacement.as_ptr(), payload);
        }
    }
}

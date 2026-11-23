//! The first owner remains in Heap; secondary slots are allocated on demand.
use super::{Owner, small_scratch::SmallScratch};
use std::ptr;

const INLINE: usize = 24;
const INLINE_MASK: u32 = (1 << INLINE) - 1;

#[derive(Default)]
pub(super) struct OwnerRegistry {
    inline: [*mut Owner; INLINE],
    live: u32,
    overflow: Vec<*mut Owner>,
    free: Vec<usize>,
    pub(super) scratch: SmallScratch,
}

impl OwnerRegistry {
    pub(super) fn get(&self, slot: usize) -> Option<*mut Owner> {
        let index = slot - 1;
        if index < INLINE {
            Some(self.inline[index])
        } else {
            self.overflow.get(index - INLINE).copied()
        }
    }

    pub(super) fn next_slot(&self) -> usize {
        let available = INLINE_MASK & !self.live;
        if available != 0 {
            available.trailing_zeros() as usize + 1
        } else {
            INLINE + 1 + self.free.last().copied().unwrap_or(self.overflow.len())
        }
    }

    pub(super) fn insert(&mut self, owner: *mut Owner) -> usize {
        let slot = self.next_slot();
        let index = slot - 1;
        if index < INLINE {
            self.inline[index] = owner;
            self.live |= 1 << index;
        } else if let Some(vacant) = self.free.pop() {
            self.overflow[vacant] = owner;
        } else {
            self.overflow.push(owner);
        }
        slot
    }

    pub(super) fn remove(&mut self, slot: usize) {
        let index = slot - 1;
        if index < INLINE {
            self.inline[index] = ptr::null_mut();
            self.live &= !(1 << index);
        } else {
            let index = index - INLINE;
            self.overflow[index] = ptr::null_mut();
            self.free.push(index);
        }
    }

    pub(super) fn slots(&self) -> usize {
        INLINE + self.overflow.len()
    }

    pub(super) fn owners(&self) -> impl Iterator<Item = *mut Owner> + '_ {
        self.inline.iter().chain(&self.overflow).copied()
    }
}

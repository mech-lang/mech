//! Persistent parser events and pending diagnostics. Publication shares a root;
//! subsequent writes copy only the path to an affected entry.
use crate::document::retained_sequence::{Measured, RetainedSequence, UnitMeasured};
use core::fmt;
#[derive(Clone)]
struct Entry<T>(T);
impl<T> Measured for Entry<T> {
    fn measure(&self) -> usize {
        1
    }
}
impl<T> UnitMeasured for Entry<T> {}
pub struct Journal<T> {
    items: RetainedSequence<Entry<T>>,
    pub(crate) allocations: u64,
    pub(crate) mutations: u64,
    pub(crate) discarded: u64,
    pub(crate) changed_from: usize,
}
impl<T> Clone for Journal<T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            allocations: self.allocations,
            mutations: self.mutations,
            discarded: self.discarded,
            changed_from: self.changed_from,
        }
    }
}
impl<T> Default for Journal<T> {
    fn default() -> Self {
        Self {
            items: RetainedSequence::default(),
            allocations: 0,
            mutations: 0,
            discarded: 0,
            changed_from: usize::MAX,
        }
    }
}
impl<T: fmt::Debug> fmt::Debug for Journal<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}
impl<T> Journal<T> {
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index).map(|entry| &entry.0)
    }
    pub(crate) fn get_with_work(&self, index: usize) -> (Option<&T>, u64) {
        let (entry, steps) = self.items.at_measure_with_work(index);
        (entry.map(|(_, entry)| &entry.0), steps)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter().map(|entry| &entry.0)
    }
    pub(crate) fn iter_range(&self, range: core::ops::Range<usize>) -> impl Iterator<Item = &T> {
        self.items
            .iter_from(range.start)
            .take(range.end.saturating_sub(range.start))
            .map(|entry| &entry.0)
    }
    pub(crate) fn push(&mut self, value: T) {
        self.changed_from = self.changed_from.min(self.len());
        self.allocations += self.items.push(Entry(value));
        self.mutations += 1;
    }
    pub(crate) fn truncate(&mut self, len: usize) {
        if len >= self.len() {
            return;
        }
        self.changed_from = self.changed_from.min(len);
        self.discarded += (self.len() - len) as u64;
        self.allocations += self.items.truncate(len);
        self.mutations += 1;
    }
    pub(crate) fn clear(&mut self) {
        self.truncate(0);
    }
}
impl<T: Clone> Journal<T> {
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len() {
            return None;
        }
        self.changed_from = self.changed_from.min(index);
        self.mutations += 1;
        self.items
            .get_mut(index, &mut self.allocations)
            .map(|entry| &mut entry.0)
    }
    pub(crate) fn last_mut(&mut self) -> Option<&mut T> {
        self.len()
            .checked_sub(1)
            .and_then(|index| self.get_mut(index))
    }
    pub(crate) fn pop(&mut self) -> Option<T> {
        let index = self.len().checked_sub(1)?;
        let value = self.get(index)?.clone();
        self.truncate(index);
        Some(value)
    }
}
impl<T> core::ops::Index<usize> for Journal<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        self.get(index).expect("journal index")
    }
}
impl<T: Clone> core::ops::IndexMut<usize> for Journal<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index).expect("journal mutable index")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unshared_append_and_rewind_allocate_only_the_retained_tree() {
        for count in [64, 256, 1024] {
            let mut journal = Journal::default();
            for value in 0..count {
                journal.push(value);
            }
            assert!(journal.allocations <= 2 * count as u64);
            let append_allocations = journal.allocations;
            for len in (0..count).rev() {
                journal.truncate(len);
                assert_eq!(journal.len(), len);
                assert_eq!(
                    journal.iter().copied().collect::<alloc::vec::Vec<_>>(),
                    (0..len).collect::<alloc::vec::Vec<_>>()
                );
            }
            assert_eq!(
                journal.allocations, append_allocations,
                "unshared rewind must not copy a path"
            );
        }
    }

    #[test]
    fn shared_snapshots_preserve_values_across_rewind_append_and_marker_edits() {
        let mut journal = Journal::default();
        for value in 0..128 {
            journal.push(value);
        }
        let frozen = journal.clone();
        journal.truncate(65);
        journal[64] = 1000;
        for value in 128..192 {
            journal.push(value);
        }
        assert_eq!(
            frozen.iter().copied().collect::<alloc::vec::Vec<_>>(),
            (0..128).collect::<alloc::vec::Vec<_>>()
        );
        assert_eq!(journal[63], 63);
        assert_eq!(journal[64], 1000);
        assert_eq!(journal[65], 128);
        assert_eq!(journal[128], 191);
        assert_eq!(frozen.len(), 128);
        assert_eq!(journal.len(), 129);
    }
}

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
        let (items, allocations) = self.items.appended(Entry(value));
        self.items = items;
        self.allocations += allocations;
        self.mutations += 1;
    }
    pub(crate) fn truncate(&mut self, len: usize) {
        if len >= self.len() {
            return;
        }
        self.changed_from = self.changed_from.min(len);
        self.discarded += (self.len() - len) as u64;
        let (items, allocations) = self.items.truncated(len);
        self.items = items;
        self.allocations += allocations;
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

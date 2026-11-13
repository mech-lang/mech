//! Persistent parser events and diagnostics, grouped into bounded storage pages.
use crate::document::retained_sequence::{Measured, RetainedSequence, UnitMeasured};
use alloc::sync::Arc;
use core::fmt;
use smallvec::SmallVec;
const PAGE: usize = 4;
#[derive(Clone)]
struct Chunk<T>(Arc<SmallVec<[T; PAGE]>>);
impl<T> Measured for Chunk<T> {
    fn measure(&self) -> usize {
        1
    }
}
impl<T> UnitMeasured for Chunk<T> {}
pub struct Journal<T> {
    items: RetainedSequence<Chunk<T>>,
    len: usize,
    pub(crate) allocations: u64,
    pub(crate) mutations: u64,
    pub(crate) discarded: u64,
    pub(crate) changed_from: usize,
}
impl<T> Clone for Journal<T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            len: self.len,
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
            len: 0,
            allocations: 0,
            mutations: 0,
            discarded: 0,
            changed_from: usize::MAX,
        }
    }
}
impl<T: fmt::Debug> fmt::Debug for Journal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
impl<T> Journal<T> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        self.items.get(index / PAGE)?.0.get(index % PAGE)
    }
    pub(crate) fn get_with_work(&self, index: usize) -> (Option<&T>, u64) {
        let (chunk, work) = self.items.at_measure_with_work(index / PAGE);
        let value = chunk.and_then(|(_, chunk)| chunk.0.get(index % PAGE));
        (value, work + 1)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter().flat_map(|chunk| chunk.0.iter())
    }
    pub(crate) fn iter_range(&self, range: core::ops::Range<usize>) -> impl Iterator<Item = &T> {
        self.items
            .iter_from(range.start / PAGE)
            .flat_map(|chunk| chunk.0.iter())
            .skip(range.start % PAGE)
            .take(range.end.saturating_sub(range.start))
    }
}
impl<T: Clone> Journal<T> {
    fn chunk_mut(&mut self, index: usize) -> Option<&mut SmallVec<[T; PAGE]>> {
        let chunk = self.items.get_mut(index, &mut self.allocations)?;
        if Arc::strong_count(&chunk.0) > 1 {
            self.allocations += 1;
        }
        Some(Arc::make_mut(&mut chunk.0))
    }
    pub(crate) fn push(&mut self, value: T) {
        self.changed_from = self.changed_from.min(self.len);
        if self.len % PAGE == 0 {
            let mut values = SmallVec::new();
            values.push(value);
            self.allocations += 1;
            self.allocations += self.items.push(Chunk(Arc::new(values)));
        } else {
            self.chunk_mut(self.len / PAGE)
                .expect("last journal page")
                .push(value);
        }
        self.len += 1;
        self.mutations += 1;
    }
    pub(crate) fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        self.changed_from = self.changed_from.min(len);
        self.discarded += (self.len - len) as u64;
        self.allocations += self.items.truncate(len.div_ceil(PAGE));
        if len % PAGE != 0 {
            self.chunk_mut(len / PAGE)
                .expect("retained journal page")
                .truncate(len % PAGE);
        }
        self.len = len;
        self.mutations += 1;
    }
    pub(crate) fn clear(&mut self) {
        self.truncate(0);
    }
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        self.changed_from = self.changed_from.min(index);
        self.mutations += 1;
        self.chunk_mut(index / PAGE)?.get_mut(index % PAGE)
    }
    pub(crate) fn last_mut(&mut self) -> Option<&mut T> {
        self.len
            .checked_sub(1)
            .and_then(|index| self.get_mut(index))
    }
    pub(crate) fn pop(&mut self) -> Option<T> {
        let index = self.len.checked_sub(1)?;
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
    use alloc::vec::Vec;

    fn same(journal: &Journal<usize>, values: &[usize]) {
        assert_eq!(journal.len(), values.len());
        assert_eq!(journal.iter().copied().collect::<Vec<_>>(), values);
        for index in 0..=values.len() {
            assert_eq!(journal.get(index), values.get(index));
            assert_eq!(journal.get_with_work(index).0, values.get(index));
            assert_eq!(
                journal
                    .iter_range(index..values.len())
                    .copied()
                    .collect::<Vec<_>>(),
                &values[index..]
            );
        }
    }

    #[test]
    fn retained_pages_survive_mutations_and_rewinds_at_every_page_boundary() {
        let mut journal = Journal::default();
        let mut values = Vec::new();
        let mut retained = Vec::new();
        for value in 0..128 {
            journal.push(value);
            values.push(value);
            retained.push((journal.clone(), values.clone()));
        }
        for len in (0..128).rev() {
            journal.truncate(len);
            values.truncate(len);
            if len > 0 {
                *journal.get_mut(len - 1).unwrap() += 1000;
                values[len - 1] += 1000;
            }
            journal.push(9000);
            values.push(9000);
            same(&journal, &values);
            journal.pop();
            values.pop();
            for (old, expected) in &retained {
                assert_eq!(old.iter().copied().collect::<Vec<_>>(), *expected);
            }
        }
        same(&journal, &values);
    }

    #[test]
    fn unpublished_journal_pages_amortize_allocations_without_copying_the_prefix() {
        let mut journal = Journal::default();
        for value in 0..1024 {
            journal.push(value);
        }
        assert!(journal.allocations < journal.len() as u64);
        let allocations = journal.allocations;
        for len in (0..1024).rev() {
            journal.truncate(len);
        }
        assert_eq!(journal.allocations, allocations);
        assert!(journal.is_empty());
    }
}

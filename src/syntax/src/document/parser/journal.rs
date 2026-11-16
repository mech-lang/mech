//! Persistent parser events and diagnostics, grouped into bounded storage pages.
use crate::document::retained_sequence::{Measured, RetainedSequence, UnitMeasured};
use alloc::sync::Arc;
use core::fmt;
use smallvec::SmallVec;
const PAGE: usize = 4;
struct Chunk<T>(Arc<SmallVec<[T; PAGE]>>);
impl<T> Clone for Chunk<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Measured for Chunk<T> {
    fn measure(&self) -> usize {
        1
    }
}
impl<T> UnitMeasured for Chunk<T> {}
pub struct Journal<T> {
    // Completed pages retain the persistent snapshot structure. Keep the
    // current page separate so ordinary appends and small rewinds do not
    // repeatedly walk and copy a path through that structure.
    items: RetainedSequence<Chunk<T>>,
    tail: Option<Chunk<T>>,
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
            tail: self.tail.clone(),
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
            tail: None,
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
        let page = index / PAGE;
        let chunk = if page == self.items.len() {
            self.tail.as_ref()?
        } else {
            self.items.get(page)?
        };
        chunk.0.get(index % PAGE)
    }
    pub(crate) fn get_with_work(&self, index: usize) -> (Option<&T>, u64) {
        if index / PAGE == self.items.len() {
            return (
                self.tail
                    .as_ref()
                    .and_then(|chunk| chunk.0.get(index % PAGE)),
                1,
            );
        }
        let (chunk, work) = self.items.at_measure_with_work(index / PAGE);
        let value = chunk.and_then(|(_, chunk)| chunk.0.get(index % PAGE));
        (value, work + 1)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items
            .iter()
            .chain(self.tail.iter())
            .flat_map(|chunk| chunk.0.iter())
    }
    pub(crate) fn iter_range(&self, range: core::ops::Range<usize>) -> impl Iterator<Item = &T> {
        self.items
            .iter_from(range.start / PAGE)
            .chain(
                self.tail
                    .iter()
                    .filter(move |_| range.start / PAGE <= self.items.len()),
            )
            .flat_map(|chunk| chunk.0.iter())
            .skip(range.start % PAGE)
            .take(range.end.saturating_sub(range.start))
    }
}
impl<T: Clone> Journal<T> {
    fn chunk_mut(&mut self, index: usize) -> Option<&mut SmallVec<[T; PAGE]>> {
        let chunk = if index == self.items.len() {
            self.tail.as_mut()?
        } else {
            self.items.get_mut(index, &mut self.allocations)?
        };
        if Arc::strong_count(&chunk.0) > 1 {
            self.allocations += 1;
        }
        Some(Arc::make_mut(&mut chunk.0))
    }
    pub(crate) fn push(&mut self, value: T) {
        self.changed_from = self.changed_from.min(self.len);
        if self.len % PAGE == 0 {
            if let Some(full) = self.tail.take() {
                self.allocations += self.items.push(full);
            }
            let mut values = SmallVec::new();
            values.push(value);
            self.allocations += 1;
            self.tail = Some(Chunk(Arc::new(values)));
        } else {
            self.chunk_mut(self.len / PAGE)
                .expect("last journal page")
                .push(value);
        }
        self.len += 1;
        self.mutations += 1;
    }
    /// Append complete pages together. Retained snapshots still own their old
    /// chunks; the current last page needs at most one mutable lookup per fill.
    pub(crate) fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        let mut values = values.into_iter();
        loop {
            let remaining = PAGE - self.len % PAGE;
            let mut batch = SmallVec::<[T; PAGE]>::new();
            for _ in 0..remaining {
                let Some(value) = values.next() else {
                    break;
                };
                batch.push(value);
            }
            if batch.is_empty() {
                return;
            }
            let count = batch.len();
            self.changed_from = self.changed_from.min(self.len);
            if self.len % PAGE == 0 {
                if let Some(full) = self.tail.take() {
                    self.allocations += self.items.push(full);
                }
                self.allocations += 1;
                self.tail = Some(Chunk(Arc::new(batch)));
            } else {
                self.chunk_mut(self.len / PAGE)
                    .expect("last journal page")
                    .extend(batch);
            }
            self.len += count;
            self.mutations += count as u64;
        }
    }

    pub(crate) fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        self.changed_from = self.changed_from.min(len);
        self.discarded += (self.len - len) as u64;
        if len == 0 {
            self.tail = None;
            self.allocations += self.items.truncate(0);
        } else {
            let page = (len - 1) / PAGE;
            if page < self.items.len() {
                // Move the retained final page out of the completed prefix.
                // Older snapshots still own their original page and tree.
                self.tail = self.items.get(page).cloned();
                self.allocations += self.items.truncate(page);
            }
            let retained = (len - 1) % PAGE + 1;
            if self.tail.as_ref().expect("retained journal page").0.len() != retained {
                self.chunk_mut(page)
                    .expect("retained journal page")
                    .truncate(retained);
            }
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
    fn batched_appends_preserve_retained_snapshots_and_partial_pages() {
        for prefix in 0..=20 {
            for count in 0..=20 {
                let mut journal = Journal::default();
                let mut expected = Vec::new();
                for value in 0..prefix {
                    journal.push(value);
                    expected.push(value);
                }
                let retained = journal.clone();
                let before = journal.mutations;
                journal.extend(100..100 + count);
                expected.extend(100..100 + count);
                same(&journal, &expected);
                same(&retained, &(0..prefix).collect::<Vec<_>>());
                assert_eq!(journal.mutations - before, count as u64);
                for len in (0..=expected.len()).rev() {
                    let saved = journal.clone();
                    let saved_expected = expected.clone();
                    journal.truncate(len);
                    expected.truncate(len);
                    journal.extend([200, 201, 202, 203, 204]);
                    expected.extend([200, 201, 202, 203, 204]);
                    same(&journal, &expected);
                    same(&saved, &saved_expected);
                }
            }
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
    fn retained_snapshots_survive_nonlocal_writes_after_tail_rotation() {
        let mut journal = Journal::default();
        let mut expected = (0..256).collect::<Vec<_>>();
        journal.extend(expected.iter().copied());
        let original = journal.clone();
        for index in [0, 3, 4, 17, 127, 252, 255] {
            journal[index] += 1000;
            expected[index] += 1000;
        }
        let changed = journal.clone();
        let changed_values = expected.clone();
        journal.extend(256..300);
        expected.extend(256..300);
        same(&journal, &expected);
        same(&original, &(0..256).collect::<Vec<_>>());
        same(&changed, &changed_values);

        // A full retained page becomes the new tail, then rotates back into
        // the prefix. All three saved views must retain their own contents.
        let extended = journal.clone();
        let extended_values = expected.clone();
        journal.truncate(PAGE);
        expected.truncate(PAGE);
        journal.extend(500..509);
        expected.extend(500..509);
        journal[1] = 77;
        expected[1] = 77;
        same(&journal, &expected);
        same(&changed, &changed_values);
        same(&extended, &extended_values);
        journal.clear();
        journal.extend([4, 8, 15, 16, 23, 42]);
        same(&journal, &[4, 8, 15, 16, 23, 42]);
        same(&original, &(0..256).collect::<Vec<_>>());
        same(&changed, &changed_values);
        same(&extended, &extended_values);
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

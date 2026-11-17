//! Parser events and diagnostics, grouped into bounded storage pages.
mod batch;

use crate::document::retained_sequence::{Measured, RetainedSequence, UnitMeasured};
use alloc::sync::Arc;
use core::fmt;
use smallvec::SmallVec;
// Larger bounded pages amortize persistent tree traversal and allocation.
const PAGE: usize = 32;
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

enum CompletedPages<T> {
    Retained(RetainedSequence<Chunk<T>>),
    Batch(batch::BatchPages<T>),
}

impl<T> Clone for CompletedPages<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Retained(pages) => Self::Retained(pages.clone()),
            Self::Batch(pages) => Self::Batch(pages.clone()),
        }
    }
}

impl<T> CompletedPages<T> {
    #[inline]
    fn get(&self, index: usize) -> Option<&Chunk<T>> {
        match self {
            Self::Retained(pages) => pages.get(index),
            Self::Batch(pages) => pages.get(index),
        }
    }

    #[inline]
    fn get_mut(&mut self, index: usize, allocations: &mut u64) -> Option<&mut Chunk<T>> {
        match self {
            Self::Retained(pages) => pages.get_mut(index, allocations),
            Self::Batch(pages) => pages.get_mut(index, allocations),
        }
    }

    fn push(&mut self, page: Chunk<T>, allocations: &mut u64) {
        match self {
            Self::Retained(pages) => *allocations += pages.push(page),
            Self::Batch(pages) => pages.push(page, allocations),
        }
    }

    fn truncate(&mut self, len: usize, allocations: &mut u64) {
        match self {
            Self::Retained(pages) => *allocations += pages.truncate(len),
            Self::Batch(pages) => pages.truncate(len, allocations),
        }
    }

    fn iter_from(&self, index: usize) -> impl Iterator<Item = &Chunk<T>> {
        let retained = match self {
            Self::Retained(pages) => Some(pages.iter_from(index)),
            Self::Batch(_) => None,
        };
        let batch = match self {
            Self::Retained(_) => None,
            Self::Batch(pages) => Some(pages.iter_from(index)),
        };
        retained
            .into_iter()
            .flatten()
            .chain(batch.into_iter().flatten())
    }
}

pub struct Journal<T> {
    // Completed pages retain the persistent snapshot structure. Keep the
    // current page separate so ordinary appends and small rewinds do not
    // repeatedly walk and copy a path through that structure.
    items: CompletedPages<T>,
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
            items: CompletedPages::Retained(RetainedSequence::default()),
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
    pub(super) fn batch() -> Self {
        Self {
            items: CompletedPages::Batch(batch::BatchPages::new()),
            ..Self::default()
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn completed_pages(&self) -> usize {
        self.len.saturating_sub(1) / PAGE
    }

    #[inline]
    fn chunk(&self, page: usize) -> Option<&Chunk<T>> {
        if page == self.completed_pages() {
            self.tail.as_ref()
        } else {
            self.items.get(page)
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        self.chunk(index / PAGE)?.0.get(index % PAGE)
    }
    /// Borrow a page containing this item, with its absolute starting index.
    pub(crate) fn page_at(&self, index: usize) -> Option<(usize, &[T])> {
        if index >= self.len {
            return None;
        }
        let page = index / PAGE;
        Some((page * PAGE, self.chunk(page)?.0.as_slice()))
    }
    pub(crate) fn get_with_work(&self, index: usize) -> (Option<&T>, u64) {
        let page = index / PAGE;
        if page == self.completed_pages() {
            return (
                self.tail
                    .as_ref()
                    .and_then(|chunk| chunk.0.get(index % PAGE)),
                1,
            );
        }
        let (chunk, work) = match &self.items {
            CompletedPages::Retained(pages) => {
                let (chunk, work) = pages.at_measure_with_work(page);
                (chunk.map(|(_, chunk)| chunk), work)
            }
            CompletedPages::Batch(pages) => pages.get_with_work(page),
        };
        let value = chunk.and_then(|chunk| chunk.0.get(index % PAGE));
        (value, work + 1)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.iter_range(0..self.len)
    }
    pub(crate) fn iter_range(&self, range: core::ops::Range<usize>) -> impl Iterator<Item = &T> {
        self.items
            .iter_from(range.start / PAGE)
            .chain(
                self.tail
                    .iter()
                    .filter(move |_| range.start / PAGE <= self.completed_pages()),
            )
            .flat_map(|chunk| chunk.0.iter())
            .skip(range.start % PAGE)
            .take(range.end.saturating_sub(range.start))
    }
}
impl<T: Clone> Journal<T> {
    fn chunk_mut(&mut self, index: usize) -> Option<&mut SmallVec<[T; PAGE]>> {
        let chunk = if index == self.completed_pages() {
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
                self.items.push(full, &mut self.allocations);
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
                    self.items.push(full, &mut self.allocations);
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
        let completed_pages = self.completed_pages();
        self.len = len;
        if len == 0 {
            self.tail = None;
            self.items.truncate(0, &mut self.allocations);
        } else {
            let page = (len - 1) / PAGE;
            if page < completed_pages {
                // Move the retained final page out of the completed prefix.
                // Older snapshots still own their original page and tree.
                self.tail = self.items.get(page).cloned();
                self.items.truncate(page, &mut self.allocations);
            }
            let retained = (len - 1) % PAGE + 1;
            if self.tail.as_ref().expect("retained journal page").0.len() != retained {
                self.chunk_mut(page)
                    .expect("retained journal page")
                    .truncate(retained);
            }
        }
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

    #[test]
    fn batch_page_groups_survive_mutation_and_rewind() {
        let mut journal = Journal::batch();
        let group_edge = 32 * PAGE;
        let count = 3 * group_edge + 5;
        journal.extend(0..count);
        let original = journal.clone();
        let start = group_edge - 1;

        journal[start] = usize::MAX;
        journal.truncate(group_edge + 1);
        journal.extend(0..2 * group_edge);
        assert_eq!(journal.get(start), Some(&usize::MAX));
        assert_eq!(journal.get(group_edge), Some(&group_edge));
        assert!(
            journal
                .iter_range(group_edge + 1..journal.len())
                .copied()
                .eq(0..2 * group_edge)
        );
        assert!(original.iter().copied().eq(0..count));
    }

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

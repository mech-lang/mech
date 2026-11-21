//! Persistent parser journal. Completed pages share an immutable prefix;
//! the current page stays directly accessible for append and rewind.
mod batch;

use crate::document::retained_sequence::{Measured, RetainedSequence, UnitMeasured};
use alloc::{sync::Arc, vec::Vec};
use core::fmt;
// Larger bounded pages amortize persistent tree traversal and allocation.
const PAGE: usize = 32;
struct Values<T>(Vec<T>);
impl<T: Clone> Clone for Values<T> {
    fn clone(&self) -> Self {
        let mut values = Vec::with_capacity(PAGE);
        values.extend_from_slice(&self.0);
        Self(values)
    }
}
struct Page<T>(Arc<Values<T>>);
impl<T> Clone for Page<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Measured for Page<T> {
    fn measure(&self) -> usize {
        1
    }
}
impl<T> UnitMeasured for Page<T> {}
impl<T: Clone> Page<T> {
    fn values_mut(&mut self, allocations: &mut u64) -> &mut Vec<T> {
        if Arc::strong_count(&self.0) > 1 {
            *allocations += 2;
        }
        &mut Arc::make_mut(&mut self.0).0
    }
}
enum CompletedPages<T> {
    Retained(RetainedSequence<Page<T>>),
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
    fn get(&self, index: usize) -> Option<&Page<T>> {
        match self {
            Self::Retained(pages) => pages.get(index),
            Self::Batch(pages) => pages.get(index),
        }
    }

    #[inline]
    fn get_mut(&mut self, index: usize, allocations: &mut u64) -> Option<&mut Page<T>> {
        match self {
            Self::Retained(pages) => pages.get_mut(index, allocations),
            Self::Batch(pages) => pages.get_mut(index, allocations),
        }
    }

    fn push(&mut self, page: Page<T>, allocations: &mut u64) {
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

    fn get_with_work(&self, index: usize) -> (Option<&Page<T>>, u64) {
        match self {
            Self::Retained(pages) => {
                let (page, work) = pages.at_measure_with_work(index);
                (page.map(|(_, page)| page), work)
            }
            Self::Batch(pages) => pages.get_with_work(index),
        }
    }

    fn iter_from(&self, index: usize) -> impl Iterator<Item = &Page<T>> {
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
    flat: Option<Arc<Vec<T>>>,
    items: CompletedPages<T>,
    tail: Option<Page<T>>,
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
            flat: self.flat.clone(),
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
            flat: None,
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
    // A sealed parse publishes only its final tree; it needs no persistent edit paths.
    pub(crate) fn transient() -> Self {
        Self {
            flat: Some(Arc::new(Vec::new())),
            allocations: 1,
            ..Self::default()
        }
    }
    pub(super) fn batch() -> Self {
        Self {
            items: CompletedPages::Batch(batch::BatchPages::new()),
            ..Self::default()
        }
    }
    fn completed_pages(&self) -> usize {
        self.len.saturating_sub(1) / PAGE
    }
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
        if let Some(values) = &self.flat {
            return values.get(index);
        }
        let page = if index / PAGE < self.len.saturating_sub(1) / PAGE {
            self.items.get(index / PAGE)?
        } else {
            self.tail.as_ref()?
        };
        page.0.0.get(index % PAGE)
    }
    /// Borrow a page containing this item, with its absolute starting index.
    pub(crate) fn page_at(&self, index: usize) -> Option<(usize, &[T])> {
        if index >= self.len {
            return None;
        }
        if let Some(values) = &self.flat {
            return Some((0, values.as_slice()));
        }
        let index = index / PAGE;
        let page = if index == self.completed_pages() {
            self.tail.as_ref()?
        } else {
            self.items.get(index)?
        };
        Some((index * PAGE, page.0.0.as_slice()))
    }
    pub(crate) fn get_with_work(&self, index: usize) -> (Option<&T>, u64) {
        if index >= self.len {
            return (None, 1);
        }
        if let Some(values) = &self.flat {
            return (values.get(index), 1);
        }
        if index / PAGE >= self.completed_pages() {
            return (
                self.tail
                    .as_ref()
                    .and_then(|page| page.0.0.get(index % PAGE)),
                1,
            );
        }
        let (page, steps) = self.items.get_with_work(index / PAGE);
        (page.and_then(|page| page.0.0.get(index % PAGE)), steps + 1)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.flat.iter().flat_map(|values| values.iter()).chain(
            self.items
                .iter_from(0)
                .chain(self.tail.iter())
                .flat_map(|page| page.0.0.iter()),
        )
    }
    pub(crate) fn iter_range(&self, range: core::ops::Range<usize>) -> impl Iterator<Item = &T> {
        let count = range.end.min(self.len).saturating_sub(range.start);
        self.flat
            .iter()
            .flat_map(|values| values.iter())
            .skip(range.start)
            .take(count)
            .chain(
                self.items
                    .iter_from(range.start / PAGE)
                    .chain(
                        self.tail
                            .iter()
                            .filter(move |_| range.start / PAGE <= self.completed_pages()),
                    )
                    .flat_map(|page| page.0.0.iter())
                    .skip(range.start % PAGE)
                    .take(count),
            )
    }
}
fn flat_mut<'a, T: Clone>(values: &'a mut Arc<Vec<T>>, allocations: &mut u64) -> &'a mut Vec<T> {
    if Arc::strong_count(values) > 1 {
        *allocations += 1 + u64::from(!values.is_empty());
    }
    Arc::make_mut(values)
}
impl<T: Clone> Journal<T> {
    /// Append through the directly accessible tail; retained snapshots keep their pages.
    pub(crate) fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        for value in values {
            self.push(value);
        }
    }
    pub(crate) fn push(&mut self, value: T) {
        self.changed_from = self.changed_from.min(self.len);
        self.mutations += 1;
        if let Some(values) = &mut self.flat {
            let values = flat_mut(values, &mut self.allocations);
            self.allocations += u64::from(values.len() == values.capacity());
            values.push(value);
            self.len += 1;
            return;
        }
        if self
            .tail
            .as_ref()
            .is_some_and(|page| page.0.0.len() == PAGE)
        {
            self.items
                .push(self.tail.take().unwrap(), &mut self.allocations);
        }
        if self.tail.is_none() {
            self.tail = Some(Page(Arc::new(Values(Vec::with_capacity(PAGE)))));
            self.allocations += 2;
        }
        self.tail
            .as_mut()
            .unwrap()
            .values_mut(&mut self.allocations)
            .push(value);
        self.len += 1;
    }
    pub(crate) fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        self.changed_from = self.changed_from.min(len);
        self.discarded += (self.len - len) as u64;
        if let Some(values) = &mut self.flat {
            flat_mut(values, &mut self.allocations).truncate(len);
        } else if len == 0 {
            self.items.truncate(0, &mut self.allocations);
            if let Some(values) = self
                .tail
                .as_mut()
                .and_then(|page| Arc::get_mut(&mut page.0))
            {
                values.0.clear();
            } else {
                self.tail = None;
            }
        } else {
            if (len - 1) / PAGE < self.completed_pages() {
                let page = (len - 1) / PAGE;
                let tail = self.items.get(page).unwrap().clone();
                self.items.truncate(page, &mut self.allocations);
                self.tail = Some(tail);
            }
            let tail_len = (len - 1) % PAGE + 1;
            self.tail
                .as_mut()
                .unwrap()
                .values_mut(&mut self.allocations)
                .truncate(tail_len);
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
        if let Some(values) = &mut self.flat {
            return flat_mut(values, &mut self.allocations).get_mut(index);
        }
        let page = if index / PAGE < self.len.saturating_sub(1) / PAGE {
            self.items.get_mut(index / PAGE, &mut self.allocations)?
        } else {
            self.tail.as_mut()?
        };
        page.values_mut(&mut self.allocations).get_mut(index % PAGE)
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

    #[test]
    fn mutations_and_suffix_views_match_a_vector_across_shared_page_boundaries() {
        for mut journal in [Journal::default(), Journal::transient(), Journal::batch()] {
            let mut expected = (0..1024).collect::<Vec<_>>();
            for value in &expected {
                journal.push(*value);
            }
            let mut frozen = Vec::new();
            let mut random = 829_u64;
            for value in 1024..4096 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                let index = (random >> 32) as usize;
                match index % 10 {
                    0 if !expected.is_empty() => {
                        let len = index % expected.len();
                        journal.truncate(len);
                        expected.truncate(len);
                    }
                    1 if !expected.is_empty() => {
                        let at = index % expected.len();
                        journal[at] = value;
                        expected[at] = value;
                    }
                    2 => assert_eq!(journal.pop(), expected.pop()),
                    _ => {
                        journal.push(value);
                        expected.push(value);
                    }
                }
                if value % 47 == 0 {
                    frozen.push((journal.clone(), expected.clone()));
                    if frozen.len() > 8 {
                        frozen.remove(0);
                    }
                }
                assert_eq!(journal.len(), expected.len());
                let start = index % (expected.len() + 17);
                let end = start + index % 49;
                let suffix = journal.iter_range(start..end).copied().collect::<Vec<_>>();
                assert_eq!(
                    suffix,
                    expected
                        .iter()
                        .copied()
                        .skip(start)
                        .take(end - start)
                        .collect::<Vec<_>>()
                );
                for (snapshot, values) in &frozen {
                    assert_eq!(snapshot.iter().copied().collect::<Vec<_>>(), *values);
                }
            }
        }
    }

    #[test]
    fn a_shared_edit_copies_a_bounded_page_independently_of_journal_length() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicUsize, Ordering};

        struct Counted(usize, Arc<AtomicUsize>);
        impl Clone for Counted {
            fn clone(&self) -> Self {
                self.1.fetch_add(1, Ordering::Relaxed);
                Self(self.0, self.1.clone())
            }
        }
        let mut prior_copies = None;
        for len in [1024, 4096, 16384] {
            let copies = Arc::new(AtomicUsize::new(0));
            let mut journal = Journal::default();
            for index in 0..len {
                journal.push(Counted(index, copies.clone()));
            }
            let frozen = journal.clone();
            journal[len / 2].0 = usize::MAX;
            let observed = copies.load(Ordering::Relaxed);
            assert!(observed <= 64, "a shared edit copied {observed} entries");
            if let Some(prior) = prior_copies {
                assert_eq!(observed, prior, "copying grew with the journal");
            }
            prior_copies = Some(observed);
            assert_eq!(frozen[len / 2].0, len / 2);
            assert_eq!(journal[len / 2].0, usize::MAX);
            assert_eq!(frozen.iter().count(), len);
        }
    }

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
        for mut journal in [Journal::default(), Journal::transient(), Journal::batch()] {
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
}

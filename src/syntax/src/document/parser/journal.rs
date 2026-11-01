//! Persistent parser journal. Completed pages share an immutable prefix;
//! the current page stays directly accessible for append and rewind.
use crate::document::retained_sequence::{Measured, RetainedSequence, UnitMeasured};
use alloc::{sync::Arc, vec::Vec};
use core::fmt;
const PAGE: usize = 16;
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
pub struct Journal<T> {
    flat: Option<Arc<Vec<T>>>,
    items: RetainedSequence<Page<T>>,
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
            items: RetainedSequence::default(),
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
        let page = if index / PAGE < self.items.len() {
            self.items.get(index / PAGE)?
        } else {
            self.tail.as_ref()?
        };
        page.0.0.get(index % PAGE)
    }
    pub(crate) fn get_with_work(&self, index: usize) -> (Option<&T>, u64) {
        if index >= self.len {
            return (None, 1);
        }
        if let Some(values) = &self.flat {
            return (values.get(index), 1);
        }
        if index / PAGE >= self.items.len() {
            return (
                self.tail
                    .as_ref()
                    .and_then(|page| page.0.0.get(index % PAGE)),
                1,
            );
        }
        let (page, steps) = self.items.at_measure_with_work(index / PAGE);
        (
            page.and_then(|(_, page)| page.0.0.get(index % PAGE)),
            steps + 1,
        )
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.flat.iter().flat_map(|values| values.iter()).chain(
            self.items
                .iter()
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
                    .chain(self.tail.iter())
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
            self.allocations += self.items.push(self.tail.take().unwrap());
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
            self.allocations += self.items.truncate(0);
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
            if len < self.items.len() * PAGE {
                let page = (len - 1) / PAGE;
                let tail = self.items.get(page).unwrap().clone();
                self.allocations += self.items.truncate(page);
                self.tail = Some(tail);
            }
            let tail_len = len - self.items.len() * PAGE;
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
        let page = if index / PAGE < self.items.len() {
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
    fn mutations_and_suffix_views_match_a_vector_across_shared_page_boundaries() {
        for mut journal in [Journal::default(), Journal::transient()] {
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
        for mut journal in [Journal::default(), Journal::transient()] {
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
}

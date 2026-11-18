//! The one-shot parser groups complete event pages for direct local lookup.
use super::Chunk;
use crate::document::retained_sequence::{Measured, RetainedSequence, UnitMeasured};
use alloc::sync::Arc;
use smallvec::{SmallVec, smallvec};

const GROUP: usize = 32;
type Pages<T> = SmallVec<[Chunk<T>; GROUP]>;

struct PageGroup<T>(Arc<Pages<T>>);

impl<T> Clone for PageGroup<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Measured for PageGroup<T> {
    fn measure(&self) -> usize {
        1
    }
}

impl<T> UnitMeasured for PageGroup<T> {}

impl<T> PageGroup<T> {
    fn make_mut(&mut self, allocations: &mut u64) -> &mut Pages<T> {
        if Arc::strong_count(&self.0) > 1 {
            *allocations += 1;
        }
        Arc::make_mut(&mut self.0)
    }
}

pub(super) struct BatchPages<T> {
    groups: RetainedSequence<PageGroup<T>>,
    frontier: Option<PageGroup<T>>,
}

impl<T> Clone for BatchPages<T> {
    fn clone(&self) -> Self {
        Self {
            groups: self.groups.clone(),
            frontier: self.frontier.clone(),
        }
    }
}

impl<T> BatchPages<T> {
    pub(super) fn new() -> Self {
        Self {
            groups: RetainedSequence::default(),
            frontier: None,
        }
    }

    pub(super) fn get(&self, page: usize) -> Option<&Chunk<T>> {
        let group = page / GROUP;
        let pages = if group == self.groups.len() {
            self.frontier.as_ref()?
        } else {
            self.groups.get(group)?
        };
        pages.0.get(page % GROUP)
    }

    pub(super) fn get_mut(&mut self, page: usize, allocations: &mut u64) -> Option<&mut Chunk<T>> {
        let group = page / GROUP;
        let pages = if group == self.groups.len() {
            self.frontier.as_mut()?
        } else {
            self.groups.get_mut(group, allocations)?
        };
        pages.make_mut(allocations).get_mut(page % GROUP)
    }

    pub(super) fn get_with_work(&self, page: usize) -> (Option<&Chunk<T>>, u64) {
        let group = page / GROUP;
        if group == self.groups.len() {
            return (
                self.frontier
                    .as_ref()
                    .and_then(|pages| pages.0.get(page % GROUP)),
                1,
            );
        }
        let (pages, work) = self.groups.at_measure_with_work(group);
        (
            pages.and_then(|(_, pages)| pages.0.get(page % GROUP)),
            work + 1,
        )
    }

    pub(super) fn iter_from(&self, page: usize) -> impl Iterator<Item = &Chunk<T>> {
        let group = page / GROUP;
        self.groups
            .iter_from(group)
            .chain(
                self.frontier
                    .iter()
                    .filter(move |_| group <= self.groups.len()),
            )
            .flat_map(|pages| pages.0.iter())
            .skip(page % GROUP)
    }

    pub(super) fn push(&mut self, page: Chunk<T>, allocations: &mut u64) {
        if self
            .frontier
            .as_ref()
            .is_some_and(|pages| pages.0.len() == GROUP)
        {
            *allocations += self
                .groups
                .push(self.frontier.take().expect("full page group"));
        }
        if let Some(frontier) = &mut self.frontier {
            frontier.make_mut(allocations).push(page);
        } else {
            *allocations += 1;
            self.frontier = Some(PageGroup(Arc::new(smallvec![page])));
        }
    }

    pub(super) fn truncate(&mut self, len: usize, allocations: &mut u64) {
        let Some(last) = len.checked_sub(1) else {
            self.frontier = None;
            *allocations += self.groups.truncate(0);
            return;
        };
        let group = last / GROUP;
        if group < self.groups.len() {
            self.frontier = self.groups.get(group).cloned();
            *allocations += self.groups.truncate(group);
        }
        let frontier = self.frontier.as_mut().expect("retained page group");
        let keep = last % GROUP + 1;
        if keep < frontier.0.len() {
            frontier.make_mut(allocations).truncate(keep);
        }
    }
}

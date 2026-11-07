//! One scalar, invocation-local innermost child edge. Immutable loop ancestry
//! describes suspended outer edges; this marker never holds an owner or fuel.
use super::{Error, Invocation, invalid};
use std::num::NonZeroUsize;

#[derive(Default)]
pub(in crate::native_artifact::program::runtime) struct Pending(Option<NonZeroUsize>);
impl Pending {
    pub(super) fn is_pending(&self) -> bool {
        self.0.is_some()
    }
    pub(super) fn current(&self) -> Option<usize> {
        self.0.map(|value| value.get() - 1)
    }
    fn prepared(&self, expected: Option<usize>, next: Option<usize>) -> Result<Self, Error> {
        if self.current() != expected {
            return Err(invalid("native parent edge differs from its root context"));
        }
        let next = next
            .map(|child| {
                child
                    .checked_add(1)
                    .and_then(NonZeroUsize::new)
                    .ok_or_else(|| invalid("native parent child edge overflows"))
            })
            .transpose()?;
        Ok(Self(next))
    }
    fn arm(&mut self, child: usize, outer: Option<usize>) -> Result<(), Error> {
        if self.current() != outer {
            return Err(invalid("parent child edge differs from active ancestor"));
        }
        self.0 = Some(
            NonZeroUsize::new(
                child
                    .checked_add(1)
                    .ok_or_else(|| invalid("parent child edge overflows"))?,
            )
            .unwrap(),
        );
        Ok(())
    }
    fn complete(&mut self, child: usize, outer: Option<usize>) -> Result<(), Error> {
        let current = self
            .0
            .take()
            .ok_or_else(|| invalid("child completion without pending parent edge"))?;
        if current.get() - 1 != child {
            return Err(invalid("child completion differs from pending parent edge"));
        }
        self.0 = outer
            .map(|outer| {
                outer
                    .checked_add(1)
                    .and_then(NonZeroUsize::new)
                    .ok_or_else(|| invalid("parent child ancestor overflows"))
            })
            .transpose()?;
        Ok(())
    }
}

impl Invocation<'_, '_, '_> {
    /// Prepare only a cold handoff's scalar marker. Native initialization did
    /// not run the ordinary child callback; paid depth is a separate fact.
    /// All fallible validation precedes publication of the restored control.
    pub(super) fn native_parent_continuation(
        &self,
        region: usize,
        active: u32,
    ) -> Result<Option<Pending>, Error> {
        if self.worker.is_some() {
            return Ok(None);
        }
        let metadata = self.descriptor.metadata();
        let root = metadata.closed_tails.regions[region].iteration;
        let expected = metadata
            .compositions
            .native_context(region)
            .ok_or_else(|| invalid("native parent root context is outside its inventory"))?
            .map(|child| child as usize);
        let tracked = |iteration: u32| -> Result<Option<usize>, Error> {
            let control = metadata
                .plan
                .iterations()
                .get(iteration as usize)
                .ok_or_else(|| invalid("native parent ancestry is outside its plan"))?
                .as_ref();
            let Some((boundary, true)) = metadata
                .compositions
                .boundary(control.initialize() as usize)
            else {
                return Ok(None);
            };
            let (_, child) = metadata
                .compositions
                .child_boundary(boundary)
                .ok_or_else(|| invalid("native parent ancestry lacks its child edge"))?;
            if child.child != iteration {
                return Err(invalid(
                    "native parent ancestry differs from its child edge",
                ));
            }
            Ok(Some(iteration as usize))
        };
        let parent = |iteration: u32| {
            metadata.plan.iterations()[iteration as usize]
                .as_ref()
                .parent()
        };
        // Find the innermost tracked ACTIVE child and prove it lies under the
        // same native root, even if a nearer marker has already been found.
        let mut at = active;
        let mut next = None;
        loop {
            if next.is_none() {
                next = tracked(at)?;
            }
            if at == root {
                break;
            }
            at =
                parent(at).ok_or_else(|| invalid("native parent continuation escapes its root"))?;
        }
        let next = next.or(expected);
        self.parent_continuation.prepared(expected, next).map(Some)
    }

    pub(super) fn arm_parent_continuation(&mut self, site: usize) -> Result<(), Error> {
        if self.worker.is_some() {
            return Ok(());
        }
        let metadata = self.descriptor.metadata();
        if let Some((boundary, true)) = metadata.compositions.boundary(site) {
            let (parent, child) = metadata
                .compositions
                .child_boundary(boundary)
                .ok_or_else(|| invalid("parent child boundary is outside its inventory"))?;
            if self.state.control.current != Some(child.child)
                || self.progress[child.child as usize * 2 + 1] != child.initialize_progress
                || self.closed_tail.is_some()
            {
                return Err(invalid(
                    "pending parent edge outside admitted child initialization",
                ));
            }
            let outer = parent.suspended_child.map(|child| child as usize);
            self.parent_continuation.arm(child.child as usize, outer)?;
        }
        Ok(())
    }

    pub(super) fn complete_parent_continuation(
        &mut self,
        previous: Option<u32>,
        site: u64,
    ) -> Result<(), Error> {
        if self.worker.is_some() {
            return Ok(());
        }
        let metadata = self.descriptor.metadata();
        if let Some((boundary, false)) =
            previous.and_then(|p| self.descriptor.metadata().compositions.boundary(p as usize))
        {
            let (parent, child) = metadata
                .compositions
                .child_boundary(boundary)
                .ok_or_else(|| invalid("completed child boundary is outside its inventory"))?;
            // visit already requires the previous exit's action to be consumed.
            // Only its original successor may consume this exact child call.
            if site != u64::from(child.child_successor)
                || self.state.control.current != Some(parent.iteration)
                || self.progress[child.child as usize * 2 + 1] != 0
                || self.closed_tail.is_some()
            {
                return Err(invalid(
                    "parent continuation before exact completed child exit",
                ));
            }
            let outer = parent.suspended_child.map(|child| child as usize);
            self.parent_continuation
                .complete(child.child as usize, outer)?;
            #[cfg(all(test, feature = "canonical-native-program"))]
            super::super::composition_tests::observe::parent_child_completed(
                child.child,
                parent.iteration,
            );
        }
        if self.state.control.completed && self.parent_continuation.is_pending() {
            return Err(invalid("completion with pending parent child edge"));
        }
        Ok(())
    }
}

#[test]
fn parent_continuation_is_single_use_and_matches_the_exact_child_edge() {
    assert_eq!(std::mem::size_of::<Pending>(), std::mem::size_of::<usize>());
    let mut state = Pending::default();
    assert!(state.complete(3, None).is_err());
    state.arm(3, None).unwrap();
    assert!(state.arm(3, None).is_err());
    assert!(state.complete(4, None).is_err());
    assert!(!state.is_pending());
    state.arm(3, None).unwrap();
    state.arm(7, Some(3)).unwrap();
    assert_eq!(state.current(), Some(7));
    assert!(state.arm(9, Some(3)).is_err());
    state.complete(7, Some(3)).unwrap();
    assert_eq!(state.current(), Some(3));
    state.complete(3, None).unwrap();
    assert!(state.complete(3, None).is_err());
    state.arm(4, None).unwrap();
    state.complete(4, None).unwrap();
}

#[test]
fn native_parent_continuation_prepares_without_mutating_the_old_edge() {
    let mut state = Pending::default();
    state.arm(0, None).unwrap();
    assert!(state.prepared(None, Some(3)).is_err());
    assert!(state.prepared(Some(0), Some(usize::MAX)).is_err());
    assert_eq!(state.current(), Some(0));
    let mut restored = state.prepared(Some(0), Some(3)).unwrap();
    assert_eq!(state.current(), Some(0));
    assert_eq!(restored.current(), Some(3));
    restored.complete(3, Some(0)).unwrap();
    restored.complete(0, None).unwrap();
    assert!(!restored.is_pending());
}

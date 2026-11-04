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
    /// Replace only a sealed loop-tree's scalar edge marker. The caller has
    /// independently checked both markers against the current source ancestry;
    /// this neither borrows an owner nor admits an unvisited ordinary action.
    pub(super) fn reconcile_tree(
        &mut self,
        previous: Option<usize>,
        current: Option<usize>,
    ) -> Result<(), Error> {
        if self.current() != previous {
            return Err(invalid("numeric tree changed its pending parent edge"));
        }
        let next = current
            .map(|value| {
                value
                    .checked_add(1)
                    .and_then(NonZeroUsize::new)
                    .ok_or_else(|| invalid("numeric tree parent edge overflows"))
            })
            .transpose()?;
        self.0 = next;
        Ok(())
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
    pub(super) fn arm_parent_continuation(&mut self, site: usize) -> Result<(), Error> {
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

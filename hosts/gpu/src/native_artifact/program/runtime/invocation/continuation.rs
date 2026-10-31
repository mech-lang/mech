//! One scalar, invocation-local child edge. It never holds an owner or fuel.
use super::{Error, Invocation, invalid};
use std::num::NonZeroUsize;

#[derive(Default)]
pub(in crate::native_artifact::program::runtime) struct Pending(Option<NonZeroUsize>);
impl Pending {
    pub(super) fn is_pending(&self) -> bool {
        self.0.is_some()
    }
    fn arm(&mut self, parent: usize) -> Result<(), Error> {
        if self.is_pending() {
            return Err(invalid("parent child edge already pending"));
        }
        self.0 = Some(
            NonZeroUsize::new(
                parent
                    .checked_add(1)
                    .ok_or_else(|| invalid("parent child edge overflows"))?,
            )
            .unwrap(),
        );
        Ok(())
    }
    fn complete(&mut self, parent: usize) -> Result<(), Error> {
        let current = self
            .0
            .take()
            .ok_or_else(|| invalid("child completion without pending parent edge"))?;
        if current.get() - 1 != parent {
            return Err(invalid("child completion differs from pending parent edge"));
        }
        Ok(())
    }
}

impl Invocation<'_, '_, '_> {
    pub(super) fn arm_parent_continuation(&mut self, site: usize) -> Result<(), Error> {
        if self.worker.is_some() {
            return Ok(());
        }
        if let Some((parent, true)) = self.descriptor.metadata().compositions.boundary(site) {
            let proof = &self.descriptor.metadata().compositions.parents[parent];
            if self.state.control.current != Some(proof.child)
                || self.progress[proof.child as usize * 2 + 1] != 1
                || self.closed_tail.is_some()
            {
                return Err(invalid(
                    "pending parent edge outside admitted child initialization",
                ));
            }
            self.parent_continuation.arm(parent)?;
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
        if let Some((parent, false)) =
            previous.and_then(|p| self.descriptor.metadata().compositions.boundary(p as usize))
        {
            let proof = &self.descriptor.metadata().compositions.parents[parent];
            // visit already requires the previous exit's action to be consumed.
            // Only its original successor may consume this exact child call.
            if site != u64::from(proof.child_successor)
                || self.state.control.current != Some(proof.iteration)
                || self.progress[proof.child as usize * 2 + 1] != 0
                || self.closed_tail.is_some()
            {
                return Err(invalid(
                    "parent continuation before exact completed child exit",
                ));
            }
            self.parent_continuation.complete(parent)?;
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
    assert!(state.complete(3).is_err());
    state.arm(3).unwrap();
    assert!(state.arm(3).is_err());
    assert!(state.complete(4).is_err());
    assert!(!state.is_pending());
    state.arm(3).unwrap();
    state.complete(3).unwrap();
    assert!(state.complete(3).is_err());
    state.arm(4).unwrap();
    state.complete(4).unwrap();
}

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

use super::{
    MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult, PlanObjectKey, PublishedValueVersion,
    RealizedMemoryPlan, RuntimeBinding,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationCandidate {
    pub object: PlanObjectKey,
    pub binding: RuntimeBinding,
    pub shape: Box<[u64]>,
    pub changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedPublication {
    pub object: PlanObjectKey,
    pub binding: RuntimeBinding,
    pub shape: Box<[u64]>,
    pub version: PublishedValueVersion,
    pub changed: bool,
}

pub struct PreparedPublication {
    candidates: Box<[PublicationCandidate]>,
    completed: bool,
}

impl PreparedPublication {
    pub fn candidates(&self) -> &[PublicationCandidate] {
        &self.candidates
    }

    pub const fn is_completed(&self) -> bool {
        self.completed
    }
}

impl MemoryDomain {
    pub fn prepare_publication(
        &self,
        realized: &RealizedMemoryPlan,
        candidates: Vec<PublicationCandidate>,
    ) -> MemoryRuntimeResult<PreparedPublication> {
        if realized.domain() != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: realized.domain(),
            });
        }
        let state = self.state.borrow();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let mut seen = Vec::new();
        seen.try_reserve_exact(candidates.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: candidates.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        for candidate in &candidates {
            realized.binding(candidate.object)?;
            if seen.contains(&candidate.object.object()) {
                return Err(MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(candidate.object.object()),
                    reason: "a publication contains the same object more than once".into(),
                });
            }
            if candidate.binding.capacity_bytes() < candidate.binding.initialized_bytes() {
                return Err(MemoryRuntimeError::CapacityExceeded {
                    object: candidate.object.object(),
                    requested: candidate.binding.initialized_bytes(),
                    capacity: candidate.binding.capacity_bytes(),
                });
            }
            if let Some(handle) = candidate.binding.handle() {
                state.record(handle)?;
            }
            seen.push(candidate.object.object());
        }
        Ok(PreparedPublication {
            candidates: candidates.into_boxed_slice(),
            completed: false,
        })
    }

    pub fn commit_publication(
        &self,
        prepared: &mut PreparedPublication,
    ) -> MemoryRuntimeResult<Box<[CommittedPublication]>> {
        if prepared.completed {
            return Err(MemoryRuntimeError::PublicationAlreadyCompleted);
        }
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let changed = prepared
            .candidates
            .iter()
            .filter(|candidate| candidate.changed)
            .count();
        let final_version = (0..changed).try_fold(state.next_publication, |version, _| {
            version.checked_successor("published value version")
        })?;
        let mut next = state.next_publication;
        let mut committed = Vec::new();
        committed
            .try_reserve_exact(prepared.candidates.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: prepared.candidates.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            })?;
        for candidate in &prepared.candidates {
            let version = next;
            if candidate.changed {
                next = next.checked_successor("published value version")?;
            }
            committed.push(CommittedPublication {
                object: candidate.object,
                binding: candidate.binding.clone(),
                shape: candidate.shape.clone(),
                version,
                changed: candidate.changed,
            });
        }
        state.next_publication = final_version;
        prepared.completed = true;
        Ok(committed.into_boxed_slice())
    }

    pub fn abort_publication(&self, prepared: &mut PreparedPublication) -> MemoryRuntimeResult<()> {
        if prepared.completed {
            return Err(MemoryRuntimeError::PublicationAlreadyCompleted);
        }
        prepared.completed = true;
        Ok(())
    }
}

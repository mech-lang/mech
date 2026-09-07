#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

use super::{
    MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult, PlanObjectKey, PublishedValueVersion,
    RealizedMemoryPlan, RuntimeBinding,
};
use crate::{MResult, MechError, Value, ValueCell};

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
    committed: Box<[CommittedPublication]>,
    initial_version: PublishedValueVersion,
    final_version: PublishedValueVersion,
    completed: bool,
}

pub struct CellPublicationCandidate {
    pub cell: ValueCell,
    pub object: PlanObjectKey,
    pub binding: RuntimeBinding,
    pub value: Value,
    pub changed: bool,
}

pub struct PreparedCellPublication {
    memory: PreparedPublication,
    replacements: Box<[crate::cell_binding::PreparedManagedCellBinding]>,
}

/// Final publication authority. Construction acquires the domain gate and
/// every target cell gate after all validation has completed. Commit consumes
/// only already-owned candidates and performs no allocation or fallible work.
pub struct ReadyPublication {
    domain: MemoryDomain,
    prepared: PreparedCellPublication,
    completed: bool,
}

impl PreparedPublication {
    pub fn candidates(&self) -> &[PublicationCandidate] {
        &self.candidates
    }

    pub const fn is_completed(&self) -> bool {
        self.completed
    }

    pub fn planned_commits(&self) -> &[CommittedPublication] {
        &self.committed
    }
}

impl Drop for PreparedPublication {
    fn drop(&mut self) {
        // Candidates own no published state until commit. Marking an
        // abandoned preparation completed makes drop the fail-closed abort
        // path without invoking user code or fallible cleanup.
        self.completed = true;
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
            let expected = realized.binding(candidate.object)?;
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
            if candidate.binding.initialized_bytes()
                < candidate.binding.required_initialization_bytes()
            {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: candidate.object.object(),
                    requested: candidate.binding.required_initialization_bytes(),
                    initialized: candidate.binding.initialized_bytes(),
                });
            }
            if let Some(handle) = candidate.binding.handle() {
                state.record(handle)?;
            }
            if candidate.binding != expected {
                return Err(MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(candidate.object.object()),
                    reason: "publication binding differs from the realized plan object".into(),
                });
            }
            seen.push(candidate.object.object());
        }
        let initial_version = state.next_publication;
        let mut next = initial_version;
        let mut committed = Vec::new();
        committed.try_reserve_exact(candidates.len()).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: candidates.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }
        })?;
        for candidate in &candidates {
            let version = if candidate.changed {
                next = next.checked_successor("published value version")?;
                next
            } else {
                next
            };
            committed.push(CommittedPublication {
                object: candidate.object,
                binding: candidate.binding.clone(),
                shape: candidate.shape.clone(),
                version,
                changed: candidate.changed,
            });
        }
        Ok(PreparedPublication {
            candidates: candidates.into_boxed_slice(),
            committed: committed.into_boxed_slice(),
            initial_version,
            final_version: next,
            completed: false,
        })
    }

    pub fn commit_publication<'a>(
        &self,
        prepared: &'a mut PreparedPublication,
    ) -> MemoryRuntimeResult<&'a [CommittedPublication]> {
        if prepared.completed {
            return Err(MemoryRuntimeError::PublicationAlreadyCompleted);
        }
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if state.next_publication != prepared.initial_version {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: None,
                reason: "publication authority was superseded by another commit".into(),
            });
        }
        state.next_publication = prepared.final_version;
        prepared.completed = true;
        Ok(&prepared.committed)
    }

    pub fn abort_publication(&self, prepared: &mut PreparedPublication) -> MemoryRuntimeResult<()> {
        if prepared.completed {
            return Err(MemoryRuntimeError::PublicationAlreadyCompleted);
        }
        prepared.completed = true;
        Ok(())
    }

    pub fn prepare_cell_publication(
        &self,
        realized: &RealizedMemoryPlan,
        candidates: Vec<CellPublicationCandidate>,
    ) -> MResult<PreparedCellPublication> {
        let mut memory_candidates = Vec::new();
        memory_candidates
            .try_reserve_exact(candidates.len())
            .map_err(|_| {
                MechError::from(MemoryRuntimeError::AllocationFailed {
                    object: None,
                    requested: candidates.len() as u64,
                    alignment: 1,
                    space: crate::MemorySpace::Host,
                })
            })?;
        for candidate in &candidates {
            memory_candidates.push(PublicationCandidate {
                object: candidate.object,
                binding: candidate.binding.clone(),
                shape: candidate
                    .value
                    .shape()
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
                changed: candidate.changed,
            });
        }
        let mut memory = self
            .prepare_publication(realized, memory_candidates)
            .map_err(MechError::from)?;
        let mut replacements = Vec::new();
        if replacements.try_reserve_exact(candidates.len()).is_err() {
            let _ = self.abort_publication(&mut memory);
            return Err(MechError::from(MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: candidates.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            }));
        }
        let mut seen_cells = Vec::new();
        seen_cells
            .try_reserve_exact(candidates.len())
            .map_err(|_| {
                MechError::from(MemoryRuntimeError::AllocationFailed {
                    object: None,
                    requested: candidates.len() as u64,
                    alignment: 1,
                    space: crate::MemorySpace::Host,
                })
            })?;
        for candidate in candidates {
            let identity = candidate.cell.reactive_cell_id();
            if seen_cells.contains(&identity) {
                let _ = self.abort_publication(&mut memory);
                return Err(MechError::from(
                    MemoryRuntimeError::CandidateValidationFailed {
                        object: Some(candidate.object.object()),
                        reason: "a publication contains the same logical cell more than once"
                            .into(),
                    },
                ));
            }
            seen_cells.push(identity);
            match candidate
                .cell
                .prepare_managed_binding(self, &candidate.value, candidate.changed)
            {
                Ok(replacement) => replacements.push(replacement),
                Err(error) => {
                    let _ = self.abort_publication(&mut memory);
                    return Err(error);
                }
            }
        }
        Ok(PreparedCellPublication {
            memory,
            replacements: replacements.into_boxed_slice(),
        })
    }

    pub fn ready_cell_publication(
        &self,
        mut prepared: PreparedCellPublication,
    ) -> MResult<ReadyPublication> {
        if prepared.memory.completed {
            return Err(MechError::from(
                MemoryRuntimeError::PublicationAlreadyCompleted,
            ));
        }
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MechError::from(MemoryRuntimeError::DomainClosed));
        }
        if state.next_publication != prepared.memory.initial_version {
            return Err(MechError::from(
                MemoryRuntimeError::CandidateValidationFailed {
                    object: None,
                    reason: "publication authority was superseded by another commit".into(),
                },
            ));
        }
        if state.publication_in_progress {
            return Err(MechError::from(MemoryRuntimeError::PublicationInProgress));
        }
        for (replacement, committed) in prepared
            .replacements
            .iter()
            .zip(prepared.memory.committed.iter_mut())
        {
            committed.version = if replacement.changed {
                replacement
                    .expected_version
                    .checked_successor("cell content version")
                    .map_err(MechError::from)?
            } else {
                replacement.expected_version
            };
        }
        let mut locked = 0_usize;
        for replacement in prepared.replacements.iter() {
            if let Err(error) = replacement
                .cell
                .lock_publication(replacement.expected_version)
            {
                for previous in &prepared.replacements[..locked] {
                    previous.cell.unlock_publication();
                }
                return Err(error);
            }
            locked += 1;
        }
        state.publication_in_progress = true;
        drop(state);
        Ok(ReadyPublication {
            domain: self.clone(),
            prepared,
            completed: false,
        })
    }

    pub fn abort_cell_publication(
        &self,
        prepared: &mut PreparedCellPublication,
    ) -> MemoryRuntimeResult<()> {
        self.abort_publication(&mut prepared.memory)
    }
}

impl ReadyPublication {
    pub fn commit(mut self) -> Box<[CommittedPublication]> {
        {
            let mut state = self.domain.state.borrow_mut();
            debug_assert!(state.publication_in_progress);
            debug_assert_eq!(state.next_publication, self.prepared.memory.initial_version);
            state.next_publication = self.prepared.memory.final_version;
            state.publication_in_progress = false;
        }
        for (replacement, committed) in self
            .prepared
            .replacements
            .iter_mut()
            .zip(self.prepared.memory.committed.iter())
        {
            let cell = replacement.cell.clone();
            cell.install_managed_binding(replacement, committed.version);
        }
        self.prepared.memory.completed = true;
        self.completed = true;
        core::mem::take(&mut self.prepared.memory.committed)
    }
}

impl Drop for ReadyPublication {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        for replacement in self.prepared.replacements.iter() {
            replacement.cell.unlock_publication();
        }
        let mut state = self.domain.state.borrow_mut();
        state.publication_in_progress = false;
        self.prepared.memory.completed = true;
    }
}

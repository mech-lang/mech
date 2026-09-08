#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

use super::{
    MemoryAccessRegion, MemoryDomain, MemoryRuntimeError, MemoryRuntimeResult, PlanObjectKey,
    PublishedValueVersion, RealizedMemoryPlan, RuntimeBinding,
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
    domain: super::MemoryDomainId,
    revision: super::MemoryPlanRevision,
    realized: RealizedMemoryPlan,
    requires_active_plan: bool,
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
    /// Exact initialized geometry that becomes the cell's published view.
    pub region: MemoryAccessRegion,
    pub value: Value,
    pub changed: bool,
}

pub struct PreparedCellPublication {
    memory: PreparedPublication,
    replacements: Box<[crate::cell_binding::PreparedManagedCellBinding]>,
    undo: Option<super::PreparedUndoSnapshot>,
}

impl PreparedCellPublication {
    /// Whether this candidate still depends on the execution scope that
    /// produced it. Owned cells and admitted transaction partners retain
    /// their storage authority directly, so a reactive batch may finish
    /// staging sibling calls before acquiring the one atomic publication
    /// gate for the complete batch.
    pub(crate) const fn requires_active_plan(&self) -> bool {
        self.memory.requires_active_plan
    }
}

/// Final publication authority. Construction acquires the domain gate and
/// every target cell gate after all validation has completed. Commit consumes
/// only already-owned candidates and performs no allocation or fallible work.
pub struct ReadyPublication {
    domain: MemoryDomain,
    prepared: PreparedCellPublication,
    completed: bool,
}

struct PreparedBatchDomain {
    domain: MemoryDomain,
    initial_version: PublishedValueVersion,
    final_version: PublishedValueVersion,
}

/// A complete set of already-staged cell publications for one reactive
/// boundary. The individual candidates may belong to distinct call-plan
/// revisions (and, at explicit embedding boundaries, distinct owner
/// sessions), but none can become visible until the whole set is ready.
pub struct PreparedCellPublicationBatch {
    publications: Box<[PreparedCellPublication]>,
    domains: Box<[PreparedBatchDomain]>,
    completed: bool,
}

/// Infallible publication authority for an entire reactive register batch.
/// Every domain and cell gate is held before this type can be constructed.
pub struct ReadyPublicationBatch {
    prepared: PreparedCellPublicationBatch,
    completed: bool,
}

impl PreparedCellPublicationBatch {
    pub fn new(publications: Vec<PreparedCellPublication>) -> MResult<Self> {
        let mut domains: Vec<PreparedBatchDomain> = Vec::new();
        domains.try_reserve_exact(publications.len()).map_err(|_| {
            MechError::from(MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: publications.len() as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            })
        })?;
        for publication in &publications {
            if publication.memory.completed {
                return Err(MechError::from(
                    MemoryRuntimeError::PublicationAlreadyCompleted,
                ));
            }
            let domain_id = publication.memory.domain;
            if let Some(existing) = domains
                .iter()
                .find(|existing| existing.domain.id() == domain_id)
            {
                if existing.initial_version != publication.memory.initial_version {
                    return Err(MechError::from(
                        MemoryRuntimeError::CandidateValidationFailed {
                            object: None,
                            reason: "register publications were prepared against different domain versions"
                                .into(),
                        },
                    ));
                }
            } else {
                domains.push(PreparedBatchDomain {
                    domain: publication.memory.realized.owner_domain(),
                    initial_version: publication.memory.initial_version,
                    final_version: publication.memory.initial_version,
                });
            }
        }
        domains.sort_by_key(|domain| domain.domain.id());
        Ok(Self {
            publications: publications.into_boxed_slice(),
            domains: domains.into_boxed_slice(),
            completed: false,
        })
    }

    pub fn ready(mut self) -> MResult<ReadyPublicationBatch> {
        if self.completed {
            return Err(MechError::from(
                MemoryRuntimeError::PublicationAlreadyCompleted,
            ));
        }

        let mut seen_cells = Vec::new();
        let cell_count = self
            .publications
            .iter()
            .map(|publication| publication.replacements.len())
            .sum::<usize>();
        seen_cells.try_reserve_exact(cell_count).map_err(|_| {
            MechError::from(MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: cell_count as u64,
                alignment: 1,
                space: crate::MemorySpace::Host,
            })
        })?;

        for publication in self.publications.iter() {
            for candidate in publication.memory.candidates.iter() {
                if publication.memory.realized.binding(candidate.object)? != candidate.binding {
                    return Err(MemoryRuntimeError::CandidateValidationFailed {
                        object: Some(candidate.object.object()),
                        reason: "candidate storage incarnation changed after preparation".into(),
                    }
                    .into());
                }
            }
            for replacement in publication.replacements.iter() {
                let cell = replacement.cell.reactive_cell_id();
                if seen_cells.contains(&cell) {
                    return Err(MemoryRuntimeError::CandidateValidationFailed {
                        object: None,
                        reason: "a publication batch contains the same logical cell more than once"
                            .into(),
                    }
                    .into());
                }
                seen_cells.push(cell);
            }
        }

        for domain in self.domains.iter() {
            let state = domain.domain.state.borrow();
            if state.closed {
                return Err(MechError::from(MemoryRuntimeError::DomainClosed));
            }
            if state.next_publication != domain.initial_version {
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
            for publication in self
                .publications
                .iter()
                .filter(|publication| publication.memory.domain == domain.domain.id())
            {
                state.validate_publishable_revision(publication.memory.revision)?;
                if publication.memory.requires_active_plan
                    && state.execution_revision.or(state.active_revision)
                        != Some(publication.memory.revision)
                {
                    return Err(MemoryRuntimeError::InvalidPlanRevision {
                        expected: state.active_revision.unwrap_or(publication.memory.revision),
                        actual: publication.memory.revision,
                    }
                    .into());
                }
            }
        }

        for domain in self.domains.iter_mut() {
            let mut next = domain.initial_version;
            for publication in self
                .publications
                .iter_mut()
                .filter(|publication| publication.memory.domain == domain.domain.id())
            {
                for (replacement, committed) in publication
                    .replacements
                    .iter()
                    .zip(publication.memory.committed.iter_mut())
                {
                    committed.version = if replacement.changed {
                        next = next
                            .checked_successor("published value version")
                            .map_err(MechError::from)?;
                        replacement
                            .expected_version
                            .checked_successor("cell content version")
                            .map_err(MechError::from)?
                    } else {
                        replacement.expected_version
                    };
                }
            }
            domain.final_version = next;
        }

        let mut locked = 0_usize;
        for publication in self.publications.iter_mut() {
            for replacement in publication.replacements.iter_mut() {
                let cell = replacement.cell.clone();
                if let Err(error) = cell.lock_publication(replacement) {
                    unlock_batch_cells(&self.publications, locked);
                    return Err(error);
                }
                locked += 1;
            }
        }

        let mut entered_domains = 0_usize;
        for domain in self.domains.iter() {
            let Ok(mut state) = domain.domain.state.try_borrow_mut() else {
                unlock_batch_cells(&self.publications, locked);
                clear_batch_domain_gates(&self.domains, entered_domains);
                return Err(MechError::from(MemoryRuntimeError::PublicationInProgress));
            };
            if state.publication_in_progress {
                drop(state);
                unlock_batch_cells(&self.publications, locked);
                clear_batch_domain_gates(&self.domains, entered_domains);
                return Err(MechError::from(MemoryRuntimeError::PublicationInProgress));
            }
            state.publication_in_progress = true;
            entered_domains += 1;
        }

        Ok(ReadyPublicationBatch {
            prepared: self,
            completed: false,
        })
    }
}

fn unlock_batch_cells(publications: &[PreparedCellPublication], mut count: usize) {
    for publication in publications {
        for replacement in publication.replacements.iter() {
            if count == 0 {
                return;
            }
            replacement.cell.unlock_publication();
            count -= 1;
        }
    }
}

fn clear_batch_domain_gates(domains: &[PreparedBatchDomain], count: usize) {
    for domain in domains.iter().take(count) {
        domain.domain.state.borrow_mut().publication_in_progress = false;
    }
}

impl ReadyPublicationBatch {
    pub fn commit(mut self) {
        for domain in self.prepared.domains.iter() {
            let domain_id = domain.domain.id();
            let mut state = domain.domain.state.borrow_mut();
            debug_assert!(state.publication_in_progress);
            debug_assert_eq!(state.next_publication, domain.initial_version);
            state.next_publication = domain.final_version;
            for publication in self
                .prepared
                .publications
                .iter()
                .filter(|publication| publication.memory.domain == domain_id)
            {
                state.publish_ready_revision(publication.memory.revision);
            }
        }
        for publication in self.prepared.publications.iter_mut() {
            for (replacement, committed) in publication
                .replacements
                .iter_mut()
                .zip(publication.memory.committed.iter())
            {
                let cell = replacement.cell.clone();
                cell.install_managed_binding(replacement, committed.version);
            }
            if let Some(undo) = publication.undo.as_mut() {
                undo.commit();
            }
        }
        unlock_batch_cells(&self.prepared.publications, usize::MAX);
        clear_batch_domain_gates(&self.prepared.domains, self.prepared.domains.len());
        for publication in self.prepared.publications.iter_mut() {
            publication.memory.completed = true;
        }
        self.prepared.completed = true;
        self.completed = true;
    }
}

impl Drop for ReadyPublicationBatch {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        unlock_batch_cells(&self.prepared.publications, usize::MAX);
        clear_batch_domain_gates(&self.prepared.domains, self.prepared.domains.len());
        for publication in self.prepared.publications.iter_mut() {
            publication.memory.completed = true;
        }
        self.prepared.completed = true;
    }
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
        self.prepare_publication_with_authority(realized, candidates, true, None)
    }

    fn prepare_publication_with_authority(
        &self,
        realized: &RealizedMemoryPlan,
        candidates: Vec<PublicationCandidate>,
        requires_active_plan: bool,
        cell_candidates: Option<&[CellPublicationCandidate]>,
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
        if requires_active_plan
            && state.execution_revision.or(state.active_revision) != Some(realized.revision())
        {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(realized.revision()),
                actual: realized.revision(),
            });
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
        for (index, candidate) in candidates.iter().enumerate() {
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
            // An owned dynamic value may shrink within an admitted envelope.
            // Publication exposes its exact new logical region, not the old
            // extent or spare capacity. Use the same span and slot checks as
            // a read lease; a contiguous high-water mark cannot authorize gaps.
            let access = cell_candidates.map_or(MemoryAccessRegion::WholeInitialized, |cells| {
                cells[index].region
            });
            let (_, _, requested) = super::access::enclosing_span(
                candidate.object.object(),
                &expected,
                super::MemoryAccessMode::Read,
                access,
            )?;
            let region = state.regions.get(&candidate.object).ok_or(
                MemoryRuntimeError::UnknownPlanObject {
                    key: candidate.object,
                },
            )?;
            if !region
                .initialization
                .contains_region(access, region.initialized_bytes)
            {
                return Err(MemoryRuntimeError::UninitializedAccess {
                    object: candidate.object.object(),
                    requested,
                    initialized: region.initialized_bytes,
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
            domain: self.id(),
            revision: realized.revision(),
            realized: realized.clone(),
            requires_active_plan,
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
        if prepared.domain != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: prepared.domain,
                actual: self.id(),
            });
        }
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if state.active_revision != Some(prepared.revision) {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(prepared.revision),
                actual: prepared.revision,
            });
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
        if prepared.domain != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: prepared.domain,
                actual: self.id(),
            });
        }
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
        self.prepare_cell_publication_with_undo(realized, candidates, None)
    }

    pub(crate) fn prepare_cell_publication_with_undo(
        &self,
        realized: &RealizedMemoryPlan,
        candidates: Vec<CellPublicationCandidate>,
        undo: Option<super::PreparedUndoSnapshot>,
    ) -> MResult<PreparedCellPublication> {
        if let Some(undo) = &undo
            && (candidates.len() != 1 || !undo.matches(realized, candidates[0].object))
        {
            return Err(MemoryRuntimeError::CandidateValidationFailed {
                object: candidates
                    .first()
                    .map(|candidate| candidate.object.object()),
                reason: "undo authority differs from its single publication target".into(),
            }
            .into());
        }
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
        // The prepared cell replacements and the retained realization own
        // both sides of the publication until the ready gate is acquired.
        // They are therefore candidate authority in their own right; keeping
        // the producing execution scope alive would serialize sibling calls
        // in one session and make an atomic register batch impossible.
        // `ready_cell_publication` revalidates the revision, storage
        // incarnation, per-cell version, and domain before installing any
        // binding.
        let mut memory = self
            .prepare_publication_with_authority(
                realized,
                memory_candidates,
                false,
                Some(&candidates),
            )
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
            match candidate.cell.prepare_managed_binding(
                self,
                realized,
                candidate.object,
                candidate.region,
                &candidate.value,
                candidate.changed,
            ) {
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
            undo,
        })
    }

    pub fn ready_cell_publication(
        &self,
        mut prepared: PreparedCellPublication,
    ) -> MResult<ReadyPublication> {
        if prepared.memory.domain != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: prepared.memory.domain,
                actual: self.id(),
            }
            .into());
        }
        if prepared.memory.completed {
            return Err(MechError::from(
                MemoryRuntimeError::PublicationAlreadyCompleted,
            ));
        }
        for candidate in prepared.memory.candidates.iter() {
            if prepared.memory.realized.binding(candidate.object)? != candidate.binding {
                return Err(MemoryRuntimeError::CandidateValidationFailed {
                    object: Some(candidate.object.object()),
                    reason: "candidate storage incarnation changed after preparation".into(),
                }
                .into());
            }
        }
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MechError::from(MemoryRuntimeError::DomainClosed));
        }
        state.validate_publishable_revision(prepared.memory.revision)?;
        if prepared.memory.requires_active_plan
            && state.execution_revision.or(state.active_revision) != Some(prepared.memory.revision)
        {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.active_revision.unwrap_or(prepared.memory.revision),
                actual: prepared.memory.revision,
            }
            .into());
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
        for replacement in prepared.replacements.iter_mut() {
            let cell = replacement.cell.clone();
            if let Err(error) = cell.lock_publication(replacement) {
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
            state.publish_ready_revision(self.prepared.memory.revision);
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
        if let Some(undo) = self.prepared.undo.as_mut() {
            undo.commit();
        }
        for replacement in self.prepared.replacements.iter() {
            replacement.cell.unlock_publication();
        }
        self.domain.state.borrow_mut().publication_in_progress = false;
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

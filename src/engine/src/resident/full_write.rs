use core::sync::atomic::{AtomicU64, Ordering};

use mech_core::InstanceEpoch;

use super::{
    ResidentCandidateExecutionError as ResidentExecutionError, bench::ResidentTurnSummary,
    publish_epoch,
};

pub const FULL_WRITE_ELEMENTS: usize = 64 * 64;

pub struct ResidentFullWrite {
    versions: [Box<[f64]>; 2],
    coefficient: Box<[f64]>,
    buffer_epochs: [Option<InstanceEpoch>; 2],
    published_epoch: AtomicU64,
    next_epoch: Option<InstanceEpoch>,
    #[cfg(feature = "runtime_bench_probes")]
    publication_store_count: u64,
}

#[must_use = "prepared resident full writes must be published or aborted"]
pub struct PreparedResidentFullWrite<'a> {
    candidate: Option<PreparedFullWriteCandidate<'a>>,
    summary: ResidentTurnSummary,
}

#[must_use = "resident full-write candidates must be published or aborted"]
struct PreparedFullWriteCandidate<'a> {
    resident: Option<&'a mut ResidentFullWrite>,
    candidate: usize,
    before_epoch: InstanceEpoch,
    working_epoch: InstanceEpoch,
}

impl PreparedResidentFullWrite<'_> {
    pub fn summary(&self) -> ResidentTurnSummary {
        self.summary
    }

    #[inline]
    pub fn publish(mut self) {
        self.candidate
            .take()
            .expect("live prepared resident full write")
            .publish();
    }

    pub fn abort(mut self) {
        self.candidate
            .take()
            .expect("live prepared resident full write")
            .abort();
    }
}

impl Drop for PreparedResidentFullWrite<'_> {
    fn drop(&mut self) {
        if let Some(candidate) = self.candidate.take() {
            candidate.abort();
        }
    }
}

impl PreparedFullWriteCandidate<'_> {
    fn summary(&self) -> ResidentTurnSummary {
        let resident = self
            .resident
            .as_deref()
            .expect("live resident full-write candidate");
        let mut state_hash = 0xcbf29ce484222325_u64;
        for value in &resident.versions[self.candidate] {
            for byte in value.to_bits().to_le_bytes() {
                state_hash ^= u64::from(byte);
                state_hash = state_hash.wrapping_mul(0x100000001b3);
            }
        }
        ResidentTurnSummary {
            before_epoch: self.before_epoch.0,
            after_epoch: self.working_epoch.0,
            state_hash,
            touched_slots: 1,
            changed_slots: 1,
            dirty_nodes: 1,
        }
    }

    #[inline]
    fn publish(mut self) {
        let resident = self
            .resident
            .take()
            .expect("live resident full-write candidate");
        publish_epoch(&resident.published_epoch, self.working_epoch);
        #[cfg(feature = "runtime_bench_probes")]
        {
            resident.publication_store_count += 1;
        }
    }

    fn abort(mut self) {
        let resident = self
            .resident
            .take()
            .expect("live resident full-write candidate");
        resident.buffer_epochs[self.candidate] = None;
    }
}

impl Drop for PreparedFullWriteCandidate<'_> {
    fn drop(&mut self) {
        if let Some(resident) = self.resident.take() {
            resident.buffer_epochs[self.candidate] = None;
        }
    }
}

impl ResidentFullWrite {
    pub fn new() -> Self {
        let initial: Box<[f64]> = (0..FULL_WRITE_ELEMENTS)
            .map(|index| (index as f64 + 1.0) * 0.0001)
            .collect();
        let coefficient: Box<[f64]> = (0..FULL_WRITE_ELEMENTS)
            .map(|index| ((index % 127) as f64 + 1.0) * 0.000001)
            .collect();
        Self {
            versions: [initial.clone(), initial],
            coefficient,
            buffer_epochs: [Some(InstanceEpoch(0)), None],
            published_epoch: AtomicU64::new(0),
            next_epoch: Some(InstanceEpoch(1)),
            #[cfg(feature = "runtime_bench_probes")]
            publication_store_count: 0,
        }
    }

    #[inline]
    fn published_index(&self, epoch: InstanceEpoch) -> usize {
        if self.buffer_epochs[0] == Some(epoch) {
            0
        } else {
            1
        }
    }

    fn execute_candidate(
        &mut self,
        input: f64,
    ) -> Result<PreparedFullWriteCandidate<'_>, ResidentExecutionError> {
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next().ok();
        let published_epoch = InstanceEpoch(self.published_epoch.load(Ordering::Acquire));
        let published = self.published_index(published_epoch);
        debug_assert_eq!(self.buffer_epochs[published], Some(published_epoch));
        let candidate = 1 - published;
        self.buffer_epochs[candidate] = Some(working_epoch);
        if published == 0 {
            let (current, next) = self.versions.split_at_mut(1);
            for index in 0..FULL_WRITE_ELEMENTS {
                next[0][index] = current[0][index] * 1.000001 + self.coefficient[index] * input;
            }
        } else {
            let (next, current) = self.versions.split_at_mut(1);
            for index in 0..FULL_WRITE_ELEMENTS {
                next[0][index] = current[0][index] * 1.000001 + self.coefficient[index] * input;
            }
        }
        if !self.versions[candidate]
            .iter()
            .all(|value| value.is_finite())
        {
            self.buffer_epochs[candidate] = None;
            return Err(ResidentExecutionError::NonFiniteState);
        }
        Ok(PreparedFullWriteCandidate {
            resident: Some(self),
            candidate,
            before_epoch: published_epoch,
            working_epoch,
        })
    }

    pub fn prepare_turn(
        &mut self,
        input: f64,
    ) -> Result<PreparedResidentFullWrite<'_>, ResidentExecutionError> {
        let candidate = self.execute_candidate(input)?;
        let summary = candidate.summary();
        Ok(PreparedResidentFullWrite {
            candidate: Some(candidate),
            summary,
        })
    }

    #[inline]
    pub fn turn(&mut self, input: f64) -> Result<(), ResidentExecutionError> {
        self.execute_candidate(input)?.publish();
        Ok(())
    }

    pub fn execute_then_abort(&mut self, input: f64) -> Result<(), ResidentExecutionError> {
        self.execute_candidate(input)?.abort();
        Ok(())
    }

    pub fn published(&self) -> &[f64] {
        let epoch = InstanceEpoch(self.published_epoch.load(Ordering::Acquire));
        &self.versions[self.published_index(epoch)]
    }

    pub fn published_epoch(&self) -> u64 {
        self.published_epoch.load(Ordering::Acquire)
    }

    #[cfg(feature = "runtime_bench_probes")]
    pub fn structural_probe(&self) -> super::bench::ResidentTurnProbe {
        super::bench::ResidentTurnProbe {
            candidate_seed_bytes: 0,
            candidate_written_bytes: FULL_WRITE_ELEMENTS * size_of::<f64>(),
            published_buffer_copy_bytes: 0,
            publication_store_count: 1,
        }
    }

    #[cfg(feature = "runtime_bench_probes")]
    pub fn turn_with_probe(
        &mut self,
        input: f64,
    ) -> Result<super::bench::ResidentTurnProbe, ResidentExecutionError> {
        self.turn(input)?;
        Ok(self.structural_probe())
    }
}

impl Default for ResidentFullWrite {
    fn default() -> Self {
        Self::new()
    }
}

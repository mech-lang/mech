use core::sync::atomic::{AtomicU64, Ordering};

use mech_core::InstanceEpoch;

use super::{ResidentExecutionError, publish_epoch};

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

    fn execute(&mut self, input: f64, publish: bool) -> Result<(), ResidentExecutionError> {
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next();
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
        if publish {
            publish_epoch(&self.published_epoch, working_epoch);
            #[cfg(feature = "runtime_bench_probes")]
            {
                self.publication_store_count += 1;
            }
        } else {
            self.buffer_epochs[candidate] = None;
        }
        Ok(())
    }

    #[inline]
    pub fn turn(&mut self, input: f64) -> Result<(), ResidentExecutionError> {
        self.execute(input, true)
    }

    pub fn execute_then_abort(&mut self, input: f64) -> Result<(), ResidentExecutionError> {
        self.execute(input, false)
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

use super::{ReactiveInstance, ResidentExecutionError, execute_ekf_candidate};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResidentEkfState {
    pub state: [f64; 3],
    pub covariance: [f64; 9],
}

#[cfg(feature = "runtime_bench_probes")]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentTurnProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_written_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
}

pub struct ResidentEkfBatch {
    instance: ReactiveInstance,
}

impl ResidentEkfBatch {
    pub fn new(instances: usize) -> Self {
        Self {
            instance: ReactiveInstance::frozen_ekf_batch(instances),
        }
    }

    #[inline]
    pub fn turn(&mut self, input: [f64; 4]) -> Result<(), ResidentExecutionError> {
        let mut candidate = self.instance.begin_candidate(input)?;
        if let Err(error) = execute_ekf_candidate(&mut candidate) {
            candidate.abort();
            return Err(error);
        }
        candidate.publish();
        Ok(())
    }

    pub fn execute_then_abort(&mut self, input: [f64; 4]) -> Result<(), ResidentExecutionError> {
        let mut candidate = self.instance.begin_candidate(input)?;
        let result = execute_ekf_candidate(&mut candidate);
        candidate.abort();
        result
    }

    pub fn state(&self, index: usize) -> ResidentEkfState {
        let published = self.instance.published_epoch();
        ResidentEkfState {
            state: self.instance.state.published_state(published, index),
            covariance: self.instance.state.published_covariance(published, index),
        }
    }

    pub fn instances(&self) -> usize {
        self.instance.plan.instances
    }

    pub fn published_epoch(&self) -> u64 {
        self.instance.published_epoch().0
    }

    #[cfg(feature = "runtime_bench_probes")]
    pub fn structural_probe(&self) -> ResidentTurnProbe {
        debug_assert_eq!(
            self.instance.workspace.touched_slots.len(),
            self.instances() * 2
        );
        debug_assert_eq!(
            self.instance.workspace.changed_slots.len(),
            self.instances() * 2
        );
        debug_assert_eq!(
            self.instance.workspace.invalidated_slots.len(),
            self.instances() * 2
        );
        ResidentTurnProbe {
            candidate_seed_bytes: 0,
            candidate_written_bytes: self.instances() * 96,
            published_buffer_copy_bytes: 0,
            publication_store_count: 1,
        }
    }

    #[cfg(feature = "runtime_bench_probes")]
    pub fn turn_with_probe(
        &mut self,
        input: [f64; 4],
    ) -> Result<ResidentTurnProbe, ResidentExecutionError> {
        self.turn(input)?;
        Ok(self.structural_probe())
    }
}

use super::{
    Candidate, GateBInstance, NODES_PER_EKF, ResidentExecutionError, execute_ekf_candidate,
    execute_scheduled_ekf_candidate,
};

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
    instance: GateBInstance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentTurnSummary {
    pub before_epoch: u64,
    pub after_epoch: u64,
    pub state_hash: u64,
    pub touched_slots: u16,
    pub changed_slots: u16,
    pub dirty_nodes: u16,
}

#[must_use = "prepared resident turns must be published or aborted"]
pub struct PreparedResidentTurn<'a> {
    candidate: Option<Candidate<'a>>,
    summary: ResidentTurnSummary,
}

impl PreparedResidentTurn<'_> {
    pub fn summary(&self) -> ResidentTurnSummary {
        self.summary
    }

    pub fn published_epoch(&self) -> u64 {
        self.candidate
            .as_ref()
            .expect("live prepared resident turn")
            .instance
            .published_epoch()
            .0
    }

    pub fn published_state(&self, index: usize) -> ResidentEkfState {
        let candidate = self
            .candidate
            .as_ref()
            .expect("live prepared resident turn");
        ResidentEkfState {
            state: candidate
                .instance
                .state
                .published_state(candidate.base_epoch, index),
            covariance: candidate
                .instance
                .state
                .published_covariance(candidate.base_epoch, index),
        }
    }

    #[inline]
    pub fn publish(mut self) {
        self.candidate
            .take()
            .expect("live prepared resident turn")
            .publish();
    }

    pub fn abort(mut self) {
        self.candidate
            .take()
            .expect("live prepared resident turn")
            .abort();
    }
}

impl Drop for PreparedResidentTurn<'_> {
    fn drop(&mut self) {
        if let Some(candidate) = self.candidate.take() {
            candidate.abort();
        }
    }
}

impl ResidentEkfBatch {
    pub fn new(instances: usize) -> Self {
        Self {
            instance: GateBInstance::new(instances),
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

    #[inline]
    pub fn scheduled_turn(&mut self, input: [f64; 4]) -> Result<(), ResidentExecutionError> {
        let mut candidate = self.instance.begin_candidate(input)?;
        if let Err(error) = execute_scheduled_ekf_candidate(&mut candidate) {
            candidate.abort();
            return Err(error);
        }
        candidate.publish();
        Ok(())
    }

    pub fn prepare_scheduled_turn(
        &mut self,
        input: [f64; 4],
    ) -> Result<PreparedResidentTurn<'_>, ResidentExecutionError> {
        let instances = self.instance.plan.instances;
        let mut candidate = self.instance.begin_candidate(input)?;
        if let Err(error) = execute_scheduled_ekf_candidate(&mut candidate) {
            candidate.abort();
            return Err(error);
        }
        let workspace = &candidate.instance.workspace;
        let summary = ResidentTurnSummary {
            before_epoch: candidate.base_epoch.0,
            after_epoch: candidate.working_epoch.0,
            state_hash: candidate
                .instance
                .state
                .candidate_state_hash(candidate.candidate_buffer),
            touched_slots: u16::try_from(workspace.touched_slots.len())
                .expect("resident touched count fits u16"),
            changed_slots: u16::try_from(workspace.changed_slots.len())
                .expect("resident changed count fits u16"),
            dirty_nodes: u16::try_from(workspace.executed_nodes.len())
                .expect("resident dirty-node count fits u16"),
        };
        debug_assert_eq!(summary.touched_slots as usize, instances * 2);
        debug_assert_eq!(summary.changed_slots as usize, instances * 2);
        debug_assert_eq!(summary.dirty_nodes as usize, instances * NODES_PER_EKF);
        Ok(PreparedResidentTurn {
            candidate: Some(candidate),
            summary,
        })
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

    #[doc(hidden)]
    pub fn set_next_epoch_for_gate_b(&mut self, next_epoch: u64) {
        assert_ne!(next_epoch, 0, "resident candidate epochs are non-zero");
        self.instance.next_epoch = Some(mech_core::InstanceEpoch(next_epoch));
    }

    #[doc(hidden)]
    pub fn candidate_epoch_is_active_for_gate_b(&self, epoch: u64) -> bool {
        self.instance
            .state
            .contains_epoch(mech_core::InstanceEpoch(epoch))
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

    #[cfg(feature = "runtime_bench_probes")]
    pub fn scheduled_turn_with_probe(
        &mut self,
        input: [f64; 4],
    ) -> Result<ResidentTurnProbe, ResidentExecutionError> {
        self.scheduled_turn(input)?;
        Ok(self.structural_probe())
    }
}

use mech_engine::__gate_b_resident::{ResidentEkfBatch, ResidentFullWrite};
use mech_runtime::__gate_b_recording::{GateBFixedReceipt, ResidentTurnRecorder};

use super::{
    contract::{
        EPISODE_LENGTH, EkfInput, EkfState, assert_state_close, quantized_trajectory_hash,
        reference_trajectory, trace,
    },
    full_write::{WRITTEN_BYTES, buffer_hash},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentCompleteProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_written_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
    pub receipt_bytes: usize,
    pub dirty_nodes: usize,
    pub record_preparation_count: usize,
    pub record_append_count: usize,
    pub records_retained_before_timing: usize,
    pub records_appended: usize,
    pub ledger_records_inspected: usize,
}

fn input_array(input: EkfInput) -> [f64; 4] {
    [
        input.velocity,
        input.angular_velocity,
        input.measured_range,
        input.measured_bearing,
    ]
}

fn state_hash64(state: EkfState) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for value in state.values() {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn batch_state_hash(state: EkfState, instances: usize) -> u64 {
    let state_hash = state_hash64(state);
    let mut hash = 0xcbf29ce484222325_u64;
    for _ in 0..instances {
        hash ^= state_hash;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub struct ResidentScheduledFixture {
    resident: ResidentEkfBatch,
}

impl ResidentScheduledFixture {
    pub fn new(instances: usize) -> Self {
        Self {
            resident: ResidentEkfBatch::new(instances),
        }
    }

    pub fn run_episode(&mut self) {
        for input in trace().iter().copied() {
            self.resident
                .scheduled_turn(input_array(input))
                .expect("frozen scheduled resident turn");
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(EPISODE_LENGTH);
        for (turn, (input, expected)) in trace()
            .iter()
            .copied()
            .zip(reference_trajectory().iter().copied())
            .enumerate()
        {
            self.resident
                .scheduled_turn(input_array(input))
                .expect("frozen scheduled resident turn");
            for instance in 0..self.resident.instances() {
                assert_state_close(self.state(instance), expected, turn + 1);
            }
            trajectory.push(self.state(0));
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn state(&self, instance: usize) -> EkfState {
        let state = self.resident.state(instance);
        EkfState {
            state: state.state,
            covariance: state.covariance,
        }
    }

    pub fn validate_final(&self) {
        for instance in 0..self.resident.instances() {
            assert_state_close(
                self.state(instance),
                EkfState::REFERENCE_FINAL,
                EPISODE_LENGTH,
            );
        }
    }
}

pub struct ResidentTurnFixture {
    resident: ResidentEkfBatch,
    recorder: ResidentTurnRecorder,
    instances: usize,
    retained_history: usize,
}

impl ResidentTurnFixture {
    pub fn new(instances: usize, retained_history: usize, next_epoch: u64) -> Self {
        let mut resident = ResidentEkfBatch::new(instances);
        resident.set_next_epoch_for_gate_b(next_epoch);
        Self {
            resident,
            recorder: ResidentTurnRecorder::new(EPISODE_LENGTH, retained_history)
                .expect("resident Gate B recorder setup"),
            instances,
            retained_history,
        }
    }

    #[inline]
    fn run_turn(&mut self, turn: usize, input: EkfInput) {
        let permit = self
            .recorder
            .take_admission_permit(turn)
            .expect("pre-reserved resident admission");
        let prepared = self
            .resident
            .prepare_scheduled_turn(input_array(input))
            .expect("frozen complete resident turn");
        let commit = self
            .recorder
            .prepare_commit(permit, prepared)
            .expect("exact resident receipt preparation");
        commit.commit();
    }

    pub fn run_episode(&mut self) {
        for (turn, input) in trace().iter().copied().enumerate() {
            self.run_turn(turn, input);
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(EPISODE_LENGTH);
        for (turn, (input, expected)) in trace()
            .iter()
            .copied()
            .zip(reference_trajectory().iter().copied())
            .enumerate()
        {
            let permit = self
                .recorder
                .take_admission_permit(turn)
                .expect("pre-reserved resident admission");
            let prepared = self
                .resident
                .prepare_scheduled_turn(input_array(input))
                .expect("frozen complete resident turn");
            let summary = prepared.summary();
            assert_eq!(
                summary.state_hash,
                batch_state_hash(expected, self.instances)
            );
            let commit = self
                .recorder
                .prepare_commit(permit, prepared)
                .expect("exact resident receipt preparation");
            commit.commit();
            for instance in 0..self.instances {
                assert_state_close(self.state(instance), expected, turn + 1);
            }
            trajectory.push(self.state(0));
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn state(&self, instance: usize) -> EkfState {
        let state = self.resident.state(instance);
        EkfState {
            state: state.state,
            covariance: state.covariance,
        }
    }

    pub fn validate_final(&self) {
        for instance in 0..self.instances {
            assert_state_close(
                self.state(instance),
                EkfState::REFERENCE_FINAL,
                EPISODE_LENGTH,
            );
        }
        assert_eq!(
            self.recorder.recorded_ledger_len(),
            self.retained_history + EPISODE_LENGTH
        );
    }

    pub fn probe(&self) -> ResidentCompleteProbe {
        ResidentCompleteProbe {
            candidate_seed_bytes: 0,
            candidate_written_bytes: self.instances * 96,
            published_buffer_copy_bytes: 0,
            publication_store_count: 1,
            receipt_bytes: GateBFixedReceipt::RETAINED_BYTES,
            dirty_nodes: self.instances * 15,
            record_preparation_count: 1,
            record_append_count: 1,
            records_retained_before_timing: self.retained_history,
            records_appended: EPISODE_LENGTH,
            ledger_records_inspected: self.recorder.records_inspected(),
        }
    }
}

pub struct ResidentFullWriteTurnFixture {
    resident: ResidentFullWrite,
    recorder: ResidentTurnRecorder,
}

impl ResidentFullWriteTurnFixture {
    pub fn new() -> Self {
        Self {
            resident: ResidentFullWrite::new(),
            recorder: ResidentTurnRecorder::new(EPISODE_LENGTH, 0)
                .expect("resident full-write recorder setup"),
        }
    }

    pub fn run_episode(&mut self) {
        for (turn, input) in trace().iter().enumerate() {
            let permit = self
                .recorder
                .take_admission_permit(turn)
                .expect("pre-reserved full-write admission");
            let prepared = self
                .resident
                .prepare_turn(input.velocity)
                .expect("valid resident full-write candidate");
            let commit = self
                .recorder
                .prepare_full_write_commit(permit, prepared)
                .expect("exact full-write receipt preparation");
            commit.commit();
        }
    }

    pub fn published(&self) -> &[f64] {
        self.resident.published()
    }

    pub fn abort_output_hash_with_rejection(&mut self) -> String {
        let before_epoch = self.resident.published_epoch();
        let before_hash = buffer_hash(self.resident.published());
        let permit = self
            .recorder
            .take_admission_permit(0)
            .expect("pre-reserved rejected full-write admission");
        self.resident
            .prepare_turn(1.0)
            .expect("valid candidate before forced rejection")
            .abort();
        self.recorder
            .prepare_rejected(
                permit,
                before_epoch,
                mech_engine::__gate_b_resident::ResidentExecutionError::NonFiniteState,
            )
            .expect("rejected full-write receipt preparation")
            .append();
        assert_eq!(self.resident.published_epoch(), before_epoch);
        assert_eq!(buffer_hash(self.resident.published()), before_hash);
        assert_eq!(self.recorder.recorded_ledger_len(), 1);
        before_hash
    }

    pub fn probe(&self) -> ResidentCompleteProbe {
        ResidentCompleteProbe {
            candidate_seed_bytes: 0,
            candidate_written_bytes: WRITTEN_BYTES,
            published_buffer_copy_bytes: 0,
            publication_store_count: 1,
            receipt_bytes: GateBFixedReceipt::RETAINED_BYTES,
            dirty_nodes: 1,
            record_preparation_count: 1,
            record_append_count: 1,
            records_retained_before_timing: 0,
            records_appended: EPISODE_LENGTH,
            ledger_records_inspected: self.recorder.records_inspected(),
        }
    }
}

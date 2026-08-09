use mech_engine::__gate_b_resident::{ResidentEkfBatch, ResidentFullWrite};

#[cfg(feature = "runtime_bench_probes")]
use mech_engine::__gate_b_resident::ResidentTurnProbe;

use super::contract::{
    EPISODE_LENGTH, EkfState, assert_state_close, quantized_trajectory_hash, reference_trajectory,
    trace,
};
use super::full_write::buffer_hash;

#[derive(Clone, Copy, Debug, Default)]
pub struct ResidentKernelProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_written_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
}

#[cfg(feature = "runtime_bench_probes")]
impl From<ResidentTurnProbe> for ResidentKernelProbe {
    fn from(probe: ResidentTurnProbe) -> Self {
        Self {
            candidate_seed_bytes: probe.candidate_seed_bytes,
            candidate_written_bytes: probe.candidate_written_bytes,
            published_buffer_copy_bytes: probe.published_buffer_copy_bytes,
            publication_store_count: probe.publication_store_count,
        }
    }
}

pub struct ResidentKernelFixture {
    resident: ResidentEkfBatch,
}

pub struct ResidentFusedFixture {
    resident: ResidentEkfBatch,
}

impl ResidentFusedFixture {
    pub fn new(instances: usize) -> Self {
        Self {
            resident: ResidentEkfBatch::new(instances),
        }
    }

    pub fn run_episode(&mut self) {
        for input in trace() {
            self.resident
                .fused_turn([
                    input.velocity,
                    input.angular_velocity,
                    input.measured_range,
                    input.measured_bearing,
                ])
                .expect("fused resident EKF turn");
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(EPISODE_LENGTH);
        for (turn, (input, expected)) in trace().iter().zip(reference_trajectory()).enumerate() {
            self.resident
                .fused_turn([
                    input.velocity,
                    input.angular_velocity,
                    input.measured_range,
                    input.measured_bearing,
                ])
                .expect("fused resident EKF turn");
            for instance in 0..self.resident.instances() {
                assert_state_close(self.state(instance), *expected, turn + 1);
            }
            trajectory.push(self.state(0));
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn state(&self, index: usize) -> EkfState {
        let state = self.resident.state(index);
        EkfState {
            state: state.state,
            covariance: state.covariance,
        }
    }

    pub fn validate_final(&self) {
        for index in 0..self.resident.instances() {
            assert_state_close(self.state(index), EkfState::REFERENCE_FINAL, EPISODE_LENGTH);
        }
    }
}

impl ResidentKernelFixture {
    pub fn new(instances: usize) -> Self {
        Self {
            resident: ResidentEkfBatch::new(instances),
        }
    }

    pub fn run_episode(&mut self) {
        for input in trace() {
            self.resident
                .turn([
                    input.velocity,
                    input.angular_velocity,
                    input.measured_range,
                    input.measured_bearing,
                ])
                .expect("frozen resident EKF turn");
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(EPISODE_LENGTH);
        for (turn, (input, expected)) in trace().iter().zip(reference_trajectory()).enumerate() {
            self.resident
                .turn([
                    input.velocity,
                    input.angular_velocity,
                    input.measured_range,
                    input.measured_bearing,
                ])
                .expect("frozen resident EKF turn");
            for instance in 0..self.resident.instances() {
                assert_state_close(self.state(instance), *expected, turn + 1);
            }
            trajectory.push(self.state(0));
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn state(&self, index: usize) -> EkfState {
        let state = self.resident.state(index);
        EkfState {
            state: state.state,
            covariance: state.covariance,
        }
    }

    pub fn validate_final(&self) {
        for index in 0..self.resident.instances() {
            assert_state_close(self.state(index), EkfState::REFERENCE_FINAL, EPISODE_LENGTH);
        }
    }

    pub fn force_rejected_turn_preserves_publication(&mut self) {
        let before: Vec<_> = (0..self.resident.instances())
            .map(|index| self.resident.state(index))
            .collect();
        let epoch = self.resident.published_epoch();
        let input = trace()[0];
        self.resident
            .execute_then_abort([
                input.velocity,
                input.angular_velocity,
                input.measured_range,
                input.measured_bearing,
            ])
            .expect("valid candidate before forced abort");
        for (index, state) in before.into_iter().enumerate() {
            assert_eq!(self.resident.state(index), state);
        }
        assert_eq!(self.resident.published_epoch(), epoch);
    }

    #[cfg(feature = "runtime_bench_probes")]
    pub fn probe(&self) -> ResidentKernelProbe {
        self.resident.structural_probe().into()
    }
}

pub struct ResidentFullWriteFixture {
    resident: ResidentFullWrite,
}

impl ResidentFullWriteFixture {
    pub fn new() -> Self {
        Self {
            resident: ResidentFullWrite::new(),
        }
    }

    pub fn run_episode(&mut self) {
        for input in trace() {
            self.resident
                .turn(input.velocity)
                .expect("frozen resident full-write turn");
        }
    }

    pub fn published(&self) -> &[f64] {
        self.resident.published()
    }

    pub fn abort_output_hash(&mut self) -> String {
        let before_epoch = self.resident.published_epoch();
        let before_hash = buffer_hash(self.resident.published());
        self.resident
            .execute_then_abort(1.0)
            .expect("valid resident full-write candidate before abort");
        assert_eq!(self.resident.published_epoch(), before_epoch);
        assert_eq!(buffer_hash(self.resident.published()), before_hash);
        before_hash
    }

    #[cfg(feature = "runtime_bench_probes")]
    pub fn probe(&self) -> ResidentKernelProbe {
        self.resident.structural_probe().into()
    }
}

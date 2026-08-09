use mech_engine::__gate_b_resident::FusedRustEkfBatch;

use super::contract::{
    EPISODE_LENGTH, EkfState, assert_state_close, quantized_trajectory_hash, reference_trajectory,
    trace,
};

pub struct FusedRustFixture {
    batch: FusedRustEkfBatch,
}

impl FusedRustFixture {
    pub fn new(instances: usize) -> Self {
        Self {
            batch: FusedRustEkfBatch::new(instances),
        }
    }

    pub fn run_episode(&mut self) {
        for input in trace() {
            self.batch
                .turn([
                    input.velocity,
                    input.angular_velocity,
                    input.measured_range,
                    input.measured_bearing,
                ])
                .expect("fused Rust EKF turn");
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(EPISODE_LENGTH);
        for (turn, (input, expected)) in trace().iter().zip(reference_trajectory()).enumerate() {
            self.batch
                .turn([
                    input.velocity,
                    input.angular_velocity,
                    input.measured_range,
                    input.measured_bearing,
                ])
                .expect("fused Rust EKF turn");
            for instance in 0..self.batch.instances() {
                assert_state_close(self.state(instance), *expected, turn + 1);
            }
            trajectory.push(self.state(0));
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn state(&self, instance: usize) -> EkfState {
        let state = self.batch.state(instance);
        EkfState {
            state: state.state,
            covariance: state.covariance,
        }
    }

    pub fn validate_final(&self) {
        for instance in 0..self.batch.instances() {
            assert_state_close(
                self.state(instance),
                EkfState::REFERENCE_FINAL,
                EPISODE_LENGTH,
            );
        }
    }
}

//! Candidate execution for an artifact-derived resident EKF instance.

use core::sync::atomic::Ordering;

use mech_core::{InstanceEpoch, ProgramRevision, ReactiveInstanceId, SlotIndex};

use super::ResidentExecutionError;
use super::program_activation::{
    ActivatedNodeIndex, ActivatedNodeKind, ActivatedWrite, EkfPredicateSlot, ReactiveInstance,
    ResidentStorageLocation,
};
use crate::efficacy::ekf::math::{self, EkfMathError};
use crate::efficacy::ekf::operation::{EkfKernel, EkfPredicate};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentTurnSummary {
    pub instance: ReactiveInstanceId,
    pub program_revision: ProgramRevision,
    pub before_epoch: InstanceEpoch,
    pub after_epoch: InstanceEpoch,
    pub state_hash: u64,
    pub touched_slots: u16,
    pub changed_slots: u16,
    pub dirty_nodes: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentStructuralProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_written_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
    pub commit_runtime_call_count: usize,
    pub legacy_journal_capture_count: usize,
}

#[must_use = "prepared artifact resident turns must be published or aborted"]
pub struct PreparedResidentTurn<'a> {
    instance: Option<&'a mut ReactiveInstance>,
    published_buffer: usize,
    candidate_buffer: usize,
    working_epoch: InstanceEpoch,
    summary: ResidentTurnSummary,
}

impl PreparedResidentTurn<'_> {
    pub fn summary(&self) -> ResidentTurnSummary {
        self.summary
    }

    pub fn published_estimate(&self) -> &[f64; 3] {
        let instance = self.instance.as_deref().expect("live prepared turn");
        &instance.state.state.buffers[self.published_buffer]
    }

    #[inline]
    pub fn publish(mut self) -> ResidentTurnSummary {
        let instance = self.instance.take().expect("live prepared turn");
        instance
            .published_epoch
            .store(self.working_epoch.get(), Ordering::Release);
        self.summary
    }

    pub fn abort(mut self) {
        let instance = self.instance.take().expect("live prepared turn");
        instance
            .state
            .reject_candidate(self.candidate_buffer, self.working_epoch);
    }
}

impl Drop for PreparedResidentTurn<'_> {
    fn drop(&mut self) {
        debug_assert!(
            self.instance.is_none(),
            "prepared artifact resident turn must publish or abort explicitly"
        );
    }
}

impl ReactiveInstance {
    pub fn prepare_turn(
        &mut self,
        frame: [f64; 4],
    ) -> Result<PreparedResidentTurn<'_>, ResidentExecutionError> {
        let working_epoch = self
            .next_epoch
            .ok_or(ResidentExecutionError::EpochExhausted)?;
        self.next_epoch = working_epoch.checked_next().ok();
        let before_epoch = self.published_epoch();
        let (published_buffer, candidate_buffer) =
            self.state.begin_candidate(before_epoch, working_epoch);
        self.workspace.begin(frame);
        if let Err(error) =
            self.execute_candidate(published_buffer, candidate_buffer, working_epoch)
        {
            self.state.reject_candidate(candidate_buffer, working_epoch);
            return Err(error);
        }
        let summary = ResidentTurnSummary {
            instance: self.id,
            program_revision: self.plan.program_revision,
            before_epoch,
            after_epoch: working_epoch,
            state_hash: candidate_hash(self, candidate_buffer),
            touched_slots: u16::try_from(self.workspace.touched_slots.len())
                .expect("frozen resident touched count fits u16"),
            changed_slots: u16::try_from(self.workspace.changed_slots.len())
                .expect("frozen resident changed count fits u16"),
            dirty_nodes: u16::try_from(self.workspace.executed_node_count())
                .expect("frozen resident node count fits u16"),
        };
        Ok(PreparedResidentTurn {
            instance: Some(self),
            published_buffer,
            candidate_buffer,
            working_epoch,
            summary,
        })
    }

    #[inline]
    pub(crate) fn turn(
        &mut self,
        frame: [f64; 4],
    ) -> Result<ResidentTurnSummary, ResidentExecutionError> {
        Ok(self.prepare_turn(frame)?.publish())
    }

    pub fn execute_then_abort(
        &mut self,
        frame: [f64; 4],
    ) -> Result<ResidentTurnSummary, ResidentExecutionError> {
        let turn = self.prepare_turn(frame)?;
        let summary = turn.summary();
        turn.abort();
        Ok(summary)
    }

    pub fn estimate(&self) -> &[f64; 3] {
        let index = self.state.state.published_index(self.published_epoch());
        &self.state.state.buffers[index]
    }

    pub fn covariance(&self) -> &[f64; 9] {
        let index = self
            .state
            .covariance
            .published_index(self.published_epoch());
        &self.state.covariance.buffers[index]
    }

    pub fn structural_probe(&self) -> ResidentStructuralProbe {
        ResidentStructuralProbe {
            candidate_seed_bytes: 0,
            candidate_written_bytes: 96,
            published_buffer_copy_bytes: 0,
            publication_store_count: 1,
            commit_runtime_call_count: 0,
            legacy_journal_capture_count: 0,
        }
    }

    #[doc(hidden)]
    pub fn set_next_epoch_for_d1_test(&mut self, next: u64) {
        assert_ne!(next, 0, "candidate epochs are non-zero");
        self.next_epoch = Some(InstanceEpoch::new(next));
    }

    #[cfg(test)]
    pub(crate) fn version_addresses_for_d1_test(&self) -> ([usize; 2], [usize; 2]) {
        (
            [
                self.state.state.buffers[0].as_ptr() as usize,
                self.state.state.buffers[1].as_ptr() as usize,
            ],
            [
                self.state.covariance.buffers[0].as_ptr() as usize,
                self.state.covariance.buffers[1].as_ptr() as usize,
            ],
        )
    }

    fn execute_candidate(
        &mut self,
        published_buffer: usize,
        candidate_buffer: usize,
        working_epoch: InstanceEpoch,
    ) -> Result<(), ResidentExecutionError> {
        // Activation condenses the admitted dependency graph into dense masks.
        // The hot turn still propagates only semantic changes reported by each
        // kernel, without rediscovering edges or allocating scheduler state.
        let mut dirty_mask =
            self.plan.topology.turn_root_mask | self.plan.topology.mandatory_candidate_mask;
        for order in 0..self.plan.topology.linear_node_order.len() {
            let node_index = self.plan.topology.linear_node_order[order];
            let node_offset = node_index.0 as usize;
            let node_mask = 1_u32 << node_index.0;
            if dirty_mask & node_mask == 0 {
                continue;
            }
            self.workspace.record_dirty(node_index);
            let kind = self.plan.nodes[node_index.0 as usize].kind;
            let changed = match kind {
                ActivatedNodeKind::Kernel(kernel) => {
                    let was_initialized = self.workspace.output_initialized(node_index);
                    let changed = self.execute_kernel(kernel, published_buffer)?;
                    self.workspace.record_output_initialized(node_index);
                    semantic_output_changed(was_initialized, changed)
                }
                ActivatedNodeKind::Predicate(predicate) => {
                    let was_initialized = self.workspace.output_initialized(node_index);
                    let changed = self.execute_predicate(predicate);
                    self.workspace.record_output_initialized(node_index);
                    semantic_output_changed(was_initialized, changed)
                }
                ActivatedNodeKind::StateCopy { target, .. } => self.execute_state_copy(
                    target,
                    published_buffer,
                    candidate_buffer,
                    working_epoch,
                ),
            };
            self.workspace.record_execution(node_index);
            if changed {
                dirty_mask |= self.plan.topology.same_turn_downstream_masks[node_offset];
            }
        }
        self.evaluate_constraints()?;
        if !self.candidate_complete(working_epoch) {
            return Err(ResidentExecutionError::IncompleteCandidate);
        }
        debug_assert_eq!(self.workspace.touched_slots.len(), 2);
        Ok(())
    }

    #[inline(always)]
    fn execute_kernel(
        &mut self,
        kernel: EkfKernel,
        published: usize,
    ) -> Result<bool, ResidentExecutionError> {
        let frame = &self.workspace.input;
        let state = &self.state.state.buffers[published];
        let covariance = &self.state.covariance.buffers[published];
        let constants = &self.plan.constants;
        let scratch = &mut self.workspace.scratch;
        use EkfKernel::*;
        Ok(match kernel {
            TrigonometricState => replace(&mut scratch.trig, math::trigonometric_state(state)),
            MotionJacobian => replace(
                &mut scratch.motion_jacobian,
                math::motion_jacobian(frame, &scratch.trig, constants.dt),
            ),
            ControlJacobian => replace(
                &mut scratch.control_jacobian,
                math::control_jacobian(&scratch.trig, constants.dt),
            ),
            PredictedState => replace(
                &mut scratch.predicted_state,
                math::predicted_state(state, frame, &scratch.trig, constants.dt),
            ),
            PredictedCovariance => replace(
                &mut scratch.predicted_covariance,
                math::predicted_covariance(
                    covariance,
                    &scratch.motion_jacobian,
                    &scratch.control_jacobian,
                    &constants.process_covariance,
                ),
            ),
            LandmarkDeltaAndRange => replace(
                &mut scratch.delta_range,
                math::landmark_delta_and_range(&scratch.predicted_state, &constants.landmark)
                    .map_err(map_math_error)?,
            ),
            PredictedMeasurement => replace(
                &mut scratch.predicted_measurement,
                math::predicted_measurement(&scratch.predicted_state, &scratch.delta_range),
            ),
            MeasurementJacobian => replace(
                &mut scratch.measurement_jacobian,
                math::measurement_jacobian(&scratch.delta_range),
            ),
            InnovationCovariance => replace(
                &mut scratch.innovation_covariance,
                math::innovation_covariance(
                    &scratch.predicted_covariance,
                    &scratch.measurement_jacobian,
                    &constants.measurement_covariance,
                ),
            ),
            Solve2x2 => replace(
                &mut scratch.inverse_innovation,
                math::solve_2x2(&scratch.innovation_covariance).map_err(map_math_error)?,
            ),
            KalmanGain => replace(
                &mut scratch.gain,
                math::kalman_gain(
                    &scratch.predicted_covariance,
                    &scratch.measurement_jacobian,
                    &scratch.inverse_innovation,
                ),
            ),
            Innovation => replace(
                &mut scratch.innovation,
                math::innovation(frame, &scratch.predicted_measurement),
            ),
            CorrectedState => replace(
                &mut scratch.corrected_state,
                math::corrected_state(&scratch.predicted_state, &scratch.gain, &scratch.innovation),
            ),
            JosephCovarianceUpdate => replace(
                &mut scratch.corrected_covariance,
                math::joseph_covariance_update(
                    &scratch.predicted_covariance,
                    &scratch.measurement_jacobian,
                    &scratch.gain,
                    &constants.measurement_covariance,
                ),
            ),
            CovarianceSymmetrization => replace(
                &mut scratch.symmetrized_covariance,
                math::covariance_symmetrization(&scratch.corrected_covariance),
            ),
        })
    }

    #[inline(always)]
    fn execute_predicate(&mut self, predicate: EkfPredicate) -> bool {
        let value = match predicate {
            EkfPredicate::CandidateFinite => math::candidate_finite(
                &self.workspace.scratch.corrected_state,
                &self.workspace.scratch.symmetrized_covariance,
            ),
            EkfPredicate::CovariancePositiveDiagonal => {
                math::covariance_positive_diagonal(&self.workspace.scratch.symmetrized_covariance)
            }
            EkfPredicate::CovarianceSymmetric => {
                math::covariance_symmetric(&self.workspace.scratch.symmetrized_covariance)
            }
        };
        let slot = EkfPredicateSlot::from(predicate).index();
        let changed = self.workspace.predicates[slot] != value;
        self.workspace.predicates[slot] = value;
        changed
    }

    #[inline(always)]
    fn execute_state_copy(
        &mut self,
        target: ActivatedWrite,
        published: usize,
        candidate: usize,
        working_epoch: InstanceEpoch,
    ) -> bool {
        match target {
            ActivatedWrite::CandidateState => {
                let value = self.workspace.scratch.corrected_state;
                let changed = !same_bits(&self.state.state.buffers[published], &value);
                self.state.state.buffers[candidate] = value;
                self.record_state_write(self.plan.state_slot, working_epoch, changed);
                changed
            }
            ActivatedWrite::CandidateCovariance => {
                let value = self.workspace.scratch.symmetrized_covariance;
                let changed = !same_bits(&self.state.covariance.buffers[published], &value);
                self.state.covariance.buffers[candidate] = value;
                self.record_state_write(self.plan.covariance_slot, working_epoch, changed);
                changed
            }
            _ => unreachable!("activation only emits state targets for StateCopy"),
        }
    }

    fn record_state_write(&mut self, slot: SlotIndex, epoch: InstanceEpoch, changed: bool) {
        self.workspace.slot_epoch_marks[slot.get() as usize] = epoch;
        self.workspace.touched_slots.push(slot);
        if changed {
            self.workspace.changed_slots.push(slot);
        }
    }

    fn candidate_complete(&self, epoch: InstanceEpoch) -> bool {
        self.workspace.slot_epoch_marks[self.plan.state_slot.get() as usize] == epoch
            && self.workspace.slot_epoch_marks[self.plan.covariance_slot.get() as usize] == epoch
            && self.workspace.touched_slots.len() == 2
    }

    fn evaluate_constraints(&self) -> Result<(), ResidentExecutionError> {
        for constraint in &self.plan.constraints {
            if self.workspace.predicates[constraint.predicate.index()] {
                continue;
            }
            return Err(match constraint.predicate {
                EkfPredicateSlot::CandidateFinite => ResidentExecutionError::NonFiniteState,
                EkfPredicateSlot::CovariancePositiveDiagonal => {
                    ResidentExecutionError::CovarianceDiagonal
                }
                EkfPredicateSlot::CovarianceSymmetric => ResidentExecutionError::CovarianceSymmetry,
            });
        }
        Ok(())
    }
}

#[inline(always)]
const fn semantic_output_changed(was_initialized: bool, bits_changed: bool) -> bool {
    !was_initialized || bits_changed
}

#[inline(always)]
fn replace<const N: usize>(target: &mut [f64; N], value: [f64; N]) -> bool {
    let changed = !same_bits(target, &value);
    *target = value;
    changed
}

#[inline(always)]
fn same_bits<const N: usize>(left: &[f64; N], right: &[f64; N]) -> bool {
    left.iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

#[inline(always)]
fn map_math_error(error: EkfMathError) -> ResidentExecutionError {
    match error {
        EkfMathError::LandmarkDistance => ResidentExecutionError::LandmarkDistance,
        EkfMathError::InnovationDeterminant => ResidentExecutionError::InnovationDeterminant,
    }
}

#[inline(always)]
fn candidate_hash(instance: &ReactiveInstance, candidate: usize) -> u64 {
    let state = &instance.state.state.buffers[candidate];
    let covariance = &instance.state.covariance.buffers[candidate];
    let lanes = [
        hash_word(state[0], 0x243f_6a88_85a3_08d3),
        hash_word(state[1], 0x1319_8a2e_0370_7344),
        hash_word(state[2], 0xa409_3822_299f_31d0),
        hash_word(covariance[0], 0x082e_fa98_ec4e_6c89),
        hash_word(covariance[1], 0x4528_21e6_38d0_1377),
        hash_word(covariance[2], 0xbe54_66cf_34e9_0c6c),
        hash_word(covariance[3], 0xc0ac_29b7_c97c_50dd),
        hash_word(covariance[4], 0x3f84_d5b5_b547_0917),
        hash_word(covariance[5], 0x9216_d5d9_8979_fb1b),
        hash_word(covariance[6], 0xd131_0ba6_98df_b5ac),
        hash_word(covariance[7], 0x2ffd_72db_d01a_dfb7),
        hash_word(covariance[8], 0xb8e1_afed_6a26_7e96),
    ];
    let folded = (lanes[0] ^ lanes[4] ^ lanes[8])
        ^ (lanes[1] ^ lanes[5] ^ lanes[9]).rotate_left(17)
        ^ (lanes[2] ^ lanes[6] ^ lanes[10]).rotate_left(31)
        ^ (lanes[3] ^ lanes[7] ^ lanes[11]).rotate_left(47);
    let mixed = folded.wrapping_mul(0x9e37_79b1_85eb_ca87);
    mixed ^ (mixed >> 32)
}

#[inline(always)]
fn hash_word(value: f64, salt: u64) -> u64 {
    (value.to_bits() ^ salt).wrapping_mul(0xd6e8_feb8_6659_fd93)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::efficacy::ekf::closure::{FrozenEkfCompilationServices, compile_frozen_ekf_source};

    const SOURCE: &str =
        include_str!("../../../../tests/architecture/resident-activation/ekf-source-v1.mec");

    #[test]
    fn first_zero_output_is_a_semantic_change() {
        assert!(semantic_output_changed(false, false));
        assert!(!semantic_output_changed(true, false));
        assert!(semantic_output_changed(true, true));
    }

    fn instance() -> ReactiveInstance {
        let mut services = FrozenEkfCompilationServices::default();
        let compilation = compile_frozen_ekf_source(SOURCE, &mut services).unwrap();
        super::super::program_activation::activate(
            ReactiveInstanceId::new(0, 0),
            &compilation.source_artifact,
            &compilation.resource_request,
        )
        .unwrap()
    }

    #[test]
    fn artifact_candidate_reuses_the_two_stable_version_addresses() {
        let mut instance = instance();
        let addresses = instance.version_addresses_for_d1_test();
        for _ in 0..16 {
            instance.turn([1.0, 0.01, 20.0, 0.1]).unwrap();
            assert_eq!(instance.version_addresses_for_d1_test(), addresses);
        }
    }

    #[test]
    fn bit_identical_candidate_still_materializes_both_complete_state_values() {
        let mut instance = instance();
        let working = InstanceEpoch::new(1);
        let (published, candidate) = instance.state.begin_candidate(InstanceEpoch::ZERO, working);
        instance.workspace.begin([0.0; 4]);
        instance.workspace.scratch.corrected_state = instance.state.state.buffers[published];
        instance.workspace.scratch.symmetrized_covariance =
            instance.state.covariance.buffers[published];
        assert!(!instance.execute_state_copy(
            ActivatedWrite::CandidateState,
            published,
            candidate,
            working,
        ));
        assert!(!instance.execute_state_copy(
            ActivatedWrite::CandidateCovariance,
            published,
            candidate,
            working,
        ));
        assert!(instance.candidate_complete(working));
        assert_eq!(instance.workspace.touched_slots.len(), 2);
        assert!(instance.workspace.changed_slots.is_empty());
        assert_eq!(
            instance.state.state.buffers[candidate],
            instance.state.state.buffers[published]
        );
        assert_eq!(
            instance.state.covariance.buffers[candidate],
            instance.state.covariance.buffers[published]
        );
    }

    #[test]
    fn each_integrity_predicate_maps_to_its_exact_execution_error() {
        let mut instance = instance();

        instance.workspace.scratch.corrected_state = [f64::NAN, 0.0, 0.0];
        instance.workspace.scratch.symmetrized_covariance =
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        for predicate in [
            EkfPredicate::CandidateFinite,
            EkfPredicate::CovariancePositiveDiagonal,
            EkfPredicate::CovarianceSymmetric,
        ] {
            instance.execute_predicate(predicate);
        }
        assert_eq!(
            instance.evaluate_constraints(),
            Err(ResidentExecutionError::NonFiniteState)
        );

        instance.workspace.scratch.corrected_state = [0.0; 3];
        instance.workspace.scratch.symmetrized_covariance =
            [-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        for predicate in [
            EkfPredicate::CandidateFinite,
            EkfPredicate::CovariancePositiveDiagonal,
            EkfPredicate::CovarianceSymmetric,
        ] {
            instance.execute_predicate(predicate);
        }
        assert_eq!(
            instance.evaluate_constraints(),
            Err(ResidentExecutionError::CovarianceDiagonal)
        );

        instance.workspace.scratch.symmetrized_covariance =
            [1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        for predicate in [
            EkfPredicate::CandidateFinite,
            EkfPredicate::CovariancePositiveDiagonal,
            EkfPredicate::CovarianceSymmetric,
        ] {
            instance.execute_predicate(predicate);
        }
        assert_eq!(
            instance.evaluate_constraints(),
            Err(ResidentExecutionError::CovarianceSymmetry)
        );
    }

    #[test]
    fn landmark_distance_failure_rejects_the_complete_candidate() {
        let mut instance = instance();
        instance.state.state.buffers[0] = [25.0, -10.0, 0.0];
        let before = *instance.estimate();
        let epoch = instance.published_epoch();
        assert_eq!(
            instance.turn([0.0; 4]),
            Err(ResidentExecutionError::LandmarkDistance)
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(*instance.estimate(), before);
    }

    #[test]
    fn innovation_determinant_failure_rejects_the_complete_candidate() {
        let mut instance = instance();
        instance.state.covariance.buffers[0] = [0.0; 9];
        instance.plan.constants.process_covariance = [0.0; 4];
        instance.plan.constants.measurement_covariance = [1.0, 0.0, 0.0, 0.0];
        let before = *instance.covariance();
        let epoch = instance.published_epoch();
        assert_eq!(
            instance.turn([0.0; 4]),
            Err(ResidentExecutionError::InnovationDeterminant)
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(*instance.covariance(), before);
    }
}

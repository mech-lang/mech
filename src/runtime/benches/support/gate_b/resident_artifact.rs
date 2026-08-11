use mech_core::{ReactiveInstanceId, ResidentValueRef};
use mech_engine::__resident::{
    ActivationFacts, CapturedSignalInput, FrozenEkfCompilationServices, ReactiveInstance,
    ResidentActivationOptions, ResidentIntegrityMode, ResidentStorageClass,
    ResidentStructuralProbe, ResidentTurnSummary, ResidentValueBorrow, activate_with_options,
    compile_frozen_ekf_source, frozen_ekf_compiler_catalog,
};
use mech_runtime::__resident_recording::{GateBFixedReceipt, ResidentTurnRecorder};

use super::contract::{
    EPISODE_LENGTH, EkfInput, EkfState, assert_state_close, quantized_trajectory_hash,
    reference_trajectory, trace,
};

const SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/resident-activation/ekf-source-v1.mec"
));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactRoute {
    Source,
    Bytecode,
}

pub struct ResidentArtifactFixture {
    instance: ReactiveInstance,
    recorder: ResidentTurnRecorder,
    retained_history: usize,
    next_turn: usize,
    last_summary: Option<ResidentTurnSummary>,
}

pub struct ResidentArtifactKernelFixture {
    instance: ReactiveInstance,
    last_summary: Option<ResidentTurnSummary>,
}

fn activate_route(
    route: ArtifactRoute,
    next_epoch: u64,
    integrity: ResidentIntegrityMode,
) -> ReactiveInstance {
    let mut services = FrozenEkfCompilationServices::default();
    let compilation = compile_frozen_ekf_source(SOURCE, &mut services)
        .expect("compile frozen ordinary EKF source before timing");
    let artifact = match route {
        ArtifactRoute::Source => &compilation.source_artifact,
        ArtifactRoute::Bytecode => &compilation.decoded_artifact,
    };
    let catalog = frozen_ekf_compiler_catalog().expect("build frozen resident kernel catalog");
    let mut instance = activate_with_options(
        ReactiveInstanceId::new(0, 0),
        artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions { integrity },
    )
    .expect("activate frozen public ProgramArtifact before timing");
    instance.set_next_epoch_for_test(next_epoch);
    instance
}

impl ResidentArtifactFixture {
    pub fn new(route: ArtifactRoute) -> Self {
        Self::with_controls(route, 0, 1)
    }

    pub fn with_controls(route: ArtifactRoute, retained_history: usize, next_epoch: u64) -> Self {
        Self {
            instance: activate_route(route, next_epoch, ResidentIntegrityMode::Checked),
            recorder: ResidentTurnRecorder::new(EPISODE_LENGTH, retained_history)
                .expect("resident artifact recorder setup"),
            retained_history,
            next_turn: 0,
            last_summary: None,
        }
    }

    #[inline]
    fn run_turn(&mut self, input: EkfInput) {
        let permit = self
            .recorder
            .take_admission_permit(self.next_turn)
            .expect("pre-reserved artifact admission");
        let before_epoch = self.instance.published_epoch().get();
        let frame = input_array(input);
        let captured = captured_input(&self.instance, &frame);
        match self.instance.prepare_turn(&[captured]) {
            Ok(prepared) => {
                let summary = prepared.summary();
                let commit = self
                    .recorder
                    .prepare_artifact_commit(permit, prepared)
                    .expect("artifact receipt preparation");
                commit.commit();
                self.last_summary = Some(summary);
            }
            Err(failure) => {
                self.recorder
                    .prepare_artifact_rejected(permit, before_epoch, failure.clone())
                    .expect("artifact rejection preparation")
                    .append();
                panic!("frozen artifact resident turn failed: {failure:?}");
            }
        }
        self.next_turn += 1;
    }

    pub fn run_episode(&mut self) {
        for input in trace().iter().copied() {
            self.run_turn(input);
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
            self.run_turn(input);
            let actual = self.state();
            assert_state_close(actual, expected, turn + 1);
            trajectory.push(actual);
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn state(&self) -> EkfState {
        artifact_state(&self.instance)
    }

    pub fn validate_final(&self) {
        assert_state_close(self.state(), EkfState::REFERENCE_FINAL, EPISODE_LENGTH);
        assert_eq!(
            self.recorder.recorded_ledger_len(),
            self.retained_history + self.next_turn
        );
    }

    pub fn probe(&self) -> ResidentArtifactProbe {
        ResidentArtifactProbe::from_execution(
            self.instance.structural_probe(),
            self.last_summary,
            self.retained_history,
            self.next_turn,
            self.recorder.records_inspected(),
        )
    }

    pub fn abort_output_hash(&mut self) -> String {
        let before = self.state();
        let frame = input_array(trace()[0]);
        let captured = captured_input(&self.instance, &frame);
        self.instance
            .execute_then_abort(&[captured])
            .expect("valid candidate before forced artifact abort");
        assert_eq!(self.state(), before);
        quantized_trajectory_hash(&[before])
    }
}

impl ResidentArtifactKernelFixture {
    pub fn new(route: ArtifactRoute) -> Self {
        Self::with_integrity(route, ResidentIntegrityMode::Checked)
    }

    pub fn with_integrity(route: ArtifactRoute, integrity: ResidentIntegrityMode) -> Self {
        Self {
            instance: activate_route(route, 1, integrity),
            last_summary: None,
        }
    }

    #[inline]
    fn run_turn(&mut self, input: EkfInput) {
        let frame = input_array(input);
        let captured = captured_input(&self.instance, &frame);
        self.instance
            .turn_without_summary(&[captured])
            .expect("artifact kernel candidate");
    }

    pub fn run_episode(&mut self) {
        for input in trace().iter().copied() {
            self.run_turn(input);
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
            self.run_turn(input);
            let actual = self.state();
            assert_state_close(actual, expected, turn + 1);
            trajectory.push(actual);
        }
        quantized_trajectory_hash(&trajectory)
    }

    pub fn state(&self) -> EkfState {
        artifact_state(&self.instance)
    }

    pub fn validate_final(&self) {
        assert_state_close(self.state(), EkfState::REFERENCE_FINAL, EPISODE_LENGTH);
    }

    pub fn probe(&self) -> ResidentArtifactProbe {
        let mut probe = ResidentArtifactProbe::from_execution(
            self.instance.structural_probe(),
            self.last_summary,
            0,
            0,
            0,
        );
        probe.dirty_nodes = self.instance.plan.execution_node_count();
        probe.post_publication_append_infallible = false;
        probe
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentArtifactProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_written_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
    pub receipt_bytes: usize,
    pub dirty_nodes: usize,
    pub commit_runtime_call_count: usize,
    pub legacy_journal_capture_count: usize,
    pub record_preparation_count: usize,
    pub record_append_count: usize,
    pub records_retained_before_timing: usize,
    pub records_appended: usize,
    pub ledger_records_inspected: usize,
    pub post_publication_append_infallible: bool,
}

impl ResidentArtifactProbe {
    fn from_execution(
        probe: ResidentStructuralProbe,
        summary: Option<ResidentTurnSummary>,
        retained_history: usize,
        records_appended: usize,
        records_inspected: usize,
    ) -> Self {
        Self {
            candidate_seed_bytes: probe.candidate_seed_bytes,
            candidate_written_bytes: probe.candidate_materialized_bytes,
            published_buffer_copy_bytes: probe.published_buffer_copy_bytes,
            publication_store_count: probe.publication_store_count,
            receipt_bytes: GateBFixedReceipt::RETAINED_BYTES,
            dirty_nodes: summary.map_or(0, |summary| summary.dirty_nodes as usize),
            commit_runtime_call_count: probe.commit_runtime_call_count,
            legacy_journal_capture_count: probe.legacy_journal_capture_count,
            record_preparation_count: usize::from(records_appended != 0),
            record_append_count: usize::from(records_appended != 0),
            records_retained_before_timing: retained_history,
            records_appended,
            ledger_records_inspected: records_inspected,
            post_publication_append_infallible: true,
        }
    }
}

fn input_array(input: EkfInput) -> [f64; 4] {
    [
        input.velocity,
        input.angular_velocity,
        input.measured_range,
        input.measured_bearing,
    ]
}

fn captured_input<'a>(instance: &ReactiveInstance, input: &'a [f64; 4]) -> CapturedSignalInput<'a> {
    CapturedSignalInput {
        slot: instance.plan.inputs[0].slot,
        value: ResidentValueRef::F64(input),
    }
}

fn artifact_state(instance: &ReactiveInstance) -> EkfState {
    let mut state = [0.0; 3];
    let mut covariance = [0.0; 9];
    for slot in instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        let ResidentValueBorrow::F64 { values, .. } =
            instance.state_borrow(slot.artifact_id).expect("state slot")
        else {
            panic!("EKF resident state is f64")
        };
        match values.len() {
            3 => state.copy_from_slice(values),
            9 => covariance.copy_from_slice(values),
            _ => panic!("unexpected EKF resident state shape"),
        }
    }
    EkfState { state, covariance }
}

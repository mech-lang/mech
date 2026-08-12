#![allow(clippy::result_large_err)]

mod aot;
mod kernel_ir;

use mech_core::matrix::Matrix as MechMatrix;
use mech_core::{
    ExecutionHostFunctionRequest, ExecutionResourceRequest, GenericError, LegacyValue, MResult,
    MechError, MechExecutionServices, ReactiveInstanceId, Ref, ResidentValueRef,
    ResolvedOperationContract, SlotIndex, ValRef,
};
use mech_engine::__resident::{
    ActivationFacts, CapturedSignalInput, ReactiveInstance, ResidentStorageClass,
    ResidentValueBorrow, activate,
};
use mech_engine::{MechProgram, MechProgramConfig};
use nalgebra::{Matrix2, Matrix3, Matrix3x2, Vector3};
use std::collections::BTreeMap;
use std::f64::consts::TAU;
use std::hint::black_box;
use std::time::{Duration, Instant};

use aot::{AotProgram, AotState};

const SOURCE: &str = include_str!("../../ekf-function-high-level.mec");
const NUMERIC_PROOF_SOURCE: &str = include_str!("../../numeric-kernel-proof.mec");
const NUMERIC_BATCH_PROOF_SOURCE: &str = include_str!("../../numeric-batch-proof.mec");
const NUMERIC_BATCH_LEN: usize = 64;
const BATCH_EKF_LEN: usize = 1_024;
const BATCH_VALIDATION_TURNS: usize = 256;
const BATCH_INPUT_PERIOD: usize = 256;
const INPUT_PERIOD: usize = 4_096;
const DT: f64 = 0.1;
const LANDMARK_X: f64 = 140.0;
const LANDMARK_Y: f64 = 12.0;
const MEASUREMENT_NOISE: f64 = 0.25;
const SAMPLE_COUNT: usize = 9;
const TARGET_SAMPLE: Duration = Duration::from_millis(75);
const WARMUP_MIN: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug)]
struct InputSample {
    linear_velocity: f64,
    angular_velocity: f64,
    bearing: f64,
}

fn wrap_angle(angle: f64) -> f64 {
    angle.sin().atan2(angle.cos())
}

fn input_samples() -> Vec<InputSample> {
    let mut truth = Vector3::<f64>::new(45.0, 15.0, 0.0);
    let mut samples = Vec::with_capacity(INPUT_PERIOD);
    let base_angular_velocity = TAU / (INPUT_PERIOD as f64 * DT);
    for index in 0..INPUT_PERIOD {
        let phase = TAU * index as f64 / INPUT_PERIOD as f64;
        let linear_velocity = 1.0 + 0.05 * (phase * 3.0).sin();
        let angular_velocity = base_angular_velocity * (1.0 + 0.1 * (phase * 2.0).cos());
        truth[0] += linear_velocity * truth[2].cos() * DT;
        truth[1] += linear_velocity * truth[2].sin() * DT;
        truth[2] = wrap_angle(truth[2] + angular_velocity * DT);
        let noise = 0.01 * (phase * 7.0).sin() + 0.005 * (phase * 11.0).cos();
        let bearing =
            wrap_angle((LANDMARK_Y - truth[1]).atan2(LANDMARK_X - truth[0]) - truth[2] + noise);
        samples.push(InputSample {
            linear_velocity,
            angular_velocity,
            bearing,
        });
    }
    samples
}

#[derive(Clone)]
struct ReferenceEkf {
    state: Vector3<f64>,
    covariance: Matrix3<f64>,
}

impl ReferenceEkf {
    fn new() -> Self {
        Self {
            state: Vector3::new(55.0, 25.0, 0.4),
            covariance: Matrix3::new(100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15),
        }
    }

    fn turn(&mut self, input: InputSample) {
        let theta = self.state[2];
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let distance = input.linear_velocity * DT;
        let predicted_state = self.state
            + Vector3::new(
                distance * cos_theta,
                distance * sin_theta,
                input.angular_velocity * DT,
            );
        let motion_jacobian = Matrix3::new(
            1.0,
            0.0,
            -distance * sin_theta,
            0.0,
            1.0,
            distance * cos_theta,
            0.0,
            0.0,
            1.0,
        );
        let control_jacobian = Matrix3x2::new(cos_theta * DT, 0.0, sin_theta * DT, 0.0, 0.0, DT);
        let process_noise = Matrix2::new(0.01, 0.0, 0.0, 0.0025);
        let predicted_covariance = motion_jacobian * self.covariance * motion_jacobian.transpose()
            + control_jacobian * process_noise * control_jacobian.transpose();

        let delta_x = LANDMARK_X - predicted_state[0];
        let delta_y = LANDMARK_Y - predicted_state[1];
        let squared_range = delta_x * delta_x + delta_y * delta_y;
        let predicted_bearing = delta_y.atan2(delta_x) - predicted_state[2];
        let innovation = wrap_angle(input.bearing - predicted_bearing);
        let observation_jacobian =
            nalgebra::RowVector3::new(delta_y / squared_range, -delta_x / squared_range, -1.0);
        let innovation_variance = (observation_jacobian
            * predicted_covariance
            * observation_jacobian.transpose())[(0, 0)]
            + MEASUREMENT_NOISE;
        let gain = predicted_covariance * observation_jacobian.transpose() / innovation_variance;

        self.state = predicted_state + gain * innovation;
        let correction = Matrix3::identity() - gain * observation_jacobian;
        self.covariance = correction * predicted_covariance * correction.transpose()
            + gain * gain.transpose() * MEASUREMENT_NOISE;
    }

    fn check(&self) -> f64 {
        self.state.sum() + self.covariance.trace()
    }
}

#[derive(Debug, Default)]
struct CompilationServices {
    values: BTreeMap<String, LegacyValue>,
    bindings: Vec<(ExecutionResourceRequest, ValRef)>,
}

impl CompilationServices {
    fn new(values: impl IntoIterator<Item = (&'static str, f64)>) -> Self {
        Self::with_values(
            values
                .into_iter()
                .map(|(path, value)| (path, LegacyValue::F64(Ref::new(value)))),
        )
    }

    fn with_values(values: impl IntoIterator<Item = (&'static str, LegacyValue)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(path, value)| (path.to_string(), value))
                .collect(),
            bindings: Vec::new(),
        }
    }

    fn value(&self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        let value = self
            .values
            .get(&request.path)
            .cloned()
            .ok_or_else(|| error(format!("unknown input path {}", request.path)))?;
        Ok(value)
    }
}

impl MechExecutionServices for CompilationServices {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        _arguments: &[LegacyValue],
    ) -> MResult<LegacyValue> {
        Err(error(format!("unexpected host function {request:?}")))
    }

    fn plan_resource_read_output(
        &mut self,
        request: &ExecutionResourceRequest,
    ) -> MResult<LegacyValue> {
        self.value(request)
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        self.value(request)
    }

    fn write_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        _value: &LegacyValue,
    ) -> MResult<()> {
        Err(error(format!("unexpected resource write {request:?}")))
    }

    fn bind_live_resource(
        &mut self,
        _interpreter_id: u64,
        request: &ExecutionResourceRequest,
        target: ValRef,
    ) -> MResult<()> {
        self.bindings.push((request.clone(), target));
        Ok(())
    }
}

fn error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}

#[derive(Clone, Copy)]
struct ResidentInputs {
    pulse: SlotIndex,
    linear_velocity: SlotIndex,
    angular_velocity: SlotIndex,
    bearing: SlotIndex,
}

impl ResidentInputs {
    fn from_instance(instance: &ReactiveInstance) -> Self {
        assert_eq!(instance.plan.inputs.len(), 4);
        Self {
            pulse: instance.plan.inputs[0].slot,
            linear_velocity: instance.plan.inputs[1].slot,
            angular_velocity: instance.plan.inputs[2].slot,
            bearing: instance.plan.inputs[3].slot,
        }
    }
}

fn resident_turn(
    instance: &mut ReactiveInstance,
    slots: ResidentInputs,
    turn: usize,
    sample: InputSample,
    summary: bool,
) {
    let pulse = [(turn + 1) as f64];
    let linear_velocity = [sample.linear_velocity];
    let angular_velocity = [sample.angular_velocity];
    let bearing = [sample.bearing];
    let inputs = [
        CapturedSignalInput {
            slot: slots.pulse,
            value: ResidentValueRef::F64(&pulse),
        },
        CapturedSignalInput {
            slot: slots.linear_velocity,
            value: ResidentValueRef::F64(&linear_velocity),
        },
        CapturedSignalInput {
            slot: slots.angular_velocity,
            value: ResidentValueRef::F64(&angular_velocity),
        },
        CapturedSignalInput {
            slot: slots.bearing,
            value: ResidentValueRef::F64(&bearing),
        },
    ];
    if summary {
        black_box(instance.turn(&inputs).expect("resident turn"));
    } else {
        instance
            .turn_without_summary(&inputs)
            .expect("resident turn without summary");
    }
}

fn resident_state(instance: &ReactiveInstance) -> [f64; 12] {
    let mut result = [0.0; 12];
    for slot in instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        let ResidentValueBorrow::F64 { values, .. } =
            instance.state_borrow(slot.artifact_id).expect("state slot")
        else {
            panic!("EKF state must contain f64 values")
        };
        match values.len() {
            3 => result[..3].copy_from_slice(values),
            9 => result[3..].copy_from_slice(values),
            len => panic!("unexpected EKF state length {len}"),
        }
    }
    result
}

fn validate_resident(instance: &mut ReactiveInstance, samples: &[InputSample]) {
    let slots = ResidentInputs::from_instance(instance);
    let mut reference = ReferenceEkf::new();
    let mut maximum_error = 0.0_f64;
    for (turn, sample) in samples.iter().copied().enumerate() {
        reference.turn(sample);
        resident_turn(instance, slots, turn, sample, true);
        let actual = resident_state(instance);
        let expected_state = reference.state.as_slice();
        let expected_covariance = reference.covariance.as_slice();
        let state_error = actual[..3]
            .iter()
            .zip(expected_state)
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0, f64::max);
        let covariance_error = actual[3..]
            .iter()
            .zip(expected_covariance)
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0, f64::max);
        maximum_error = maximum_error.max(state_error).max(covariance_error);
        assert!(
            state_error < 1.0e-8 && covariance_error < 1.0e-8,
            "turn {turn}: state error {state_error:e}, covariance error {covariance_error:e}; actual={actual:?}; expected state={expected_state:?}, covariance={expected_covariance:?}",
        );
    }
    assert!(
        maximum_error < 1.0e-8,
        "resident EKF diverged from fixed-size nalgebra reference: {maximum_error:e}",
    );
    println!(
        "validated {} turns; maximum absolute error {maximum_error:.3e}",
        samples.len()
    );
}

struct Measurement {
    median_ns: f64,
    min_ns: f64,
    max_ns: f64,
    batch_iterations: usize,
}

fn measure(mut operation: impl FnMut()) -> Measurement {
    let warmup_start = Instant::now();
    let mut warmup_iterations = 0;
    while warmup_iterations < 2 || warmup_start.elapsed() < WARMUP_MIN {
        operation();
        warmup_iterations += 1;
    }
    let calibration = Instant::now();
    operation();
    let per_iteration = calibration.elapsed().as_secs_f64().max(1.0e-9);
    let batch_iterations = (TARGET_SAMPLE.as_secs_f64() / per_iteration)
        .ceil()
        .clamp(1.0, 1_000_000.0) as usize;
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            operation();
        }
        samples.push(start.elapsed().as_secs_f64() * 1.0e9 / batch_iterations as f64);
    }
    samples.sort_by(f64::total_cmp);
    Measurement {
        median_ns: samples[SAMPLE_COUNT / 2],
        min_ns: samples[0],
        max_ns: samples[SAMPLE_COUNT - 1],
        batch_iterations,
    }
}

fn print_measurement(name: &str, measurement: &Measurement, check: f64) {
    println!(
        "{name:28} {:10.1} ns/turn  {:10.3} kHz  min {:8.1}  max {:8.1}  batch {:>7}  check {:.9}",
        measurement.median_ns,
        1.0e6 / measurement.median_ns,
        measurement.min_ns,
        measurement.max_ns,
        measurement.batch_iterations,
        check,
    );
}

fn benchmark_raw(samples: &[InputSample]) {
    let mut filter = ReferenceEkf::new();
    let mut turn = 0usize;
    let measurement = measure(|| {
        filter.turn(samples[turn % samples.len()]);
        turn += 1;
    });
    print_measurement(
        "raw Rust fixed nalgebra",
        &measurement,
        black_box(filter.check()),
    );
}

fn benchmark_resident(
    name: &str,
    instance: &mut ReactiveInstance,
    samples: &[InputSample],
    summary: bool,
) {
    let slots = ResidentInputs::from_instance(instance);
    let mut turn = 0usize;
    let measurement = measure(|| {
        resident_turn(
            instance,
            slots,
            turn,
            samples[turn % samples.len()],
            summary,
        );
        turn += 1;
    });
    let state = resident_state(instance);
    let check = state[..3].iter().sum::<f64>() + state[3] + state[7] + state[11];
    print_measurement(name, &measurement, black_box(check));
}

fn validate_aot(program: &AotProgram, samples: &[InputSample]) {
    let mut state = AotState::new(program);
    let mut reference = ReferenceEkf::new();
    let mut maximum_error = 0.0_f64;
    for (turn, sample) in samples.iter().copied().enumerate() {
        reference.turn(sample);
        program.turn(
            &mut state,
            &[
                (turn + 1) as f64,
                sample.linear_velocity,
                sample.angular_velocity,
                sample.bearing,
            ],
        );
        let expected = reference
            .state
            .as_slice()
            .iter()
            .chain(reference.covariance.as_slice())
            .copied();
        let error = state
            .values()
            .iter()
            .copied()
            .zip(expected)
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0, f64::max);
        maximum_error = maximum_error.max(error);
        assert!(error < 1.0e-8, "AOT mismatch at turn {turn}: {error:e}");
    }
    println!(
        "validated generated AOT for {} turns; maximum absolute error {:.3e}",
        samples.len(),
        maximum_error,
    );
}

fn benchmark_aot(program: &AotProgram, samples: &[InputSample]) {
    let mut state = AotState::new(program);
    let mut turn = 0usize;
    let measurement = measure(|| {
        let sample = samples[turn % samples.len()];
        program.turn(
            &mut state,
            &[
                (turn + 1) as f64,
                sample.linear_velocity,
                sample.angular_velocity,
                sample.bearing,
            ],
        );
        turn += 1;
    });
    let values = state.values();
    let check = values[..3].iter().sum::<f64>() + values[3] + values[7] + values[11];
    print_measurement("Mech AOT generated Rust", &measurement, black_box(check));
}

fn prove_general_numeric_kernel(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
) -> MResult<AotProgram> {
    let mut services = CompilationServices::new([("pulse", 0.0), ("drive", 0.0)]);
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), catalog.clone());
    program.run_string_with_services(NUMERIC_PROOF_SOURCE, &mut services)?;
    let (artifact, _bytecode) = program.compile_program_product()?.into_parts();
    let instance = activate(
        ReactiveInstanceId::new(3, 0),
        &artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .map_err(|resident_error| {
        error(format!(
            "numeric proof resident activation failed: {resident_error:?}"
        ))
    })?;
    let input_slots = instance
        .plan
        .inputs
        .iter()
        .map(|input| mech_core::CellSlotId::new(input.slot.get()))
        .collect::<Vec<_>>();
    let aot = AotProgram::build(&artifact, &instance.plan, &input_slots)
        .map_err(|message| error(format!("numeric proof AOT build failed: {message}")))?;

    let input_paths = services
        .bindings
        .iter()
        .map(|(request, _)| request.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(input_paths.len(), 2);
    let mut state = AotState::new(&aot);
    let mut expected = [1.0_f64, -2.0];
    let mut maximum_error = 0.0_f64;
    for turn in 0..INPUT_PERIOD {
        let drive = (TAU * turn as f64 / 127.0).sin();
        let inputs = input_paths
            .iter()
            .map(|path| match *path {
                "pulse" => (turn + 1) as f64,
                "drive" => drive,
                path => panic!("unexpected numeric proof input {path}"),
            })
            .collect::<Vec<_>>();
        let previous = expected;
        expected = [
            0.999 * previous[0] + 0.01 * previous[1] + 0.005 * drive,
            -0.02 * previous[0] + 0.998 * previous[1] - 0.003 * drive,
        ];
        aot.turn(&mut state, &inputs);
        let error = state
            .values()
            .iter()
            .zip(expected)
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0, f64::max);
        maximum_error = maximum_error.max(error);
        assert!(
            error < 1.0e-12,
            "numeric proof mismatch at turn {turn}: {error:e}"
        );
    }
    println!(
        "general numeric-kernel proof: {} Mech nodes -> {} kernel instructions; {} turns, maximum absolute error {:.3e}",
        artifact.nodes().len(),
        aot.instruction_count(),
        INPUT_PERIOD,
        maximum_error,
    );
    println!(
        "  generated independent proof: {} (rustc -O: {:.1} ms)",
        aot.source_path().display(),
        aot.compile_time().as_secs_f64() * 1.0e3,
    );
    Ok(aot)
}

fn prove_batched_numeric_kernel(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
) -> MResult<AotProgram> {
    let drive = LegacyValue::MatrixF64(MechMatrix::from_vec(
        vec![0.0; NUMERIC_BATCH_LEN],
        1,
        NUMERIC_BATCH_LEN,
    ));
    let mut services = CompilationServices::with_values([
        ("pulse", LegacyValue::F64(Ref::new(0.0))),
        ("drive", drive),
    ]);
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), catalog.clone());
    program.run_string_with_services(NUMERIC_BATCH_PROOF_SOURCE, &mut services)?;
    let (artifact, _bytecode) = program.compile_program_product()?.into_parts();
    let instance = activate(
        ReactiveInstanceId::new(4, 0),
        &artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .map_err(|resident_error| {
        let operations = artifact
            .nodes()
            .iter()
            .map(|node| {
                format!(
                    "{}/{}",
                    node.operation.module_path.join("/"),
                    node.operation.operation_name,
                )
            })
            .collect::<Vec<_>>();
        error(format!(
            "batched numeric resident activation failed: {resident_error:?}; operations={operations:?}"
        ))
    })?;
    let input_slots = instance
        .plan
        .inputs
        .iter()
        .map(|input| mech_core::CellSlotId::new(input.slot.get()))
        .collect::<Vec<_>>();
    let aot = AotProgram::build(&artifact, &instance.plan, &input_slots)
        .map_err(|message| error(format!("batched numeric AOT build failed: {message}")))?;
    assert_eq!(aot.batch_len(), Some(NUMERIC_BATCH_LEN));

    let input_paths = services
        .bindings
        .iter()
        .map(|(request, _)| request.path.as_str())
        .collect::<Vec<_>>();
    let mut state = AotState::new(&aot);
    let mut expected = vec![1.0_f64; NUMERIC_BATCH_LEN];
    let mut maximum_error = 0.0_f64;
    for turn in 0..INPUT_PERIOD {
        let drives = (0..NUMERIC_BATCH_LEN)
            .map(|lane| ((turn * 17 + lane * 13) as f64 * 0.001).sin() * 0.01)
            .collect::<Vec<_>>();
        let mut inputs = Vec::with_capacity(NUMERIC_BATCH_LEN + 1);
        for path in &input_paths {
            match *path {
                "pulse" => inputs.push((turn + 1) as f64),
                "drive" => inputs.extend_from_slice(&drives),
                path => panic!("unexpected batched numeric input {path}"),
            }
        }
        for (expected, drive) in expected.iter_mut().zip(&drives) {
            *expected = *expected * 0.999 + drive;
        }
        aot.turn(&mut state, &inputs);
        let turn_error = state
            .values()
            .iter()
            .zip(&expected)
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0, f64::max);
        maximum_error = maximum_error.max(turn_error);
        assert!(
            turn_error < 1.0e-12,
            "batched numeric mismatch at turn {turn}: {turn_error:e}"
        );
    }
    println!(
        "batch-kernel plumbing proof: {} lanes, {} instructions, maximum absolute error {:.3e}",
        NUMERIC_BATCH_LEN,
        aot.instruction_count(),
        maximum_error,
    );
    Ok(aot)
}

fn batch_sample(sample: InputSample, lane: usize) -> InputSample {
    let centered_lane = lane as f64 - (BATCH_EKF_LEN as f64 - 1.0) * 0.5;
    let phase = centered_lane * (TAU / BATCH_EKF_LEN as f64);
    InputSample {
        linear_velocity: sample.linear_velocity * (1.0 + phase.sin() * 0.02),
        angular_velocity: sample.angular_velocity * (1.0 + phase.cos() * 0.03),
        bearing: wrap_angle(sample.bearing + phase.sin() * 0.002),
    }
}

fn batch_input_frame(
    paths: &[&str],
    turn: usize,
    sample: InputSample,
) -> (Vec<f64>, Vec<InputSample>) {
    let lane_samples = (0..BATCH_EKF_LEN)
        .map(|lane| batch_sample(sample, lane))
        .collect::<Vec<_>>();
    let mut inputs = Vec::with_capacity(1 + BATCH_EKF_LEN * 3);
    for path in paths {
        match *path {
            "pulse" => inputs.push((turn + 1) as f64),
            "linear-velocity" => {
                inputs.extend(lane_samples.iter().map(|sample| sample.linear_velocity))
            }
            "angular-velocity" => {
                inputs.extend(lane_samples.iter().map(|sample| sample.angular_velocity))
            }
            "bearing" => inputs.extend(lane_samples.iter().map(|sample| sample.bearing)),
            path => panic!("unexpected batched EKF input {path}"),
        }
    }
    (inputs, lane_samples)
}

fn validate_batched_ekf(program: &AotProgram, input_paths: &[&str], samples: &[InputSample]) {
    let mut state = AotState::new(program);
    let mut references = vec![ReferenceEkf::new(); BATCH_EKF_LEN];
    let mut maximum_error = 0.0_f64;
    for turn in 0..BATCH_VALIDATION_TURNS {
        let (inputs, lane_samples) =
            batch_input_frame(input_paths, turn, samples[turn % samples.len()]);
        for (reference, sample) in references.iter_mut().zip(lane_samples) {
            reference.turn(sample);
        }
        program.turn(&mut state, &inputs);
        assert_eq!(state.values().len(), 12 * BATCH_EKF_LEN);
        for (lane, reference) in references.iter().enumerate() {
            let expected = reference
                .state
                .as_slice()
                .iter()
                .chain(reference.covariance.as_slice());
            let lane_error = expected
                .enumerate()
                .map(|(component, expected)| {
                    (state.values()[component * BATCH_EKF_LEN + lane] - expected).abs()
                })
                .fold(0.0, f64::max);
            maximum_error = maximum_error.max(lane_error);
            assert!(
                lane_error < 1.0e-8,
                "batched EKF mismatch at turn {turn}, lane {lane}: {lane_error:e}",
            );
        }
    }
    println!(
        "validated generated batch EKF: {} lanes x {} turns; maximum absolute error {:.3e}",
        BATCH_EKF_LEN, BATCH_VALIDATION_TURNS, maximum_error,
    );
}

fn benchmark_batched_ekf(program: &AotProgram, input_paths: &[&str], samples: &[InputSample]) {
    let input_frames = (0..BATCH_INPUT_PERIOD)
        .map(|turn| batch_input_frame(input_paths, turn, samples[turn % samples.len()]).0)
        .collect::<Vec<_>>();
    let mut state = AotState::new(program);
    let mut turn = 0usize;
    let measurement = measure(|| {
        program.turn(&mut state, &input_frames[turn % input_frames.len()]);
        turn += 1;
    });
    let per_filter_ns = measurement.median_ns / BATCH_EKF_LEN as f64;
    let check = (0..12)
        .map(|component| state.values()[component * BATCH_EKF_LEN])
        .sum::<f64>();
    println!(
        "Mech AOT batch {BATCH_EKF_LEN:>5} {:10.1} ns/batch  {:8.3} ns/filter  {:10.3} M filter-turns/s  min {:8.1}  max {:8.1}  check {:.9}",
        measurement.median_ns,
        per_filter_ns,
        1.0e3 / per_filter_ns,
        measurement.min_ns,
        measurement.max_ns,
        black_box(check),
    );
}

fn benchmark_raw_batched_ekf(samples: &[InputSample]) {
    let input_frames = (0..BATCH_INPUT_PERIOD)
        .map(|turn| {
            (0..BATCH_EKF_LEN)
                .map(|lane| batch_sample(samples[turn % samples.len()], lane))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut filters = vec![ReferenceEkf::new(); BATCH_EKF_LEN];
    let mut turn = 0usize;
    let measurement = measure(|| {
        for (filter, sample) in filters
            .iter_mut()
            .zip(&input_frames[turn % input_frames.len()])
        {
            filter.turn(*sample);
        }
        turn += 1;
    });
    let per_filter_ns = measurement.median_ns / BATCH_EKF_LEN as f64;
    let check = filters.iter().map(ReferenceEkf::check).sum::<f64>();
    println!(
        "raw Rust batch {BATCH_EKF_LEN:>6} {:10.1} ns/batch  {:8.3} ns/filter  {:10.3} M filter-turns/s  min {:8.1}  max {:8.1}  check {:.9}",
        measurement.median_ns,
        per_filter_ns,
        1.0e3 / per_filter_ns,
        measurement.min_ns,
        measurement.max_ns,
        black_box(check),
    );
}

fn prove_batched_ekf_kernel(
    catalog: &std::sync::Arc<mech_core::FunctionCatalog>,
    samples: &[InputSample],
) -> MResult<AotProgram> {
    let mut services = CompilationServices::new([
        ("pulse", 0.0),
        ("linear-velocity", 1.0),
        ("angular-velocity", 0.01),
        ("bearing", -0.25),
    ]);
    let frontend_started = Instant::now();
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), catalog.clone());
    program.run_string_with_services(SOURCE, &mut services)?;
    let (artifact, _bytecode) = program.compile_program_product()?.into_parts();
    let frontend_time = frontend_started.elapsed();
    let instance = activate(
        ReactiveInstanceId::new(5, 0),
        &artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .map_err(|resident_error| {
        let operations = artifact
            .nodes()
            .iter()
            .map(|node| {
                format!(
                    "{}/{}",
                    node.operation.module_path.join("/"),
                    node.operation.operation_name,
                )
            })
            .collect::<Vec<_>>();
        error(format!(
            "batched EKF resident activation failed: {resident_error:?}; operations={operations:?}"
        ))
    })?;
    let input_slots = instance
        .plan
        .inputs
        .iter()
        .map(|input| mech_core::CellSlotId::new(input.slot.get()))
        .collect::<Vec<_>>();
    let per_lane_inputs = services
        .bindings
        .iter()
        .enumerate()
        .filter_map(|(ordinal, (request, _))| (request.path != "pulse").then_some(ordinal))
        .collect::<Vec<_>>();
    let aot = AotProgram::build_outer_lifted(
        &artifact,
        &instance.plan,
        &input_slots,
        BATCH_EKF_LEN,
        &per_lane_inputs,
    )
    .map_err(|message| error(format!("batched EKF AOT build failed: {message}")))?;
    assert_eq!(aot.batch_len(), Some(BATCH_EKF_LEN));
    let input_paths = services
        .bindings
        .iter()
        .map(|(request, _)| request.path.as_str())
        .collect::<Vec<_>>();
    println!(
        "compiled one natural matrix EKF graph and outer-lifted it: {} Mech nodes -> {} kernel instructions, {} lanes ({:.1} ms frontend, {:.1} ms rustc -O)",
        artifact.nodes().len(),
        aot.instruction_count(),
        BATCH_EKF_LEN,
        frontend_time.as_secs_f64() * 1.0e3,
        aot.compile_time().as_secs_f64() * 1.0e3,
    );
    println!("  generated batch kernel: {}", aot.source_path().display());
    validate_batched_ekf(&aot, &input_paths, samples);
    benchmark_raw_batched_ekf(samples);
    benchmark_batched_ekf(&aot, &input_paths, samples);
    Ok(aot)
}

fn main() -> MResult<()> {
    let frontend_started = Instant::now();
    let catalog = mech_stdlib::source_catalog();
    let mut services = CompilationServices::new([
        ("pulse", 0.0),
        ("linear-velocity", 1.0),
        ("angular-velocity", 0.01),
        ("bearing", -0.25),
    ]);
    let mut program =
        MechProgram::with_function_catalog(MechProgramConfig::default(), catalog.clone());
    program.run_string_with_services(SOURCE, &mut services)?;
    let (artifact, _bytecode) = program.compile_program_product()?.into_parts();
    let frontend_time = frontend_started.elapsed();

    let mut operations = BTreeMap::<String, usize>::new();
    for node in artifact.nodes() {
        let operation = if node.operation.module_path.is_empty() {
            node.operation.operation_name.clone()
        } else {
            format!(
                "{}/{}",
                node.operation.module_path.join("/"),
                node.operation.operation_name
            )
        };
        *operations.entry(operation).or_default() += 1;
    }
    println!(
        "compiled high-level EKF: {} nodes, {} slots, {} live inputs ({:.1} ms frontend + artifact)",
        artifact.nodes().len(),
        artifact.slots().len(),
        services.bindings.len(),
        frontend_time.as_secs_f64() * 1.0e3,
    );
    for (ordinal, input) in artifact.inputs().iter().enumerate() {
        println!(
            "  input {ordinal}: name={} artifact-slot={}",
            input.name,
            input.slot.get(),
        );
    }
    for (operation, count) in operations {
        println!("{count:>3}  {operation}");
    }
    if std::env::var_os("MECH_EKF_DUMP_ARTIFACT").is_some() {
        for node in artifact.nodes() {
            println!(
                "node {}: {}/{} inputs={:?} outputs={:?} contract={:?}",
                node.node.get(),
                node.operation.module_path.join("/"),
                node.operation.operation_name,
                node.input_bindings,
                node.output_bindings,
                artifact.contracts().get(node.contract),
            );
        }
    }
    for node in artifact.nodes() {
        if matches!(
            artifact.contracts().get(node.contract),
            Some(ResolvedOperationContract::LegacyOpaque(_))
        ) {
            println!(
                "legacy-opaque node {}: {}/{}",
                node.node.get(),
                node.operation.module_path.join("/"),
                node.operation.operation_name
            );
        }
    }

    let mut validation = activate(
        ReactiveInstanceId::new(0, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .map_err(|resident_error| error(format!("resident activation failed: {resident_error:?}")))?;
    println!(
        "resident activation succeeded: {} turn nodes, {} scheduled nodes",
        validation.plan.nodes.len(),
        validation.plan.execution_node_count(),
    );
    for (ordinal, ((request, _), input)) in services
        .bindings
        .iter()
        .zip(validation.plan.inputs.iter())
        .enumerate()
    {
        println!(
            "  resident input {ordinal}: path={} physical-slot={}",
            request.path,
            input.slot.get(),
        );
    }
    println!(
        "  initial resident state: {:?}",
        resident_state(&validation)
    );
    if std::env::var_os("MECH_EKF_DUMP_ARTIFACT").is_some() {
        for node_id in validation.plan.activation_nodes.iter().copied() {
            let declaration = &artifact.nodes()[node_id.get() as usize];
            println!(
                "activation node {}: {}/{}",
                node_id.get(),
                declaration.operation.module_path.join("/"),
                declaration.operation.operation_name,
            );
        }
    }
    let samples = input_samples();
    validate_resident(&mut validation, &samples);

    let input_slots = validation
        .plan
        .inputs
        .iter()
        .map(|input| mech_core::CellSlotId::new(input.slot.get()))
        .collect::<Vec<_>>();
    let aot = AotProgram::build(&artifact, &validation.plan, &input_slots)
        .map_err(|message| error(format!("AOT build failed: {message}")))?;
    println!(
        "generated AOT Rust from {} kernel instructions: {} (rustc -O: {:.1} ms)",
        aot.instruction_count(),
        aot.source_path().display(),
        aot.compile_time().as_secs_f64() * 1.0e3,
    );
    validate_aot(&aot, &samples);
    let _numeric_proof = prove_general_numeric_kernel(&catalog)?;
    let _batch_proof = prove_batched_numeric_kernel(&catalog)?;
    let _batch_ekf = prove_batched_ekf_kernel(&catalog, &samples)?;

    let mut full = activate(
        ReactiveInstanceId::new(1, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .map_err(|resident_error| error(format!("resident activation failed: {resident_error:?}")))?;
    let mut streamlined = activate(
        ReactiveInstanceId::new(2, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .map_err(|resident_error| error(format!("resident activation failed: {resident_error:?}")))?;

    println!("benchmark (all lanes use the same deterministic input stream):");
    benchmark_raw(&samples);
    benchmark_aot(&aot, &samples);
    benchmark_resident("Mech resident full", &mut full, &samples, true);
    benchmark_resident(
        "Mech resident no summary",
        &mut streamlined,
        &samples,
        false,
    );
    Ok(())
}

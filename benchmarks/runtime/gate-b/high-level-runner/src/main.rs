mod aot;

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

const SOURCE: &str = include_str!("../../ekf-high-level.mec");
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
struct EkfCompilationServices {
    bindings: Vec<(ExecutionResourceRequest, ValRef)>,
}

impl EkfCompilationServices {
    fn value(request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        let value = match request.path.as_str() {
            "pulse" => 0.0,
            "linear-velocity" => 1.0,
            "angular-velocity" => 0.01,
            "bearing" => -0.25,
            path => return Err(error(format!("unknown EKF input path {path}"))),
        };
        Ok(LegacyValue::F64(Ref::new(value)))
    }
}

impl MechExecutionServices for EkfCompilationServices {
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
        Self::value(request)
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        Self::value(request)
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
            [
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
            [
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

fn main() -> MResult<()> {
    let frontend_started = Instant::now();
    let catalog = mech_stdlib::source_catalog();
    let mut services = EkfCompilationServices::default();
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
        "generated AOT Rust: {} (rustc -O: {:.1} ms)",
        aot.source_path().display(),
        aot.compile_time().as_secs_f64() * 1.0e3,
    );
    validate_aot(&aot, &samples);

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

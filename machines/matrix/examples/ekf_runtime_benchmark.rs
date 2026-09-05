use mech_core::matrix::Matrix as ValueMatrix;
use mech_core::{
    FunctionCatalog, FunctionCatalogBuilder, GenericError, MResult, MechError, Ref, Value,
};
use mech_runtime::{
    MechRuntime, ResourcePathCapability, RuntimeBuilder, RuntimeConfig, RuntimeContext,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeIngress, RuntimeLimits, RuntimeResourceProvider,
    RuntimeResourceReadRequest,
};
use nalgebra::{Matrix2, Matrix3, Matrix3x2, RowVector3};
use std::{
    env,
    f64::consts::TAU,
    hint::black_box,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

const BASE_URI: &str = "bench://ekf-runtime";
const SOURCE: &str = include_str!("../benchmarks/ekf/ekf.mec");
const SAMPLE_COUNT: usize = 9;
const INPUT_PERIOD: usize = 4_096;
const TARGET_SAMPLE: Duration = Duration::from_millis(75);
const WARMUP_MIN: Duration = Duration::from_millis(250);
const DT: f64 = 0.1;
const LANDMARK_X: f64 = 140.0;
const LANDMARK_Y: f64 = 12.0;
const MEASUREMENT_NOISE: f64 = 0.25;

#[derive(Clone, Copy, Debug)]
struct InputSample {
    linear_velocity: f64,
    angular_velocity: f64,
    bearing: f64,
}

fn wrap_angle(angle: f64) -> f64 {
    angle.sin().atan2(angle.cos())
}

fn input_samples() -> Arc<Vec<InputSample>> {
    static SAMPLES: OnceLock<Arc<Vec<InputSample>>> = OnceLock::new();
    Arc::clone(SAMPLES.get_or_init(|| {
        let mut truth = RowVector3::<f64>::new(45.0, 15.0, 0.0);
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
        Arc::new(samples)
    }))
}

#[derive(Clone)]
struct Ekf {
    state: RowVector3<f64>,
    covariance: Matrix3<f64>,
}

impl Ekf {
    fn new() -> Self {
        Self {
            state: RowVector3::new(55.0, 25.0, 0.4),
            covariance: Matrix3::new(100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15),
        }
    }

    fn turn(&mut self, input: InputSample) {
        let theta = self.state[2];
        let (sin_theta, cos_theta) = theta.sin_cos();
        let distance = input.linear_velocity * DT;
        let predicted_state = self.state
            + RowVector3::new(
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
            RowVector3::new(delta_y / squared_range, -delta_x / squared_range, -1.0);
        let innovation_variance = (observation_jacobian
            * predicted_covariance
            * observation_jacobian.transpose())[(0, 0)]
            + MEASUREMENT_NOISE;
        let gain = predicted_covariance * observation_jacobian.transpose() / innovation_variance;

        self.state = predicted_state + (gain * innovation).transpose();
        let correction = Matrix3::identity() - gain * observation_jacobian;
        self.covariance = correction * predicted_covariance * correction.transpose()
            + gain * gain.transpose() * MEASUREMENT_NOISE;
    }

    fn check(&self) -> f64 {
        self.state.sum() + self.covariance.trace()
    }
}

struct Measurement {
    median_ms: f64,
    min_ms: f64,
    max_ms: f64,
    batch_iterations: usize,
}

fn measurement(mut samples: Vec<f64>, batch_iterations: usize) -> Measurement {
    samples.sort_by(f64::total_cmp);
    Measurement {
        median_ms: samples[SAMPLE_COUNT / 2],
        min_ms: samples[0],
        max_ms: samples[SAMPLE_COUNT - 1],
        batch_iterations,
    }
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
    let per_iteration = calibration.elapsed().as_secs_f64().max(1e-9);
    let batch_iterations = (TARGET_SAMPLE.as_secs_f64() / per_iteration)
        .ceil()
        .clamp(1.0, 100_000.0) as usize;
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            operation();
        }
        samples.push(start.elapsed().as_secs_f64() * 1_000.0 / batch_iterations as f64);
    }
    measurement(samples, batch_iterations)
}

fn print_result(runtime: &str, result: &Measurement, check: f64) {
    println!(
        "{runtime},ekf,{:.9},{:.9},{:.9},{},{:.12}",
        result.median_ms, result.min_ms, result.max_ms, result.batch_iterations, check,
    );
}

fn raw_rust() {
    let samples = input_samples();
    let mut filter = Ekf::new();
    let mut index = 0;
    let result = measure(|| {
        filter.turn(samples[index]);
        index = (index + 1) % samples.len();
        black_box(filter.check());
    });
    assert!(filter.check().is_finite());
    print_result("raw-rust-loop", &result, filter.check());
}

#[derive(Debug)]
struct EkfInputProvider {
    first: InputSample,
}

impl EkfInputProvider {
    fn new() -> Self {
        Self {
            first: input_samples()[0],
        }
    }

    fn value(&self, path: &str) -> MResult<Value> {
        let value = match path {
            "pulse" => 0.0,
            "linear-velocity" => self.first.linear_velocity,
            "angular-velocity" => self.first.angular_velocity,
            "bearing" => self.first.bearing,
            _ => {
                return Err(MechError::new(
                    GenericError {
                        msg: format!("unknown EKF benchmark resource path {path}"),
                    },
                    None,
                ));
            }
        };
        Ok(Value::F64(Ref::new(value)))
    }
}

impl RuntimeResourceProvider for EkfInputProvider {
    fn scheme(&self) -> &str {
        "bench"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![BASE_URI.to_string()]
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.value(&request.path)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.value(&request.path)
    }
}

#[derive(Debug, Default)]
struct EkfInputDriver {
    live: bool,
}

impl RuntimeHostInputDriver for EkfInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == BASE_URI
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        self.live = true;
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.live = false;
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live
    }
}

fn function_catalog() -> Arc<FunctionCatalog> {
    static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();
    Arc::clone(CATALOG.get_or_init(|| {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_native_plan(&mut builder).unwrap();
        mech_engine::install_intrinsic_source(&mut builder).unwrap();
        mech_math::install_runtime(&mut builder).unwrap();
        mech_math::install_source(&mut builder).unwrap();
        mech_matrix::install_runtime(&mut builder).unwrap();
        mech_matrix::install_source(&mut builder).unwrap();
        Arc::new(builder.build().unwrap())
    }))
}

fn runtime(rollback: bool) -> (MechRuntime, RuntimeContext) {
    let builder = RuntimeBuilder::new()
        .config(RuntimeConfig::new("EKF benchmark").with_limits(RuntimeLimits::trusted()))
        .function_catalog(function_catalog())
        .resource_provider(Box::new(EkfInputProvider::new()))
        .input_driver(EkfInputDriver::default());
    let builder = if rollback {
        builder
    } else {
        builder.fail_stop_host_input_turns()
    };
    let mut runtime = builder.build().unwrap();
    let subject = runtime.runtime_context().unwrap().subject().to_string();
    let capability =
        ResourcePathCapability::wildcard(runtime.next_capability_id(), subject, BASE_URI, ["read"])
            .unwrap();
    runtime.grant_capability(Arc::new(capability)).unwrap();
    let context = runtime.runtime_context().unwrap();
    (runtime, context)
}

struct RuntimeFixture {
    runtime: MechRuntime,
    context: RuntimeContext,
    sources: [RuntimeHostInputSource; 4],
    samples: Arc<Vec<InputSample>>,
    turns: usize,
}

impl RuntimeFixture {
    fn source(rollback: bool) -> (Self, Vec<u8>) {
        let (mut runtime, mut context) = runtime(rollback);
        runtime
            .run_string_with_context(&mut context, SOURCE)
            .unwrap();
        let bytecode = runtime.compile_program_bytecode().unwrap();
        (Self::new(runtime, context), bytecode)
    }

    fn bytecode(bytecode: &[u8], rollback: bool) -> Self {
        let (mut runtime, mut context) = runtime(rollback);
        runtime
            .install_bytecode_with_context(&mut context, bytecode)
            .unwrap();
        Self::new(runtime, context)
    }

    fn new(runtime: MechRuntime, context: RuntimeContext) -> Self {
        Self {
            runtime,
            context,
            sources: [
                RuntimeHostInputSource::new(BASE_URI, "pulse").unwrap(),
                RuntimeHostInputSource::new(BASE_URI, "linear-velocity").unwrap(),
                RuntimeHostInputSource::new(BASE_URI, "angular-velocity").unwrap(),
                RuntimeHostInputSource::new(BASE_URI, "bearing").unwrap(),
            ],
            samples: input_samples(),
            turns: 0,
        }
    }

    fn turn(&mut self) {
        let sample = self.samples[self.turns % self.samples.len()];
        let input = RuntimeHostInput::new(vec![
            RuntimeHostInputUpdate {
                source: self.sources[0].clone(),
                value: RuntimeHostInputValue::F64((self.turns + 1) as f64),
            },
            RuntimeHostInputUpdate {
                source: self.sources[1].clone(),
                value: RuntimeHostInputValue::F64(sample.linear_velocity),
            },
            RuntimeHostInputUpdate {
                source: self.sources[2].clone(),
                value: RuntimeHostInputValue::F64(sample.angular_velocity),
            },
            RuntimeHostInputUpdate {
                source: self.sources[3].clone(),
                value: RuntimeHostInputValue::F64(sample.bearing),
            },
        ])
        .unwrap();
        let outcome = self
            .runtime
            .apply_host_input_with_context(&mut self.context, input)
            .unwrap();
        assert_eq!(outcome.binding_count, 4);
        black_box(outcome.turn);
        self.turns += 1;
    }

    fn state(&self) -> RowVector3<f64> {
        let Value::MatrixF64(ValueMatrix::RowVector3(value)) = self
            .runtime
            .root_symbol_value("state")
            .unwrap()
            .into_value()
        else {
            panic!("EKF state is not a row vector 3");
        };
        *value.borrow()
    }

    fn covariance(&self) -> Matrix3<f64> {
        let Value::MatrixF64(ValueMatrix::Matrix3(value)) = self
            .runtime
            .root_symbol_value("covariance")
            .unwrap()
            .into_value()
        else {
            panic!("EKF covariance is not a matrix 3");
        };
        *value.borrow()
    }

    fn check(&self) -> f64 {
        self.state().sum() + self.covariance().trace()
    }
}

fn validate(fixture: &RuntimeFixture) {
    let mut expected = Ekf::new();
    for turn in 0..fixture.turns {
        expected.turn(fixture.samples[turn % fixture.samples.len()]);
    }
    let state_error = (fixture.state() - expected.state).amax();
    let covariance_error = (fixture.covariance() - expected.covariance).amax();
    assert!(
        state_error < 1e-7 && covariance_error < 1e-7,
        "EKF mismatch after {} turns: state error {state_error}, covariance error {covariance_error}; actual state {:?}, expected state {:?}; actual covariance {:?}, expected covariance {:?}",
        fixture.turns,
        fixture.state(),
        expected.state,
        fixture.covariance(),
        expected.covariance,
    );
}

fn benchmark_mech_source(rollback: bool) {
    let (mut source, _) = RuntimeFixture::source(rollback);
    let source_result = measure(|| source.turn());
    validate(&source);
    let suffix = if rollback { "" } else { "-fail-stop" };
    print_result(
        &format!("mech-runtime-source{suffix}"),
        &source_result,
        black_box(source.check()),
    );
}

fn bytecode_semantic_error() -> String {
    let (_, bytecode_bytes) = RuntimeFixture::source(true);
    let bytecode = RuntimeFixture::bytecode(&bytecode_bytes, true);
    let initial = Ekf::new();
    let state_error = (bytecode.state() - initial.state).amax();
    let covariance_error = (bytecode.covariance() - initial.covariance).amax();
    assert!(
        state_error >= 1e-7 || covariance_error >= 1e-7,
        "bytecode activation semantics now pass initialization; enable and validate the bytecode benchmark lane",
    );
    format!(
        "bytecode EKF lane rejected: install changed activation state (state error {state_error:.6}, covariance error {covariance_error:.6}); bytecode does not yet preserve activation sampling/register semantics"
    )
}

fn main() {
    let modes = env::args().skip(1).collect::<Vec<_>>();
    if modes.iter().any(|mode| mode == "--check") {
        let (mut source, _) = RuntimeFixture::source(true);
        for _ in 0..256 {
            source.turn();
            validate(&source);
        }
        println!(
            "EKF source fixture passed 256 validated turns ({} plan nodes, {} live input bindings)",
            source.runtime.root_plan_len(),
            source.runtime.live_input_binding_count(),
        );
        eprintln!("{}", bytecode_semantic_error());
        return;
    }
    if modes.iter().any(|mode| mode == "--bytecode") {
        panic!("{}", bytecode_semantic_error());
    }
    let run_atomic = modes.is_empty() || modes.iter().any(|mode| mode == "--atomic");
    let run_fail_stop = modes.is_empty() || modes.iter().any(|mode| mode == "--fail-stop");
    println!("runtime,operation,median_ms,min_ms,max_ms,batch_iterations,check");
    raw_rust();
    if run_atomic {
        benchmark_mech_source(true);
    }
    if run_fail_stop {
        benchmark_mech_source(false);
    }
    eprintln!("{}", bytecode_semantic_error());
}

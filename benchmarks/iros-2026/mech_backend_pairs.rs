//! One fresh-process sample for the poster's matched Mech backend comparison.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    error::Error,
    io,
    time::{Duration, Instant},
};

use mech_core::CellSlotId;
use mech_gpu::{
    BatchedAotSimdCpuArtifact, BatchedAotSimdCpuSession, BatchedCpuSession, BatchedExecutionError,
    BatchedJitCpuSession, BatchedJitSimdCpuSession, BatchedResidentMetalSession, ComputeLowerer,
    FixedShapeKernel,
};
use mech_runtime::RuntimeBuilder;
use serde_json::{Value, json};

type AnyResult<T> = Result<T, Box<dyn Error>>;
type Inputs = BTreeMap<String, Vec<f32>>;
type State = BTreeMap<CellSlotId, Vec<f32>>;

const SOURCE: &str = include_str!("../../examples/embedded_ekf/ekf.mec");
const INPUTS: [&str; 7] = [
    "dt",
    "linear-velocity",
    "angular-velocity",
    "bearing",
    "measurement-noise",
    "finite-limit",
    "covariance-symmetry-tolerance",
];
const GUARDS: [&str; 3] = [
    "finite-candidate!",
    "positive-covariance!",
    "symmetric-covariance!",
];
const ABS_TOLERANCE: f32 = 2.0e-4;
const REL_TOLERANCE: f32 = 1.0e-5;
const WARMUP_TURNS: u32 = 5;
const VALIDATION_INSTANCES: usize = 4092;
const VALIDATION_TURNS: u32 = 45;

#[derive(Clone, Copy)]
enum Backend {
    Evaluator,
    ScalarJit,
    SimdAot,
    SimdJit8w,
    Metal,
}

impl Backend {
    fn name(self) -> &'static str {
        match self {
            Self::Evaluator => "evaluator",
            Self::ScalarJit => "scalar-jit",
            Self::SimdAot => "simd-aot",
            Self::SimdJit8w => "simd-jit-8w",
            Self::Metal => "metal",
        }
    }
    fn parse(value: &str) -> AnyResult<Self> {
        match value {
            "evaluator" => Ok(Self::Evaluator),
            "scalar-jit" => Ok(Self::ScalarJit),
            "simd-aot" => Ok(Self::SimdAot),
            "simd-jit-8w" => Ok(Self::SimdJit8w),
            "metal" => Ok(Self::Metal),
            _ => Err(invalid(format!("unknown backend {value}"))),
        }
    }
}

struct Options {
    backend: Backend,
    checked: bool,
    instances: usize,
    turns: u32,
    validate: bool,
}

fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}

fn options() -> AnyResult<Options> {
    let mut backend = None;
    let mut checked = None;
    let mut instances = 500_000;
    let mut turns = 40;
    let mut validate = false;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--validate" {
            validate = true;
            continue;
        }
        let value = arguments
            .next()
            .ok_or_else(|| invalid(format!("missing value for {argument}")))?;
        match argument.as_str() {
            "--backend" => backend = Some(Backend::parse(&value)?),
            "--mode" => {
                checked = Some(match value.as_str() {
                    "checked" => true,
                    "unchecked" => false,
                    _ => return Err(invalid("--mode must be checked or unchecked")),
                })
            }
            "--instances" => instances = value.parse()?,
            "--turns" => turns = value.parse()?,
            _ => return Err(invalid(format!("unknown option {argument}"))),
        }
    }
    if instances == 0 || instances > u32::MAX as usize || instances % 4 != 0 {
        return Err(invalid(
            "--instances must be positive, divisible by four, and fit u32",
        ));
    }
    if turns == 0 {
        return Err(invalid("--turns must be positive"));
    }
    Ok(Options {
        backend: backend.ok_or_else(|| invalid("--backend is required"))?,
        checked: checked.ok_or_else(|| invalid("--mode is required"))?,
        instances,
        turns,
        validate,
    })
}

fn inputs(instances: usize) -> Inputs {
    let mut linear = Vec::with_capacity(instances);
    let mut angular = Vec::with_capacity(instances);
    let mut bearing = Vec::with_capacity(instances);
    for index in 0..instances {
        let phase = std::f32::consts::TAU * index as f32 / instances as f32;
        linear.push(1.0_f32 + 0.05_f32 * (phase * 3.0_f32).sin());
        angular.push(0.015_f32 * (1.0_f32 + 0.1_f32 * (phase * 2.0_f32).sin()));
        bearing.push(
            -0.55_f32 + 0.01_f32 * (phase * 7.0_f32).sin() + 0.005_f32 * (phase * 11.0_f32).sin(),
        );
    }
    BTreeMap::from([
        ("dt".to_owned(), vec![0.1]),
        ("linear-velocity".to_owned(), linear),
        ("angular-velocity".to_owned(), angular),
        ("bearing".to_owned(), bearing),
        ("measurement-noise".to_owned(), vec![0.25]),
        ("finite-limit".to_owned(), vec![f32::MAX]),
        ("covariance-symmetry-tolerance".to_owned(), vec![0.0001]),
    ])
}

fn compile(inputs: &Inputs, checked: bool) -> AnyResult<FixedShapeKernel> {
    let tree = mech_syntax::parse(SOURCE).map_err(|error| invalid(format!("parse: {error:?}")))?;
    let artifact = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .map_err(|error| invalid(format!("compiler: {error:?}")))?
        .compile_tree_artifact_with_inputs(
            &tree,
            &BTreeMap::new(),
            &INPUTS.into_iter().map(str::to_owned).collect(),
        )
        .map_err(|error| invalid(format!("source: {error:?}")))?
        .into_artifact();
    let kernel = ComputeLowerer
        .compile_broadcast(&artifact, inputs)
        .map_err(|error| invalid(format!("lowering: {error:?}")))?;
    let names = kernel
        .named_integrity_constraints()
        .map(|(_, name)| name)
        .collect::<BTreeSet<_>>();
    if names != GUARDS.into_iter().collect() || kernel.integrity_constraints().count() != 3 {
        return Err(invalid(format!(
            "expected exactly three declared EKF guards, found {names:?}"
        )));
    }
    if checked {
        Ok(kernel)
    } else {
        kernel
            .without_named_integrity_constraints(&GUARDS)
            .map_err(|error| invalid(format!("guard removal: {error:?}")))
    }
}

struct Prepared {
    backend: Backend,
    program: FixedShapeKernel,
    aot: Option<BatchedAotSimdCpuArtifact>,
}

impl Prepared {
    fn new(program: FixedShapeKernel, backend: Backend) -> AnyResult<Self> {
        let aot = if matches!(backend, Backend::SimdAot) {
            Some(program.compile_aot_simd_cpu()?)
        } else {
            None
        };
        Ok(Self {
            backend,
            program,
            aot,
        })
    }
    fn start(&self, inputs: &Inputs) -> AnyResult<Session> {
        Ok(match self.backend {
            Backend::Evaluator => Session::Evaluator(self.program.prepare_cpu(inputs)?),
            Backend::ScalarJit => Session::ScalarJit(self.program.prepare_jit_cpu(inputs)?),
            Backend::SimdAot => Session::SimdAot(self.aot.as_ref().unwrap().prepare(inputs)?),
            Backend::SimdJit8w => {
                let mut session = self.program.prepare_jit_simd_cpu(inputs)?;
                session.prepare_parallel(8)?;
                Session::SimdJit8w(session)
            }
            Backend::Metal => Session::Metal(self.program.prepare_metal(inputs)?),
        })
    }
}

enum Session {
    Evaluator(BatchedCpuSession),
    ScalarJit(BatchedJitCpuSession),
    SimdAot(BatchedAotSimdCpuSession),
    SimdJit8w(BatchedJitSimdCpuSession),
    Metal(BatchedResidentMetalSession),
}

impl Session {
    fn dispatch(&mut self, turns: u32) -> Result<Duration, BatchedExecutionError> {
        let started = Instant::now();
        match self {
            Self::Evaluator(session) => session.dispatch_turns(turns)?,
            Self::ScalarJit(session) => session.dispatch_turns(turns)?,
            Self::SimdAot(session) => session.dispatch_turns_resident(turns)?,
            Self::SimdJit8w(session) => session.dispatch_turns_parallel(turns, 8)?,
            Self::Metal(session) => return session.dispatch_turns(turns),
        }
        Ok(started.elapsed())
    }
    fn state(&mut self) -> Result<State, BatchedExecutionError> {
        Ok(match self {
            Self::Evaluator(session) => session.state().clone(),
            Self::ScalarJit(session) => session.state().clone(),
            Self::SimdAot(session) => session.read_state().clone(),
            Self::SimdJit8w(session) => session.state().clone(),
            Self::Metal(session) => return session.read_published_state(),
        })
    }
    fn counts(&self) -> (u64, u64) {
        match self {
            Self::Evaluator(session) => (session.attempted_turns(), session.fault_count()),
            Self::ScalarJit(session) => (session.attempted_turns(), session.fault_count()),
            Self::SimdAot(session) => (session.attempted_turns(), session.fault_count()),
            Self::SimdJit8w(session) => (session.attempted_turns(), session.fault_count()),
            Self::Metal(session) => (session.attempted_turns(), session.fault_count()),
        }
    }
}

fn validation(backend: Backend, checked: bool, requested_instances: usize) -> AnyResult<Value> {
    let instances = requested_instances.min(VALIDATION_INSTANCES);
    let inputs = inputs(instances);
    let reference = compile(&inputs, true)?;
    let mut scalar = reference.prepare_cpu(&inputs)?;
    scalar.dispatch_turns(VALIDATION_TURNS)?;
    let prepared = Prepared::new(compile(&inputs, checked)?, backend)?;
    let mut actual = prepared.start(&inputs)?;
    actual.dispatch(VALIDATION_TURNS)?;
    let state = actual.state()?;
    if state.len() != scalar.state().len() {
        return Err(invalid("validation state slot count differs"));
    }
    let mut maximum_absolute_error = 0.0_f64;
    let mut maximum_tolerance_ratio = 0.0_f64;
    let mut compared_components = 0_u64;
    for (slot, expected) in scalar.state() {
        let values = state
            .get(slot)
            .ok_or_else(|| invalid("validation state slot missing"))?;
        if values.len() != expected.len() {
            return Err(invalid("validation state width differs"));
        }
        for (index, (&expected, &value)) in expected.iter().zip(values).enumerate() {
            let error = (value - expected).abs();
            let tolerance = ABS_TOLERANCE + REL_TOLERANCE * expected.abs();
            if !expected.is_finite() || !value.is_finite() || error > tolerance {
                return Err(invalid(format!(
                    "validation failed: slot {slot:?}, component {index}, expected {expected}, actual {value}, error {error}, tolerance {tolerance}"
                )));
            }
            compared_components += 1;
            maximum_absolute_error = maximum_absolute_error.max(f64::from(error));
            maximum_tolerance_ratio = maximum_tolerance_ratio.max(f64::from(error / tolerance));
        }
    }
    let nan_rollback = if checked {
        let mut invalid_inputs = inputs.clone();
        invalid_inputs.get_mut("bearing").unwrap()[instances - 1] = f32::NAN;
        let mut session = prepared.start(&invalid_inputs)?;
        let before = session.state()?;
        let fault = match session.dispatch(1) {
            Err(BatchedExecutionError::Integrity(fault)) => fault,
            other => {
                return Err(invalid(format!(
                    "NaN validation did not reject the candidate: {other:?}"
                )));
            }
        };
        if fault.instance as usize != instances - 1 || fault.attempted_turn != 1 {
            return Err(invalid(format!(
                "NaN fault attribution was incorrect: {fault:?}"
            )));
        }
        let (attempted_turns, faults) = session.counts();
        if session.state()? != before || (attempted_turns, faults) != (1, 1) {
            return Err(invalid(
                "NaN rejection changed published state or fault accounting",
            ));
        }
        json!({ "passed": true, "faults": faults, "attempted_turns": attempted_turns,
            "fault_lane": fault.instance, "fault_attempted_turn": fault.attempted_turn,
            "fault_constraint_id": fault.constraint.get(), "fault_constraint_name": fault.constraint_name,
            "injected_nan_lane": instances - 1,
            "stage": "initial published state", "all_state_unchanged": true })
    } else {
        Value::Null
    };
    Ok(
        json!({ "passed": true, "instances": instances, "turns": VALIDATION_TURNS,
        "reference": "scalar checked", "absolute_tolerance": ABS_TOLERANCE,
        "relative_tolerance": REL_TOLERANCE, "compared_components": compared_components,
        "maximum_absolute_error": maximum_absolute_error,
        "maximum_tolerance_ratio": maximum_tolerance_ratio, "nan_rollback": nan_rollback }),
    )
}

fn main() -> AnyResult<()> {
    let options = options()?;
    let validation = if options.validate {
        validation(options.backend, options.checked, options.instances)?
    } else {
        Value::Null
    };
    let inputs = inputs(options.instances);
    let prepared = Prepared::new(compile(&inputs, options.checked)?, options.backend)?;
    if prepared.program.instances() as usize != options.instances {
        return Err(invalid(
            "compiled broadcast extent differs from requested instances",
        ));
    }
    let mut session = prepared.start(&inputs)?;
    session.dispatch(WARMUP_TURNS)?;
    let elapsed = session.dispatch(options.turns)?;
    let (attempted_turns, faults) = session.counts();
    if faults != 0 || attempted_turns != u64::from(options.turns) + u64::from(WARMUP_TURNS) {
        return Err(invalid(
            "measured sample did not publish every requested turn",
        ));
    }
    let state = session.state()?;
    let checksum = state
        .values()
        .flatten()
        .map(|&value| f64::from(value))
        .sum::<f64>();
    let state_summaries = state
        .iter()
        .map(|(slot, values)| {
            json!({
                "slot": slot.get(), "components": values.len(),
                "sum_f64": values.iter().map(|&value| f64::from(value)).sum::<f64>(),
            })
        })
        .collect::<Vec<_>>();
    if !checksum.is_finite() {
        return Err(invalid("non-finite measured-state checksum"));
    }
    let total_filter_turns = options.instances as u64 * u64::from(options.turns);
    let elapsed_ns = u64::try_from(elapsed.as_nanos())?;
    if elapsed_ns == 0 {
        return Err(invalid("zero elapsed duration"));
    }
    println!(
        "{}",
        json!({
            "schema_version": 1, "backend": options.backend.name(),
            "mode": if options.checked { "checked" } else { "unchecked" },
            "instances": options.instances, "turns": options.turns,
            "total_filter_turns": total_filter_turns, "elapsed_ns": elapsed_ns,
            "throughput_million_filter_turns_per_second": total_filter_turns as f64 * 1000.0 / elapsed_ns as f64,
            "warmup_turns": WARMUP_TURNS, "warmup_in_measured_session": true,
            "measured_start_after_turn": WARMUP_TURNS,
            "workers": if matches!(options.backend, Backend::SimdJit8w) { 8 } else { 1 },
            "publication": "one completed publication per turn", "attempted_turns": attempted_turns,
            "faults": faults, "checksum": checksum, "validation": validation,
            "state_summaries": state_summaries,
            "source": "examples/embedded_ekf/ekf.mec",
            "removed_source_guards": if options.checked { vec![] } else { GUARDS.to_vec() },
            "state_components": state.values().map(Vec::len).sum::<usize>(),
            "library_path": prepared.aot.as_ref().map(|artifact| artifact.path().display().to_string()),
        })
    );
    Ok(())
}

use mech_core::{
    FunctionCatalog, FunctionCatalogBuilder, GenericError, MResult, MechError, Ref, Value,
};
use mech_runtime::{
    MechRuntime, ResourcePathCapability, RuntimeBuilder, RuntimeConfig, RuntimeContext,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue,
    RuntimeIngress, RuntimeLimits, RuntimeResourceProvider, RuntimeResourceReadRequest,
};
use std::{
    hint::black_box,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

const BASE_URI: &str = "bench://nbody-runtime";
const SOURCE: &str = include_str!("../benchmarks/nbody/nbody.mec");
const SAMPLE_COUNT: usize = 9;
const TARGET_SAMPLE: Duration = Duration::from_millis(75);
const WARMUP_MIN: Duration = Duration::from_millis(250);
const DT: f64 = 0.01;
const MASSES: [f64; 5] = [
    39.478_417_604_357_43,
    0.037_693_674_870_389_49,
    0.011_286_326_131_968_767,
    0.001_723_724_057_059_711_2,
    0.002_033_686_869_924_630_4,
];
const PAIRS: [(usize, usize); 10] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 4),
    (1, 2),
    (1, 3),
    (1, 4),
    (2, 3),
    (2, 4),
    (3, 4),
];

#[derive(Clone, Debug)]
struct System {
    position: [[f64; 3]; 5],
    velocity: [[f64; 3]; 5],
}

impl System {
    fn initial() -> Self {
        Self {
            position: [
                [0.0, 0.0, 0.0],
                [
                    4.841_431_442_464_721,
                    -1.160_320_044_027_428_4,
                    -0.103_622_044_471_123_11,
                ],
                [
                    8.343_366_718_244_58,
                    4.124_798_564_124_305,
                    -0.403_523_417_114_321_4,
                ],
                [
                    12.894_369_562_139_131,
                    -15.111_151_401_698_632,
                    -0.223_307_578_892_655_73,
                ],
                [
                    15.379_697_114_850_917,
                    -25.919_314_609_987_964,
                    0.179_258_772_950_371_18,
                ],
            ],
            velocity: [
                [
                    -0.000_387_663_407_198_742_67,
                    -0.003_275_359_037_176_570_7,
                    0.000_023_935_734_080_003,
                ],
                [
                    0.606_326_392_995_832,
                    2.811_986_844_916_260_2,
                    -0.025_218_361_659_887_63,
                ],
                [
                    -1.010_774_346_178_792_4,
                    1.825_662_371_230_411_9,
                    0.008_415_761_376_584_154,
                ],
                [
                    1.082_791_006_441_535_4,
                    0.868_713_018_169_608_2,
                    -0.010_832_637_401_363_636,
                ],
                [
                    0.979_090_732_243_898,
                    0.594_698_998_647_676_2,
                    -0.034_755_955_504_078_104,
                ],
            ],
        }
    }

    fn advance(&mut self) {
        for (left, right) in PAIRS {
            let delta = [
                self.position[left][0] - self.position[right][0],
                self.position[left][1] - self.position[right][1],
                self.position[left][2] - self.position[right][2],
            ];
            let distance_squared = delta.iter().map(|value| value * value).sum::<f64>();
            let magnitude = DT / (distance_squared * distance_squared.sqrt());
            for axis in 0..3 {
                self.velocity[left][axis] -= delta[axis] * MASSES[right] * magnitude;
                self.velocity[right][axis] += delta[axis] * MASSES[left] * magnitude;
            }
        }
        for body in 0..5 {
            for axis in 0..3 {
                self.position[body][axis] += self.velocity[body][axis] * DT;
            }
        }
    }

    fn energy(&self) -> f64 {
        let kinetic = (0..5)
            .map(|body| {
                0.5 * MASSES[body]
                    * self.velocity[body]
                        .iter()
                        .map(|value| value * value)
                        .sum::<f64>()
            })
            .sum::<f64>();
        let potential = PAIRS
            .iter()
            .map(|&(left, right)| {
                let distance = (0..3)
                    .map(|axis| {
                        let delta = self.position[left][axis] - self.position[right][axis];
                        delta * delta
                    })
                    .sum::<f64>()
                    .sqrt();
                MASSES[left] * MASSES[right] / distance
            })
            .sum::<f64>();
        kinetic - potential
    }

    fn max_error(&self, other: &Self) -> f64 {
        let mut error: f64 = 0.0;
        for body in 0..5 {
            for axis in 0..3 {
                error = error.max((self.position[body][axis] - other.position[body][axis]).abs());
                error = error.max((self.velocity[body][axis] - other.velocity[body][axis]).abs());
            }
        }
        error
    }
}

struct Measurement {
    median_ms: f64,
    min_ms: f64,
    max_ms: f64,
    batch_iterations: usize,
}

fn finish_measurement(mut samples: Vec<f64>, batch_iterations: usize) -> Measurement {
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
    let mut warmup_iterations = 0usize;
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
    finish_measurement(samples, batch_iterations)
}

#[derive(Debug)]
struct InputProvider;

impl RuntimeResourceProvider for InputProvider {
    fn scheme(&self) -> &str {
        "bench"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![BASE_URI.to_string()]
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.read(request)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.path == "pulse" {
            Ok(Value::F64(Ref::new(0.0)))
        } else {
            Err(MechError::new(
                GenericError {
                    msg: format!("unknown n-body benchmark resource path {}", request.path),
                },
                None,
            ))
        }
    }
}

#[derive(Debug, Default)]
struct InputDriver {
    live: bool,
}

impl RuntimeHostInputDriver for InputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == BASE_URI && source.path() == "pulse"
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
        Arc::new(builder.build().unwrap())
    }))
}

fn runtime(rollback: bool) -> (MechRuntime, RuntimeContext) {
    let builder = RuntimeBuilder::new()
        .config(RuntimeConfig::new("n-body benchmark").with_limits(RuntimeLimits::trusted()))
        .function_catalog(function_catalog())
        .resource_provider(Box::new(InputProvider))
        .input_driver(InputDriver::default());
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
    pulse_source: RuntimeHostInputSource,
    turns: usize,
    installation_steps: usize,
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
        let mut fixture = Self {
            runtime,
            context,
            pulse_source: RuntimeHostInputSource::new(BASE_URI, "pulse").unwrap(),
            turns: 0,
            installation_steps: 0,
        };
        let actual = fixture.system();
        let mut reference = System::initial();
        for steps in 0..=2 {
            if actual.max_error(&reference) < 1e-10 {
                fixture.installation_steps = steps;
                return fixture;
            }
            reference.advance();
        }
        panic!("installed n-body state does not match the initial system or its first two steps");
    }

    fn turn(&mut self) {
        let outcome = self
            .runtime
            .apply_host_input_with_context(
                &mut self.context,
                RuntimeHostInput::single(
                    self.pulse_source.clone(),
                    RuntimeHostInputValue::F64((self.turns + 1) as f64),
                ),
            )
            .unwrap();
        assert_eq!(outcome.binding_count, 1);
        black_box(outcome.turn);
        self.turns += 1;
    }

    fn scalar(&self, name: &str) -> f64 {
        let Value::F64(value) = self.runtime.root_symbol_value(name).unwrap().into_value() else {
            panic!("n-body symbol {name} is not f64");
        };
        *value.borrow()
    }

    fn system(&self) -> System {
        let mut system = System::initial();
        for body in 0..5 {
            for (axis, prefix) in ["x", "y", "z"].iter().enumerate() {
                system.position[body][axis] = self.scalar(&format!("{prefix}{body}"));
            }
            for (axis, prefix) in ["vx", "vy", "vz"].iter().enumerate() {
                system.velocity[body][axis] = self.scalar(&format!("{prefix}{body}"));
            }
        }
        system
    }

    fn max_reference_error(&self) -> f64 {
        let mut expected = System::initial();
        for _ in 0..self.installation_steps + self.turns {
            expected.advance();
        }
        self.system().max_error(&expected)
    }

    fn validate(&self) {
        let error = self.max_reference_error();
        assert!(
            error < 1e-8,
            "n-body mismatch after {} installed + {} host steps: max error {error}",
            self.installation_steps,
            self.turns
        );
    }
}

fn print_result(runtime: &str, result: &Measurement, fixture: &RuntimeFixture) {
    println!(
        "{runtime},nbody-step,{:.9},{:.9},{:.9},{},{:.12},{}",
        result.median_ms,
        result.min_ms,
        result.max_ms,
        result.batch_iterations,
        fixture.system().energy(),
        fixture.installation_steps,
    );
}

fn benchmark(rollback: bool) {
    let (mut source_check, bytecode_bytes) = RuntimeFixture::source(rollback);
    let mut bytecode_check = RuntimeFixture::bytecode(&bytecode_bytes, rollback);
    for _ in 0..1_000 {
        source_check.turn();
        bytecode_check.turn();
    }
    source_check.validate();
    let bytecode_error = bytecode_check.max_reference_error();
    assert!(
        bytecode_error >= 1e-8,
        "bytecode n-body semantics now validate; enable the bytecode timing lane"
    );
    eprintln!(
        "bytecode n-body lane rejected after 1,000 turns: max state error {bytecode_error:.9}; bytecode does not yet preserve activation sampling/register semantics"
    );

    let (mut source, _) = RuntimeFixture::source(rollback);
    let source_result = measure(|| source.turn());
    source.validate();
    let suffix = if rollback { "" } else { "-fail-stop" };
    print_result(
        &format!("mech-runtime-source{suffix}"),
        &source_result,
        &source,
    );
}

fn main() {
    println!("runtime,operation,median_ms,min_ms,max_ms,batch_iterations,check,installation_steps");
    benchmark(true);
    benchmark(false);
}

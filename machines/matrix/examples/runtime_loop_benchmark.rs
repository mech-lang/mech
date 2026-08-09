use mech_core::matrix::Matrix as ValueMatrix;
use mech_core::{
    FunctionCatalog, FunctionCatalogBuilder, GenericError, MResult, MechError, Ref, LegacyValue,
};
use mech_runtime::{
    MechRuntime, ResourcePathCapability, RuntimeBuilder, RuntimeConfig, RuntimeContext,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue,
    RuntimeIngress, RuntimeLimits, RuntimeResourceProvider, RuntimeResourceReadRequest,
};
use nalgebra::{DMatrix, DVector};
use std::{
    env,
    hint::black_box,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

const BASE_URI: &str = "bench://matrix-runtime";
const SAMPLE_COUNT: usize = 9;
const TARGET_SAMPLE: Duration = Duration::from_millis(75);
const WARMUP_MIN: Duration = Duration::from_millis(250);

#[derive(Clone, Copy)]
enum Operation {
    MatMul,
    Solve,
}

impl Operation {
    fn name(self) -> &'static str {
        match self {
            Self::MatMul => "matmul",
            Self::Solve => "solve",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::MatMul => {
                "@pulse := bench://matrix-runtime{:read(pulse)}\n\
                 @lhs := bench://matrix-runtime{:read(lhs)}\n\
                 @rhs := bench://matrix-runtime{:read(rhs)}\n\
                 scaled-lhs := @lhs/lhs * @pulse/pulse\n\
                 result := scaled-lhs ** @rhs/rhs\n\
                 result"
            }
            Self::Solve => {
                "@pulse := bench://matrix-runtime{:read(pulse)}\n\
                 @lhs := bench://matrix-runtime{:read(lhs)}\n\
                 @rhs := bench://matrix-runtime{:read(rhs)}\n\
                 scaled-rhs := @rhs/rhs * @pulse/pulse\n\
                 result := @lhs/lhs \\ scaled-rhs\n\
                 result"
            }
        }
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

fn measure_pair(
    mut source: impl FnMut(),
    mut bytecode: impl FnMut(),
) -> (Measurement, Measurement) {
    let warmup_start = Instant::now();
    let mut warmup_iterations = 0usize;
    while warmup_iterations < 2 || warmup_start.elapsed() < WARMUP_MIN {
        source();
        bytecode();
        warmup_iterations += 1;
    }

    let source_calibration = Instant::now();
    source();
    let source_per_iteration = source_calibration.elapsed().as_secs_f64().max(1e-9);
    let bytecode_calibration = Instant::now();
    bytecode();
    let bytecode_per_iteration = bytecode_calibration.elapsed().as_secs_f64().max(1e-9);
    let per_iteration = source_per_iteration.max(bytecode_per_iteration);
    let batch_iterations = (TARGET_SAMPLE.as_secs_f64() / per_iteration)
        .ceil()
        .clamp(1.0, 100_000.0) as usize;

    let mut source_samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut bytecode_samples = Vec::with_capacity(SAMPLE_COUNT);
    let run_sample = |operation: &mut dyn FnMut()| {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            operation();
        }
        start.elapsed().as_secs_f64() * 1_000.0 / batch_iterations as f64
    };
    for sample in 0..SAMPLE_COUNT {
        if sample % 2 == 0 {
            source_samples.push(run_sample(&mut source));
            bytecode_samples.push(run_sample(&mut bytecode));
        } else {
            bytecode_samples.push(run_sample(&mut bytecode));
            source_samples.push(run_sample(&mut source));
        }
    }
    (
        measurement(source_samples, batch_iterations),
        measurement(bytecode_samples, batch_iterations),
    )
}

fn matrix_value(row: usize, column: usize, salt: usize) -> f64 {
    ((row * 17 + column * 13 + salt * 19) % 101 + 1) as f64 / 101.0
}

fn multiply_inputs(size: usize) -> (DMatrix<f64>, DMatrix<f64>) {
    (
        DMatrix::from_fn(size, size, |row, column| matrix_value(row, column, 1)),
        DMatrix::from_fn(size, size, |row, column| matrix_value(row, column, 2)),
    )
}

fn solve_inputs(size: usize) -> (DMatrix<f64>, DVector<f64>) {
    let matrix = DMatrix::from_fn(size, size, |row, column| {
        if row == column {
            size as f64 + 4.0
        } else {
            ((row * 7 + column * 11) % 19) as f64 * 0.01 - 0.09
        }
    });
    let vector = DVector::from_fn(size, |row, _| (row % 17 + 1) as f64 / 17.0);
    (matrix, vector)
}

#[derive(Debug)]
struct MatrixInputProvider {
    lhs: DMatrix<f64>,
    rhs_matrix: Option<DMatrix<f64>>,
    rhs_vector: Option<DVector<f64>>,
}

impl MatrixInputProvider {
    fn new(size: usize, operation: Operation) -> Self {
        match operation {
            Operation::MatMul => {
                let (lhs, rhs) = multiply_inputs(size);
                Self {
                    lhs,
                    rhs_matrix: Some(rhs),
                    rhs_vector: None,
                }
            }
            Operation::Solve => {
                let (lhs, rhs) = solve_inputs(size);
                Self {
                    lhs,
                    rhs_matrix: None,
                    rhs_vector: Some(rhs),
                }
            }
        }
    }

    fn value(&self, path: &str) -> MResult<LegacyValue> {
        match path {
            "pulse" => Ok(LegacyValue::F64(Ref::new(1.0))),
            "lhs" => Ok(LegacyValue::MatrixF64(ValueMatrix::DMatrix(Ref::new(
                self.lhs.clone(),
            )))),
            "rhs" => match (&self.rhs_matrix, &self.rhs_vector) {
                (Some(rhs), None) => Ok(LegacyValue::MatrixF64(ValueMatrix::DMatrix(Ref::new(
                    rhs.clone(),
                )))),
                (None, Some(rhs)) => Ok(LegacyValue::MatrixF64(ValueMatrix::DVector(Ref::new(
                    rhs.clone(),
                )))),
                _ => unreachable!("provider has exactly one right-hand side"),
            },
            _ => Err(MechError::new(
                GenericError {
                    msg: format!("unknown matrix benchmark resource path {path}"),
                },
                None,
            )),
        }
    }
}

impl RuntimeResourceProvider for MatrixInputProvider {
    fn scheme(&self) -> &str {
        "bench"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![BASE_URI.to_string()]
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.value(&request.path)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.value(&request.path)
    }
}

#[derive(Debug, Default)]
struct BenchmarkInputDriver {
    live: bool,
}

impl RuntimeHostInputDriver for BenchmarkInputDriver {
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
        mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
        mech_engine::install_intrinsic_source(&mut builder).unwrap();
        mech_math::install_runtime(&mut builder).unwrap();
        mech_math::install_source(&mut builder).unwrap();
        mech_matrix::install_runtime(&mut builder).unwrap();
        mech_matrix::install_source(&mut builder).unwrap();
        Arc::new(builder.build().unwrap())
    }))
}

fn runtime(size: usize, operation: Operation) -> (MechRuntime, RuntimeContext) {
    let mut runtime = RuntimeBuilder::new()
        .config(RuntimeConfig::new("matrix benchmark").with_limits(RuntimeLimits::trusted()))
        .function_catalog(function_catalog())
        .resource_provider(Box::new(MatrixInputProvider::new(size, operation)))
        .input_driver(BenchmarkInputDriver::default())
        .build()
        .unwrap();
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
    input: RuntimeHostInputSource,
    pulse: f64,
}

impl RuntimeFixture {
    fn source(size: usize, operation: Operation) -> (Self, Vec<u8>) {
        let (mut runtime, mut context) = runtime(size, operation);
        runtime
            .run_string_with_context(&mut context, operation.source())
            .unwrap();
        let bytecode = runtime.compile_program_bytecode().unwrap();
        (
            Self {
                runtime,
                context,
                input: RuntimeHostInputSource::new(BASE_URI, "pulse").unwrap(),
                pulse: 1.0,
            },
            bytecode,
        )
    }

    fn bytecode(size: usize, operation: Operation, bytecode: &[u8]) -> Self {
        let (mut runtime, mut context) = runtime(size, operation);
        runtime
            .install_bytecode_with_context(&mut context, bytecode)
            .unwrap();
        Self {
            runtime,
            context,
            input: RuntimeHostInputSource::new(BASE_URI, "pulse").unwrap(),
            pulse: 1.0,
        }
    }

    fn turn(&mut self) {
        self.pulse = if self.pulse == 1.0 { 1.000_001 } else { 1.0 };
        let outcome = self
            .runtime
            .apply_host_input_with_context(
                &mut self.context,
                RuntimeHostInput::single(
                    self.input.clone(),
                    RuntimeHostInputValue::F64(self.pulse),
                ),
            )
            .unwrap();
        assert_eq!(outcome.binding_count, 1);
        black_box(outcome.turn);
    }

    fn result(&self) -> LegacyValue {
        self.runtime
            .root_symbol_value("result")
            .unwrap()
            .into_value()
    }
}

fn validate_matmul(size: usize, fixture: &RuntimeFixture) -> f64 {
    let LegacyValue::MatrixF64(ValueMatrix::DMatrix(output)) = fixture.result() else {
        panic!("matrix runtime benchmark result is not a dynamic matrix");
    };
    let (lhs, rhs) = multiply_inputs(size);
    let expected = (0..size)
        .map(|inner| lhs[(0, inner)] * rhs[(inner, 0)])
        .sum::<f64>()
        * fixture.pulse;
    let check = output.borrow()[(0, 0)];
    assert!((check - expected).abs() < 1e-7);
    check
}

fn validate_solve(size: usize, fixture: &RuntimeFixture) -> f64 {
    let LegacyValue::MatrixF64(ValueMatrix::DVector(output)) = fixture.result() else {
        panic!("solve runtime benchmark result is not a dynamic vector");
    };
    let (matrix, rhs) = solve_inputs(size);
    let residual = (&matrix * &*output.borrow() - rhs * fixture.pulse).amax();
    assert!(residual < 1e-8, "runtime solve residual {residual}");
    output.borrow()[0]
}

fn print_result(
    runtime: &str,
    operation: Operation,
    size: usize,
    result: &Measurement,
    check: f64,
) {
    println!(
        "{runtime},{},{size},{:.9},{:.9},{:.9},{},{:.12}",
        operation.name(),
        result.median_ms,
        result.min_ms,
        result.max_ms,
        result.batch_iterations,
        check,
    );
}

fn benchmark(size: usize, operation: Operation) {
    let (mut source, bytecode_bytes) = RuntimeFixture::source(size, operation);
    let mut bytecode = RuntimeFixture::bytecode(size, operation, &bytecode_bytes);
    assert_eq!(
        source.runtime.root_plan_len(),
        bytecode.runtime.root_plan_len()
    );
    assert_eq!(
        source.runtime.live_input_binding_count(),
        bytecode.runtime.live_input_binding_count(),
    );

    let (source_measurement, bytecode_measurement) =
        measure_pair(|| source.turn(), || bytecode.turn());
    let source_check = match operation {
        Operation::MatMul => validate_matmul(size, &source),
        Operation::Solve => validate_solve(size, &source),
    };
    print_result(
        "mech-runtime-source",
        operation,
        size,
        &source_measurement,
        black_box(source_check),
    );

    let bytecode_check = match operation {
        Operation::MatMul => validate_matmul(size, &bytecode),
        Operation::Solve => validate_solve(size, &bytecode),
    };
    print_result(
        "mech-runtime-bytecode",
        operation,
        size,
        &bytecode_measurement,
        black_box(bytecode_check),
    );
}

fn sizes() -> Vec<usize> {
    let values = env::args().skip(1).collect::<Vec<_>>();
    if values.is_empty() {
        return vec![64, 128, 256, 512];
    }
    values
        .iter()
        .map(|value| value.parse().expect("sizes must be positive integers"))
        .collect()
}

fn main() {
    println!("runtime,operation,size,median_ms,min_ms,max_ms,batch_iterations,check");
    for size in sizes() {
        benchmark(size, Operation::MatMul);
        benchmark(size, Operation::Solve);
    }
}

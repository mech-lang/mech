use mech_core::matrix::Matrix as ValueMatrix;
use mech_core::{
    FunctionCatalog, FunctionCatalogBuilder, GenericError, MResult, MechError, MechFunction,
    ReactiveCellId, ReactivePlan, Ref, LegacyValue,
};
use mech_matrix::MatMulMDMD;
use mech_runtime::{
    MechRuntime, ResourcePathCapability, RuntimeBuilder, RuntimeConfig, RuntimeContext,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue,
    RuntimeIngress, RuntimeLimits, RuntimeResourceProvider, RuntimeResourceReadRequest,
};
use nalgebra::DMatrix;
use std::{
    convert::Infallible,
    env,
    hint::black_box,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

const BASE_URI: &str = "bench://rectangular-runtime";
const SAMPLE_COUNT: usize = 9;
const TARGET_SAMPLE: Duration = Duration::from_millis(75);
const WARMUP_MIN: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug)]
struct Shape {
    rows: usize,
    inner: usize,
    columns: usize,
}

impl Shape {
    fn parse(value: &str) -> Self {
        let dimensions = value
            .split(['x', 'X', ','])
            .map(|dimension| {
                dimension
                    .parse::<usize>()
                    .expect("shape dimensions must be positive integers")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            dimensions.len(),
            3,
            "shapes must use the form rows x inner x columns"
        );
        assert!(
            dimensions.iter().all(|dimension| *dimension > 0),
            "shape dimensions must be positive"
        );
        Self {
            rows: dimensions[0],
            inner: dimensions[1],
            columns: dimensions[2],
        }
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

fn measure_with_error<E>(mut operation: impl FnMut() -> Result<(), E>) -> Result<Measurement, E> {
    let warmup_start = Instant::now();
    let mut warmup_iterations = 0usize;
    while warmup_iterations < 2 || warmup_start.elapsed() < WARMUP_MIN {
        operation()?;
        warmup_iterations += 1;
    }

    let calibration_start = Instant::now();
    operation()?;
    let per_iteration = calibration_start.elapsed().as_secs_f64().max(1e-9);
    let batch_iterations = (TARGET_SAMPLE.as_secs_f64() / per_iteration)
        .ceil()
        .clamp(1.0, 100_000.0) as usize;
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            operation()?;
        }
        samples.push(start.elapsed().as_secs_f64() * 1_000.0 / batch_iterations as f64);
    }
    Ok(finish_measurement(samples, batch_iterations))
}

fn measure(mut operation: impl FnMut()) -> Measurement {
    match measure_with_error(|| {
        operation();
        Ok::<(), Infallible>(())
    }) {
        Ok(measurement) => measurement,
        Err(never) => match never {},
    }
}

fn measure_result(operation: impl FnMut() -> MResult<()>) -> MResult<Measurement> {
    measure_with_error(operation)
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

    let source_start = Instant::now();
    source();
    let source_time = source_start.elapsed().as_secs_f64().max(1e-9);
    let bytecode_start = Instant::now();
    bytecode();
    let bytecode_time = bytecode_start.elapsed().as_secs_f64().max(1e-9);
    let batch_iterations = (TARGET_SAMPLE.as_secs_f64() / source_time.max(bytecode_time))
        .ceil()
        .clamp(1.0, 100_000.0) as usize;
    let run_sample = |operation: &mut dyn FnMut()| {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            operation();
        }
        start.elapsed().as_secs_f64() * 1_000.0 / batch_iterations as f64
    };

    let mut source_samples = Vec::with_capacity(SAMPLE_COUNT);
    let mut bytecode_samples = Vec::with_capacity(SAMPLE_COUNT);
    for sample in 0..SAMPLE_COUNT {
        if sample.is_multiple_of(2) {
            source_samples.push(run_sample(&mut source));
            bytecode_samples.push(run_sample(&mut bytecode));
        } else {
            bytecode_samples.push(run_sample(&mut bytecode));
            source_samples.push(run_sample(&mut source));
        }
    }
    (
        finish_measurement(source_samples, batch_iterations),
        finish_measurement(bytecode_samples, batch_iterations),
    )
}

fn matrix_value(row: usize, column: usize, salt: usize) -> f64 {
    ((row * 17 + column * 13 + salt * 19) % 101 + 1) as f64 / 101.0
}

fn inputs(shape: Shape) -> (DMatrix<f64>, DMatrix<f64>) {
    (
        DMatrix::from_fn(shape.rows, shape.inner, |row, column| {
            matrix_value(row, column, 1)
        }),
        DMatrix::from_fn(shape.inner, shape.columns, |row, column| {
            matrix_value(row, column, 2)
        }),
    )
}

fn expected_first(lhs: &DMatrix<f64>, rhs: &DMatrix<f64>, shape: Shape) -> f64 {
    (0..shape.inner)
        .map(|inner| lhs[(0, inner)] * rhs[(inner, 0)])
        .sum()
}

fn print_result(runtime: &str, shape: Shape, result: &Measurement, check: f64) {
    println!(
        "{runtime},{},{},{},{:.9},{:.9},{:.9},{},{:.12}",
        shape.rows,
        shape.inner,
        shape.columns,
        result.median_ms,
        result.min_ms,
        result.max_ms,
        result.batch_iterations,
        check,
    );
}

fn raw(shape: Shape) {
    let (lhs, rhs) = inputs(shape);
    let mut output = DMatrix::zeros(shape.rows, shape.columns);
    let result = measure(|| lhs.mul_to(&rhs, &mut output));
    let expected = expected_first(&lhs, &rhs, shape);
    assert!((output[(0, 0)] - expected).abs() < 1e-8);
    print_result("raw-rust", shape, &result, black_box(output[(0, 0)]));
}

fn kernel(shape: Shape) -> MResult<()> {
    let (lhs, rhs) = inputs(shape);
    let output = Ref::new(DMatrix::zeros(shape.rows, shape.columns));
    let function: Box<dyn MechFunction> = Box::new(MatMulMDMD::<f64> {
        lhs: Ref::new(lhs.clone()),
        rhs: Ref::new(rhs.clone()),
        out: output.clone(),
    });
    let result = measure_result(|| function.solve_result())?;
    let check = output.borrow()[(0, 0)];
    assert!((check - expected_first(&lhs, &rhs, shape)).abs() < 1e-8);
    print_result("mech-kernel", shape, &result, black_box(check));
    Ok(())
}

fn reactive(shape: Shape) {
    let (lhs, rhs) = inputs(shape);
    let lhs = Ref::new(lhs);
    let rhs = Ref::new(rhs);
    let output = Ref::new(DMatrix::zeros(shape.rows, shape.columns));
    let lhs_value = LegacyValue::MatrixF64(ValueMatrix::DMatrix(lhs.clone()));
    let rhs_value = LegacyValue::MatrixF64(ValueMatrix::DMatrix(rhs.clone()));
    let dirty: ReactiveCellId = lhs_value.reactive_root_cell_ids()[0];
    let mut plan = ReactivePlan::new();
    plan.register(
        Box::new(MatMulMDMD::<f64> {
            lhs,
            rhs,
            out: output.clone(),
        }),
        &[lhs_value, rhs_value],
    )
    .unwrap();
    let result = measure(|| {
        black_box(plan.solve_dirty_cells(&[dirty]).unwrap());
    });
    let check = output.borrow()[(0, 0)];
    assert!(check.is_finite());
    print_result("mech-reactive", shape, &result, black_box(check));
}

#[derive(Debug)]
struct MatrixInputProvider {
    lhs: DMatrix<f64>,
    rhs: DMatrix<f64>,
}

impl MatrixInputProvider {
    fn new(shape: Shape) -> Self {
        let (lhs, rhs) = inputs(shape);
        Self { lhs, rhs }
    }

    fn value(&self, path: &str) -> MResult<LegacyValue> {
        match path {
            "pulse" => Ok(LegacyValue::F64(Ref::new(1.0))),
            "lhs" => Ok(LegacyValue::MatrixF64(ValueMatrix::DMatrix(Ref::new(
                self.lhs.clone(),
            )))),
            "rhs" => Ok(LegacyValue::MatrixF64(ValueMatrix::DMatrix(Ref::new(
                self.rhs.clone(),
            )))),
            _ => Err(MechError::new(
                GenericError {
                    msg: format!("unknown rectangular benchmark resource path {path}"),
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

fn runtime(shape: Shape) -> (MechRuntime, RuntimeContext) {
    let mut runtime = RuntimeBuilder::new()
        .config(RuntimeConfig::new("rectangular benchmark").with_limits(RuntimeLimits::trusted()))
        .function_catalog(function_catalog())
        .resource_provider(Box::new(MatrixInputProvider::new(shape)))
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
    fn source(shape: Shape) -> Self {
        let (mut runtime, mut context) = runtime(shape);
        runtime
            .run_string_with_context(
                &mut context,
                "@pulse := bench://rectangular-runtime{:read(pulse)}\n\
                 @lhs := bench://rectangular-runtime{:read(lhs)}\n\
                 @rhs := bench://rectangular-runtime{:read(rhs)}\n\
                 scaled-lhs := @lhs/lhs * @pulse/pulse\n\
                 result := scaled-lhs ** @rhs/rhs\n\
                 result",
            )
            .unwrap();
        Self {
            runtime,
            context,
            input: RuntimeHostInputSource::new(BASE_URI, "pulse").unwrap(),
            pulse: 1.0,
        }
    }

    fn bytecode(shape: Shape, bytecode: &[u8]) -> Self {
        let (mut runtime, mut context) = runtime(shape);
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

    fn validate(&self, shape: Shape) -> f64 {
        let LegacyValue::MatrixF64(ValueMatrix::DMatrix(output)) = self
            .runtime
            .root_symbol_value("result")
            .unwrap()
            .into_value()
        else {
            panic!("rectangular runtime result is not a dynamic matrix");
        };
        let (lhs, rhs) = inputs(shape);
        let expected = expected_first(&lhs, &rhs, shape) * self.pulse;
        let check = output.borrow()[(0, 0)];
        assert!((check - expected).abs() < 1e-7);
        check
    }
}

fn retained_runtime(shape: Shape) {
    let mut source = RuntimeFixture::source(shape);
    let bytecode_bytes = source.runtime.compile_program_bytecode().unwrap();
    let mut bytecode = RuntimeFixture::bytecode(shape, &bytecode_bytes);
    assert_eq!(
        source.runtime.root_plan_len(),
        bytecode.runtime.root_plan_len()
    );
    assert_eq!(
        source.runtime.live_input_binding_count(),
        bytecode.runtime.live_input_binding_count()
    );
    let (source_result, bytecode_result) = measure_pair(|| source.turn(), || bytecode.turn());
    print_result(
        "mech-runtime-source",
        shape,
        &source_result,
        black_box(source.validate(shape)),
    );
    print_result(
        "mech-runtime-bytecode",
        shape,
        &bytecode_result,
        black_box(bytecode.validate(shape)),
    );
}

fn retained_source_runtime(shape: Shape) {
    let mut source = RuntimeFixture::source(shape);
    let result = measure(|| source.turn());
    print_result(
        "mech-runtime-source",
        shape,
        &result,
        black_box(source.validate(shape)),
    );
}

#[derive(Clone, Copy)]
enum Mode {
    Full,
    SourceOnly,
    DirectOnly,
}

fn configuration() -> (Mode, Vec<Shape>) {
    let mut mode = Mode::Full;
    let mut values = Vec::new();
    for value in env::args().skip(1) {
        match value.as_str() {
            "--source-only" => mode = Mode::SourceOnly,
            "--direct-only" => mode = Mode::DirectOnly,
            _ => values.push(value),
        }
    }
    if values.is_empty() {
        values = [
            "1x4096x1".to_string(),
            "4096x4x4".to_string(),
            "4096x8x8".to_string(),
            "4096x16x4".to_string(),
            "1024x1x1024".to_string(),
        ]
        .to_vec();
    }
    (
        mode,
        values.iter().map(|value| Shape::parse(value)).collect(),
    )
}

fn main() -> MResult<()> {
    println!("runtime,rows,inner,columns,median_ms,min_ms,max_ms,batch_iterations,check");
    let (mode, shapes) = configuration();
    for shape in shapes {
        raw(shape);
        kernel(shape)?;
        reactive(shape);
        match mode {
            Mode::Full => retained_runtime(shape),
            Mode::SourceOnly => retained_source_runtime(shape),
            Mode::DirectOnly => {}
        }
    }
    Ok(())
}

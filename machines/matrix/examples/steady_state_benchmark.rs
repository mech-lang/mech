use mech_core::matrix::Matrix as ValueMatrix;
use mech_core::{MechFunction, ReactiveCellId, ReactivePlan, Ref, Value};
use mech_matrix::{MatMulMDMD, MatrixSolveMDVD};
use nalgebra::{DMatrix, DVector};
use std::{
    env,
    hint::black_box,
    time::{Duration, Instant},
};

const SAMPLE_COUNT: usize = 9;
const TARGET_SAMPLE: Duration = Duration::from_millis(75);
const WARMUP_MIN: Duration = Duration::from_millis(250);

struct Measurement {
    median_ms: f64,
    min_ms: f64,
    max_ms: f64,
    batch_iterations: usize,
}

fn measure(mut operation: impl FnMut()) -> Measurement {
    let warmup_start = Instant::now();
    let mut warmup_iterations = 0usize;
    while warmup_iterations < 2 || warmup_start.elapsed() < WARMUP_MIN {
        operation();
        warmup_iterations += 1;
    }

    let calibration_start = Instant::now();
    operation();
    let per_iteration = calibration_start.elapsed().as_secs_f64().max(1e-9);
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
    samples.sort_by(f64::total_cmp);
    Measurement {
        median_ms: samples[SAMPLE_COUNT / 2],
        min_ms: samples[0],
        max_ms: samples[SAMPLE_COUNT - 1],
        batch_iterations,
    }
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

fn print_result(runtime: &str, operation: &str, size: usize, result: &Measurement, check: f64) {
    println!(
        "{runtime},{operation},{size},{:.9},{:.9},{:.9},{},{:.12}",
        result.median_ms, result.min_ms, result.max_ms, result.batch_iterations, check,
    );
}

fn raw_matmul(size: usize) {
    let (lhs, rhs) = multiply_inputs(size);
    let mut output = DMatrix::zeros(size, size);
    let measurement = measure(|| lhs.mul_to(&rhs, &mut output));
    let expected = (0..size)
        .map(|inner| lhs[(0, inner)] * rhs[(inner, 0)])
        .sum::<f64>();
    assert!((output[(0, 0)] - expected).abs() < 1e-8);
    print_result(
        "raw-rust",
        "matmul",
        size,
        &measurement,
        black_box(output[(0, 0)]),
    );
}

fn raw_solve(size: usize) {
    let (matrix, rhs) = solve_inputs(size);
    let mut output = DVector::zeros(size);
    let measurement = measure(|| {
        output = matrix
            .clone()
            .lu()
            .solve(&rhs)
            .expect("benchmark system is nonsingular");
    });
    let residual = (&matrix * &output - &rhs).amax();
    assert!(residual < 1e-8, "raw Rust solve residual {residual}");
    print_result(
        "raw-rust",
        "solve",
        size,
        &measurement,
        black_box(output[0]),
    );
}

fn mech_matmul_function(size: usize) -> (Box<dyn MechFunction>, Ref<DMatrix<f64>>) {
    let (lhs, rhs) = multiply_inputs(size);
    let output = Ref::new(DMatrix::zeros(size, size));
    let function = MatMulMDMD::<f64> {
        lhs: Ref::new(lhs),
        rhs: Ref::new(rhs),
        out: output.clone(),
    };
    (Box::new(function), output)
}

fn mech_solve_function(
    size: usize,
) -> (
    Box<dyn MechFunction>,
    Ref<DVector<f64>>,
    DMatrix<f64>,
    DVector<f64>,
) {
    let (matrix, rhs) = solve_inputs(size);
    let output = Ref::new(DVector::zeros(size));
    let function = MatrixSolveMDVD::<f64> {
        lhs: Ref::new(matrix.clone()),
        rhs: Ref::new(rhs.clone()),
        out: output.clone(),
    };
    (Box::new(function), output, matrix, rhs)
}

fn mech_kernel_matmul(size: usize) {
    let (function, output) = mech_matmul_function(size);
    let measurement = measure(|| function.solve_result().unwrap());
    let check = output.borrow()[(0, 0)];
    assert!(check.is_finite());
    print_result(
        "mech-kernel",
        "matmul",
        size,
        &measurement,
        black_box(check),
    );
}

fn mech_kernel_solve(size: usize) {
    let (function, output, matrix, rhs) = mech_solve_function(size);
    let measurement = measure(|| function.solve_result().unwrap());
    let residual = (&matrix * &*output.borrow() - rhs).amax();
    assert!(residual < 1e-8, "Mech kernel solve residual {residual}");
    print_result(
        "mech-kernel",
        "solve",
        size,
        &measurement,
        black_box(output.borrow()[0]),
    );
}

fn reactive_matmul_fixture(size: usize) -> (ReactivePlan, ReactiveCellId, Ref<DMatrix<f64>>) {
    let (lhs, rhs) = multiply_inputs(size);
    let lhs = Ref::new(lhs);
    let rhs = Ref::new(rhs);
    let output = Ref::new(DMatrix::zeros(size, size));
    let lhs_value = Value::MatrixF64(ValueMatrix::DMatrix(lhs.clone()));
    let rhs_value = Value::MatrixF64(ValueMatrix::DMatrix(rhs.clone()));
    let dirty = lhs_value.reactive_root_cell_ids()[0];
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
    (plan, dirty, output)
}

fn reactive_solve_fixture(
    size: usize,
) -> (
    ReactivePlan,
    ReactiveCellId,
    Ref<DVector<f64>>,
    DMatrix<f64>,
    DVector<f64>,
) {
    let (matrix, rhs) = solve_inputs(size);
    let lhs = Ref::new(matrix.clone());
    let input = Ref::new(rhs.clone());
    let output = Ref::new(DVector::zeros(size));
    let lhs_value = Value::MatrixF64(ValueMatrix::DMatrix(lhs.clone()));
    let rhs_value = Value::MatrixF64(ValueMatrix::DVector(input.clone()));
    let mut plan = ReactivePlan::new();
    plan.register(
        Box::new(MatrixSolveMDVD::<f64> {
            lhs,
            rhs: input,
            out: output.clone(),
        }),
        &[lhs_value, rhs_value.clone()],
    )
    .unwrap();
    (
        plan,
        rhs_value.reactive_root_cell_ids()[0],
        output,
        matrix,
        rhs,
    )
}

fn mech_reactive_matmul(size: usize) {
    let (mut plan, dirty, output) = reactive_matmul_fixture(size);
    let measurement = measure(|| {
        black_box(plan.solve_dirty_cells(&[dirty]).unwrap());
    });
    let check = output.borrow()[(0, 0)];
    assert!(check.is_finite());
    print_result(
        "mech-reactive",
        "matmul",
        size,
        &measurement,
        black_box(check),
    );
}

fn mech_reactive_solve(size: usize) {
    let (mut plan, dirty, output, matrix, rhs) = reactive_solve_fixture(size);
    let measurement = measure(|| {
        black_box(plan.solve_dirty_cells(&[dirty]).unwrap());
    });
    let residual = (&matrix * &*output.borrow() - rhs).amax();
    assert!(residual < 1e-8, "Mech reactive solve residual {residual}");
    print_result(
        "mech-reactive",
        "solve",
        size,
        &measurement,
        black_box(output.borrow()[0]),
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
        raw_matmul(size);
        mech_kernel_matmul(size);
        mech_reactive_matmul(size);
        raw_solve(size);
        mech_kernel_solve(size);
        mech_reactive_solve(size);
    }
}

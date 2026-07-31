extern crate mech_core;

use mech::program::{MechProgram, MechProgramConfig};
use mech_core::{Value, structures::matrix::Matrix};

fn run(source: &str) -> bool {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string(source).is_ok()
}

#[cfg(feature = "dynamic-modules")]
fn run_matrix_round(source: &str) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let result = program.run_string(source).unwrap();

    let detached = match result {
        Value::MutableReference(v) => v.borrow().clone(),
        value => value,
    };

    assert_eq!(
        detached,
        Value::MatrixF64(Matrix::from_vec(vec![1.0, 5.0], 1, 2))
    );
}

#[cfg(feature = "dynamic-modules")]
fn run_scalar_f64(source: &str, expected: f64) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let result = program.run_string(source).unwrap();

    let detached = match result {
        Value::MutableReference(v) => v.borrow().clone(),
        value => value,
    };

    match detached {
        Value::F64(value) => assert_eq!(*value.borrow(), expected),
        value => panic!("expected f64 result, got {:?}", value),
    }
}

#[cfg(feature = "dynamic-modules")]
fn run_matrix_f64(source: &str, expected: Vec<f64>, rows: usize, cols: usize) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let result = program.run_string(source).unwrap();

    let detached = match result {
        Value::MutableReference(v) => v.borrow().clone(),
        value => value,
    };

    assert_eq!(
        detached,
        Value::MatrixF64(Matrix::from_vec(expected, rows, cols))
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_item_import_works() {
    assert!(run("+> math/round\nx := round(1.23)"));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_module_import_works() {
    assert!(run("+> math\nx := math/round(1.23)"));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_glob_import_works() {
    assert!(run("+> math/*\nx := round(1.23)"));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_round_item_import_accepts_matrix() {
    run_matrix_round("+> math/round\nx := round([1.23 4.56])\nx");
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_round_module_import_accepts_matrix() {
    run_matrix_round("+> math\nx := math/round([1.23 4.56])\nx");
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_round_glob_import_accepts_matrix() {
    run_matrix_round("+> math/*\nx := round([1.23 4.56])\nx");
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_sqrt_item_import_works_for_scalar_and_matrix() {
    run_scalar_f64(
        "+> math/sqrt
x := sqrt(9.0)
x",
        3.0,
    );
    run_matrix_f64(
        "+> math/sqrt
x := sqrt([1.0 4.0 9.0])
x",
        vec![1.0, 2.0, 3.0],
        1,
        3,
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_floor_glob_import_works_for_scalar_and_matrix() {
    run_scalar_f64(
        "+> math/*
x := floor(4.56)
x",
        4.0,
    );
    run_matrix_f64(
        "+> math/*
x := floor([1.23 4.56])
x",
        vec![1.0, 4.0],
        1,
        2,
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_ceil_module_import_works_for_scalar_and_matrix() {
    run_scalar_f64(
        "+> math
x := math/ceil(4.56)
x",
        5.0,
    );
    run_matrix_f64(
        "+> math
x := math/ceil([1.23 4.56])
x",
        vec![2.0, 5.0],
        1,
        2,
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_atan2_item_import_works_for_scalar_and_matrix_broadcast() {
    run_scalar_f64(
        "+> math/atan2
x := atan2(0.0, 1.0)
x",
        0.0,
    );
    run_matrix_f64(
        "+> math/atan2
x := atan2([0.0 0.0], 1.0)
x",
        vec![0.0, 0.0],
        1,
        2,
    );
    run_matrix_f64(
        "+> math/atan2
x := atan2(0.0, [1.0 1.0])
x",
        vec![0.0, 0.0],
        1,
        2,
    );
    run_matrix_f64(
        "+> math/atan2
x := atan2([0.0 0.0], [1.0 1.0])
x",
        vec![0.0, 0.0],
        1,
        2,
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_trig_item_import_works_for_scalar_and_matrix() {
    run_scalar_f64(
        "+> math/sin
x := sin(0.0)
x",
        0.0,
    );
    run_matrix_f64(
        "+> math/sin
x := sin([0.0 0.0])
x",
        vec![0.0, 0.0],
        1,
        2,
    );

    run_scalar_f64(
        "+> math/cos
x := cos(0.0)
x",
        1.0,
    );
    run_matrix_f64(
        "+> math/cos
x := cos([0.0 0.0])
x",
        vec![1.0, 1.0],
        1,
        2,
    );

    run_scalar_f64(
        "+> math/tan
x := tan(0.0)
x",
        0.0,
    );
    run_matrix_f64(
        "+> math/tan
x := tan([0.0 0.0])
x",
        vec![0.0, 0.0],
        1,
        2,
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_math_inverse_trig_module_and_glob_import_work_for_scalar_and_matrix() {
    run_scalar_f64(
        "+> math
x := math/asin(0.0)
x",
        0.0,
    );
    run_matrix_f64(
        "+> math
x := math/asin([0.0 0.0])
x",
        vec![0.0, 0.0],
        1,
        2,
    );

    run_scalar_f64(
        "+> math/*
x := acos(1.0)
x",
        0.0,
    );
    run_matrix_f64(
        "+> math/*
x := acos([1.0 1.0])
x",
        vec![0.0, 0.0],
        1,
        2,
    );

    run_scalar_f64(
        "+> math/atan
x := atan(0.0)
x",
        0.0,
    );
    run_matrix_f64(
        "+> math/atan
x := atan([0.0 0.0])
x",
        vec![0.0, 0.0],
        1,
        2,
    );
}

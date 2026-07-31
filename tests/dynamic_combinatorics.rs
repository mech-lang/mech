extern crate mech_core;

use mech::program::{MechProgram, MechProgramConfig};
use mech_core::{Value, structures::matrix::Matrix};

fn run(source: &str) -> bool {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string(source).is_ok()
}

#[cfg(feature = "dynamic-modules")]
fn run_matrix_n_choose_k(source: &str, expected: Vec<f64>) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let result = program.run_string(source).unwrap();

    let detached = match result {
        Value::MutableReference(v) => v.borrow().clone(),
        value => value,
    };

    assert_eq!(detached, Value::MatrixF64(Matrix::from_vec(expected, 1, 2)));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_item_import_works() {
    assert!(run(
        "+> combinatorics/n-choose-k\nx := n-choose-k(10.0, 2.0)"
    ));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_module_import_works() {
    assert!(run(
        "+> combinatorics\nx := combinatorics/n-choose-k(10.0, 2.0)"
    ));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_glob_import_works() {
    assert!(run("+> combinatorics/*\nx := n-choose-k(10.0, 2.0)"));
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_scalar_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0], 2.0)\nx",
        vec![45.0, 190.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_scalar_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/n-choose-k\nx := n-choose-k(10.0, [2.0 3.0])\nx",
        vec![45.0, 120.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0], [2.0 3.0])\nx",
        vec![45.0, 1140.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_module_import_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics\nx := combinatorics/n-choose-k([10.0 20.0], 2.0)\nx",
        vec![45.0, 190.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_glob_import_matrix_broadcast_works() {
    run_matrix_n_choose_k(
        "+> combinatorics/*\nx := n-choose-k([10.0 20.0], 2.0)\nx",
        vec![45.0, 190.0],
    );
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_matrix_shape_mismatch_errors() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let result = program
        .run_string("+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0], [2.0 3.0 4.0])\nx");

    assert!(result.is_err());
}

#[cfg(feature = "dynamic-modules")]
#[test]
fn dynamic_combinatorics_matrix_matrix_same_cells_different_shape_errors() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let result = program.run_string(
        "+> combinatorics/n-choose-k\nx := n-choose-k([10.0 20.0 30.0 40.0], [2.0 3.0; 4.0 5.0])\nx",
    );

    assert!(result.is_err());
}

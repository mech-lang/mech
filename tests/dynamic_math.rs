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

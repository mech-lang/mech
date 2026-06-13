extern crate mech_core;

use mech::program::{MechProgram, MechProgramConfig};
use mech_core::{Value, structures::matrix::Matrix};

fn run(source: &str) -> bool {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string(source).is_ok()
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
fn dynamic_math_round_slice_item_import_works() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let result = program
        .run_string(
            "+> math/round-slice
x := round-slice([1.23 2.7 3.1])
x",
        )
        .unwrap();

    let detached = match result {
        Value::MutableReference(v) => v.borrow().clone(),
        value => value,
    };

    assert_eq!(
        detached,
        Value::MatrixF64(Matrix::from_vec(vec![1.0, 3.0, 3.0], 1, 3))
    );
}

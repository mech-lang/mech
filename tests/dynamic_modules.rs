#![cfg(feature = "dynamic-modules")]

use mech::program::{MechProgram, MechProgramConfig};

fn run_ok(source: &str) {
let mut program = MechProgram::new(MechProgramConfig::default());
let result = program.run_string(source);
assert!(result.is_ok(), "expected program to run successfully");
}

fn run_err(source: &str) {
let mut program = MechProgram::new(MechProgramConfig::default());
let result = program.run_string(source);
assert!(result.is_err(), "expected program to fail");
}

#[test]
fn dynamic_math_module_item_and_glob_imports_work() {
run_ok(
"+> math
+> math/sin
+> math/*
a := math/sin(0.0)
b := sin(0.0)
c := cos(0.0)
d := tan(0.0)
",
);
}

#[test]
fn dynamic_combinatorics_item_import_works() {
run_ok(
"+> combinatorics/n-choose-k
x := n-choose-k(5.0, 2.0)
",
);
}

#[test]
fn dynamic_matrix_unary_math_import_works() {
run_ok(
"+> math/cos
x := cos([0.0 0.0])
",
);
}

#[test]
fn dynamic_binary_broadcast_import_works() {
run_ok(
"+> combinatorics/n-choose-k
x := n-choose-k([10.0 20.0], 2.0)
",
);
}

#[test]
fn dynamic_missing_module_errors() {
run_err(
"+> doesnotexist
x := doesnotexist/thing(1.0)
",
);
}

#[test]
fn dynamic_missing_item_errors() {
run_err(
"+> math/doesnotexist
x := 1
",
);
}

#[test]
fn dynamic_matrix_matrix_same_cells_different_shape_errors() {
run_err(
"+> combinatorics/n-choose-k
x := n-choose-k([10.0 20.0 30.0 40.0], [2.0 3.0; 4.0 5.0])
",
);
}

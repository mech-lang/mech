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

#[test]
fn dynamic_item_alias_import_works() {
    run_ok(
        "+> s := math/sin\nx := s(0.0)\n",
    );
}

#[test]
fn dynamic_grouped_item_import_works() {
    run_ok(
        "+> math/{sin, cos, tan}\nx := sin(0.0)\ny := cos(0.0)\nz := tan(0.0)\n",
    );
}

#[test]
fn dynamic_multiline_grouped_item_import_works() {
    run_ok(
        "+> math/{\n  sin\n  cos\n  tan\n}\nx := sin(0.0)\ny := cos(0.0)\nz := tan(0.0)\n",
    );
}

#[test]
fn dynamic_grouped_item_import_does_not_import_other_items() {
    run_err(
        "+> math/{sin, cos, tan}\nx := round(1.23)\n",
    );
}

#[test]
fn dynamic_comma_shorthand_grouped_import_is_rejected() {
    run_err(
        "+> math/sin, cos, tan\nx := sin(0.0)\n",
    );
}

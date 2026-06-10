use mech::program::{MechProgram, MechProgramConfig};

fn run(source: &str) -> bool {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string(source).is_ok()
}

#[test]
fn prelude_operators_work_without_imports() {
    assert!(run("x := 1 + 1\ny := 1 > 2\nz := true && false"));
}

#[test]
fn set_operators_work_without_imports() {
    assert!(run("a := {1, 2}\nb := {2, 3}\nc := a ∪ b"));
}

#[test]
fn matrix_operators_work_without_imports() {
    assert!(run("a := [1 2]\nb := [3 4]\nc := a · b"));
}

#[test]
fn named_math_functions_fail_without_imports() {
    assert!(!run("x := sin(1.23)"));
    assert!(!run("x := math/sin(1.23)"));
}

#[test]
fn qualified_module_import_enables_only_qualified_calls() {
    assert!(run("+> math\nx := math/sin(1.23)"));
    assert!(!run("+> math\nx := sin(1.23)"));
}

#[test]
fn item_import_enables_only_that_unqualified_item() {
    assert!(run("+> math/sin\nx := sin(1.23)"));
    assert!(!run("+> math/sin\nx := cos(1.23)"));
}

#[test]
fn glob_import_enables_all_module_exports_unqualified() {
    assert!(run("+> math/*\nx := sin(1.23)\ny := cos(1.23)"));
}

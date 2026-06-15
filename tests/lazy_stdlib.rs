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

#[cfg(feature = "linked_stdlib")]
#[test]
fn qualified_module_import_enables_only_qualified_calls() {
    assert!(run("+> math\nx := math/sin(1.23)"));
    assert!(!run("+> math\nx := sin(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn item_import_enables_only_that_unqualified_item() {
    assert!(run("+> math/sin\nx := sin(1.23)"));
    assert!(!run("+> math/sin\nx := cos(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn glob_import_enables_all_module_exports_unqualified() {
    assert!(run("+> math/*\nx := sin(1.23)\ny := cos(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn nested_stats_imports_work_and_require_imports() {
    assert!(!run("x := stats/sum/column([1 2 3])"));
    assert!(run("+> stats\nx := stats/sum/column([1 2 3])"));
    assert!(run("+> stats/sum/column\nx := column([1 2 3])"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn repeated_module_and_item_imports_remain_idempotent() {
    assert!(run("+> math\n+> math\nx := math/sin(1.23)"));
    assert!(run("+> math\n+> math/sin\nx := sin(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn linked_loader_discovers_machine_declared_math_items() {
    assert!(run("+> math/copysign\nx := copysign(1.0, -2.0)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn linked_loader_glob_uses_machine_manifest() {
    assert!(run("+> math/*\nx := round(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn item_alias_import_enables_alias() {
    assert!(run("+> foo := math/sin\nx := foo(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn item_alias_import_does_not_create_original_unqualified_name() {
    assert!(!run("+> foo := math/sin\nx := sin(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn nested_item_alias_import_works() {
    assert!(run("+> total := stats/sum/column\nx := total([1 2 3])"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn grouped_item_import_enables_each_grouped_item() {
    assert!(run("+> math/{sin, cos, tan}\nx := sin(1.23)\ny := cos(1.23)\nz := tan(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn grouped_item_import_does_not_import_other_items() {
    assert!(!run("+> math/{sin, cos, tan}\nx := round(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn multiline_grouped_item_import_works() {
    assert!(run("+> math/{\n  sin\n  cos\n  tan\n}\nx := sin(1.23)\ny := cos(1.23)\nz := tan(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn comma_shorthand_grouped_import_is_rejected() {
    assert!(!run("+> math/sin, cos, tan\nx := sin(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn module_alias_import_is_rejected() {
    assert!(!run("+> m := math\nx := m/sin(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn glob_alias_import_is_rejected() {
    assert!(!run("+> f := math/*\nx := f(1.23)"));
}

#[cfg(feature = "linked_stdlib")]
#[test]
fn grouped_item_alias_import_is_rejected() {
    assert!(!run("+> math/{s := sin}\nx := s(1.23)"));
}

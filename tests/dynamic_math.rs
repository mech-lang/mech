use mech::program::{MechProgram, MechProgramConfig};

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

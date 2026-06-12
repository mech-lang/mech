use mech::program::{MechProgram, MechProgramConfig};

fn run(source: &str) -> bool {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string(source).is_ok()
}

#[cfg(feature = "dynamic-modules")]
#[test]
#[allow(non_snake_case)]
fn dynamicMathItemImportWorks() {
    assert!(run("+> math/round\nx := round(1.23)"));
}

#[cfg(feature = "dynamic-modules")]
#[test]
#[allow(non_snake_case)]
fn dynamicMathModuleImportWorks() {
    assert!(run("+> math\nx := math/round(1.23)"));
}

#[cfg(feature = "dynamic-modules")]
#[test]
#[allow(non_snake_case)]
fn dynamicMathGlobImportWorks() {
    assert!(run("+> math/*\nx := round(1.23)"));
}

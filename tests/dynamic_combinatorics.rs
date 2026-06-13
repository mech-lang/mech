use mech::program::{MechProgram, MechProgramConfig};

fn run(source: &str) -> bool {
    let mut program = MechProgram::new(MechProgramConfig::default());
    program.run_string(source).is_ok()
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

#![cfg(feature = "full-hosts")]

pub mod support;

use support::*;

#[test]
fn literal_only_native_application_builds_and_runs() {
    let result = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Build,
        "literal",
        fixture_path("literal-f64.mecb"),
        "native_literal",
        false,
    );

    assert_exact_mech_packages(&result.plan, &["mech-core", "mech-engine", "mech-runtime"]);
    let catalog = result.catalog_source.unwrap();
    assert!(catalog.contains("FunctionCatalogBuilder::new"));
    assert!(catalog.contains("mech_engine::install_intrinsic_resident"));
    assert_eq!(catalog.matches("__mech_native::install_").count(), 0);

    assert!(result.stdout.unwrap().contains("42"));
}

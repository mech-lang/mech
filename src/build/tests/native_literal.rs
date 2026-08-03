#![cfg(feature = "standard-hosts")]

mod support;

use support::*;

#[test]
fn literal_only_native_application_builds_and_runs() {
    let result = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Build,
        "literal",
        fixture_path("literal-f64.mecb"),
        "phase1_native_literal",
        false,
    );

    assert_exact_mech_packages(&result.plan, &["mech-core", "mech-engine"]);
    let catalog = result.catalog_source.unwrap();
    assert!(catalog.contains("FunctionCatalogBuilder::new"));
    assert!(!catalog.contains("install_"));

    assert!(result.stdout.unwrap().contains("42"));
}

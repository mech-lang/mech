#![cfg(feature = "full-hosts")]

mod support;

use support::*;

#[test]
fn scalar_add_native_application_uses_only_the_exact_installer() {
    let result = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Build,
        "scalar",
        fixture_path("scalar-add-f64.mecb"),
        "native_scalar",
        true,
    );

    assert!(result.poisoned_output_seed);
    assert_exact_mech_packages(&result.plan, &["mech-core", "mech-engine", "mech-math"]);
    let catalog = result.catalog_source.unwrap();
    assert!(catalog.contains("mech_math::__mech_native::install_add_ss_f64"));
    assert_eq!(catalog.matches("__mech_native::install_").count(), 1);

    assert_eq!(result.stdout.unwrap().trim(), "3");
}

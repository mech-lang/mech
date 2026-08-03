#![cfg(feature = "standard-hosts")]

mod support;

use std::collections::BTreeSet;

use support::*;

#[test]
fn representative_non_scalar_families_build_catalogs_and_execute() {
    let cases: [(OwnerProfile, &str, &str, &str, &str); 3] = [
        (
            OwnerProfile::Fixed,
            "fixed",
            "fixed-matrix-add-f64.mecb",
            "phase1_native_fixed_matrix",
            "[6 8; 10 12]",
        ),
        (
            OwnerProfile::Standard,
            "dynamic",
            "dynamic-matrix-add-f64.mecb",
            "phase1_native_dynamic_matrix",
            "[26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26]",
        ),
        (
            OwnerProfile::Standard,
            "variadic",
            "variadic-horzcat-f64.mecb",
            "phase1_native_variadic",
            "[1 2 3 4 5]",
        ),
    ];

    for (profile, case, fixture, binary_name, expected_output) in cases {
        let result = run_owner(
            profile,
            RunnerAction::Build,
            case,
            fixture_path(fixture),
            binary_name,
            true,
        );
        assert!(result.poisoned_output_seed);
        let catalog = result.catalog_source.unwrap();
        for function in &result.plan.runtime_functions {
            assert!(catalog.contains(&function.installer_path));
        }
        assert_eq!(
            catalog.matches("__mech_native::install_").count(),
            result.plan.runtime_functions.len(),
        );

        if profile == OwnerProfile::Fixed {
            let names = result
                .plan
                .runtime_functions
                .iter()
                .map(|function| function.runtime_name.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(
                names,
                BTreeSet::from([
                    "AddM2M2<f64>",
                    "HorizontalConcatenateS2<f64>",
                    "VerticalConcatenateR2R2<f64Matrix2RowVector2RowVector2>",
                ])
            );
            assert!(
                catalog
                    .contains("mech_engine::__mech_native::install_horizontal_concatenate_s2_f64")
            );
            assert!(
                catalog
                    .contains("mech_engine::__mech_native::install_vertical_concatenate_r2_r2_f64")
            );
        }

        assert_eq!(result.stdout.unwrap().trim(), expected_output);
    }
}

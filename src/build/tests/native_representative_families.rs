#![cfg(feature = "full-hosts")]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

pub mod support;

use std::fs;

use mech_core::{BytecodeInstruction, ParsedProgram};
use support::*;

#[test]
fn representative_non_scalar_families_build_catalogs_and_execute() {
    let cases: [(
        OwnerProfile,
        &str,
        &str,
        &str,
        &str,
        &[&str],
        &[&str],
        &[&str],
        &[&str],
    ); 3] = [
        (
            OwnerProfile::Fixed,
            "fixed",
            "fixed-matrix-add-f64.mecb",
            "native_fixed_matrix",
            "[6 8; 10 12]",
            &["math/add"],
            &["f64", "matrix2", "program"],
            &["bool", "f64", "matrix2", "runtime", "vector2"],
            &["f64", "matrix2", "resident-routing", "runtime", "string"],
        ),
        (
            OwnerProfile::Standard,
            "dynamic",
            "dynamic-matrix-add-f64.mecb",
            "native_dynamic_matrix",
            "[26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26; 26 26 26 26 26]",
            &["math/add"],
            &["f64", "matrixd", "program"],
            &["f64", "matrixd", "runtime"],
            &["f64", "matrixd", "resident-routing", "runtime", "string"],
        ),
        (
            OwnerProfile::Standard,
            "variadic",
            "variadic-horzcat-f64.mecb",
            "native_variadic",
            "[1 2 3 4 5]",
            &[],
            &["f64", "program", "row_vectord"],
            &["bool", "f64", "row_vectord", "runtime", "vectord"],
            &[
                "f64",
                "resident-routing",
                "row_vectord",
                "runtime",
                "string",
            ],
        ),
    ];

    for (
        profile,
        case,
        fixture,
        binary_name,
        expected_output,
        expected_operations,
        expected_core_features,
        expected_engine_features,
        expected_runtime_features,
    ) in cases
    {
        let fixture = fixture_path(fixture);
        let bytecode = fs::read(&fixture).unwrap();
        let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
        assert!(parsed.instructions.is_empty(), "{case}");
        assert!(
            parsed
                .instructions
                .iter()
                .filter_map(BytecodeInstruction::runtime_function)
                .next()
                .is_none(),
            "{case}"
        );
        assert_eq!(
            mech_engine::decode_program_artifact_bytecode_v1(&bytecode)
                .unwrap()
                .operation_references()
                .into_iter()
                .map(|operation| operation.canonical_name())
                .collect::<Vec<_>>(),
            expected_operations
                .iter()
                .map(|operation| (*operation).to_owned())
                .collect::<Vec<_>>(),
            "{case}"
        );

        let result = run_owner(
            profile,
            RunnerAction::Build,
            case,
            fixture,
            binary_name,
            false,
        );
        assert!(!result.poisoned_output_seed, "{case}");
        assert_eq!(result.poisoned_output_seed_count, 0, "{case}");
        assert!(result.plan.runtime_functions.is_empty(), "{case}");
        assert!(result.plan.runtime_types.is_empty(), "{case}");
        assert_exact_mech_packages(&result.plan, &["mech-core", "mech-engine", "mech-runtime"]);
        assert_eq!(
            result
                .plan
                .core_features
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected_core_features,
            "{case}"
        );
        assert_eq!(
            result
                .plan
                .engine_features
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected_engine_features,
            "{case}"
        );
        assert_eq!(
            result
                .plan
                .runtime_features
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected_runtime_features,
            "{case}"
        );
        let catalog = result.catalog_source.unwrap();
        assert!(
            catalog.contains("mech_engine::install_intrinsic_resident"),
            "{case}"
        );
        assert_eq!(
            catalog.matches("__mech_native::install_").count(),
            0,
            "{case}"
        );

        assert_eq!(result.stdout.unwrap().trim(), expected_output);
    }
}

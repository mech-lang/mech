#![cfg(feature = "full-hosts")]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

pub mod support;

use std::fs;

use mech_core::{BytecodeInstruction, ParsedProgram};
use support::*;

#[test]
fn scalar_add_native_application_uses_only_resident_artifact_authority() {
    let fixture = fixture_path("scalar-add-f64.mecb");
    let bytecode = fs::read(&fixture).unwrap();
    let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
    assert!(parsed.instructions.is_empty());
    assert!(
        parsed
            .instructions
            .iter()
            .filter_map(BytecodeInstruction::runtime_function)
            .next()
            .is_none()
    );
    assert_eq!(
        mech_engine::decode_program_artifact_bytecode_v1(&bytecode)
            .unwrap()
            .operation_references()
            .into_iter()
            .map(|operation| operation.canonical_name())
            .collect::<Vec<_>>(),
        ["math/add".to_owned()]
    );

    let result = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Build,
        "scalar",
        fixture,
        "native_scalar",
        false,
    );

    assert!(!result.poisoned_output_seed);
    assert_eq!(result.poisoned_output_seed_count, 0);
    assert!(result.plan.runtime_functions.is_empty());
    assert!(result.plan.runtime_types.is_empty());
    assert_exact_mech_packages(&result.plan, &["mech-core", "mech-engine", "mech-runtime"]);
    assert_eq!(
        result
            .plan
            .core_features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["f64", "program"]
    );
    assert_eq!(
        result
            .plan
            .engine_features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["f64", "runtime"]
    );
    assert_eq!(
        result
            .plan
            .runtime_features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["f64", "resident-routing", "runtime", "string"]
    );
    let catalog = result.catalog_source.unwrap();
    assert!(catalog.contains("mech_engine::install_intrinsic_resident"));
    assert_eq!(catalog.matches("__mech_native::install_").count(), 0);

    assert_eq!(result.stdout.unwrap().trim(), "3");
}

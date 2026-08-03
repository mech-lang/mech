#![cfg(feature = "standard-hosts")]

mod support;

use mech_runtime::LogLevel;
use support::*;

#[test]
fn materialize_every_phase_one_generated_project() {
    let cases: [(OwnerProfile, &str, &str, &str, bool); 6] = [
        (
            OwnerProfile::Standard,
            "literal",
            "literal-f64.mecb",
            "phase1_native_literal",
            false,
        ),
        (
            OwnerProfile::Standard,
            "scalar",
            "scalar-add-f64.mecb",
            "phase1_native_scalar",
            true,
        ),
        (
            OwnerProfile::Fixed,
            "fixed",
            "fixed-matrix-add-f64.mecb",
            "phase1_native_fixed_matrix",
            true,
        ),
        (
            OwnerProfile::Standard,
            "dynamic",
            "dynamic-matrix-add-f64.mecb",
            "phase1_native_dynamic_matrix",
            true,
        ),
        (
            OwnerProfile::Standard,
            "variadic",
            "variadic-horzcat-f64.mecb",
            "phase1_native_variadic",
            true,
        ),
        (
            OwnerProfile::Standard,
            "cli",
            "cli-stdout.mecb",
            "phase1_native_cli_hosted",
            false,
        ),
    ];

    for (profile, case, fixture, binary_name, poison) in cases {
        let first = run_owner(
            profile,
            RunnerAction::Generate,
            case,
            fixture_path(fixture),
            binary_name,
            poison,
        );
        let second = run_owner(
            profile,
            RunnerAction::Generate,
            case,
            fixture_path(fixture),
            binary_name,
            poison,
        );
        assert_eq!(first, second);
        let project_root = first.project_root.as_ref().unwrap();
        assert!(project_root.join("Cargo.lock").is_file());

        if case == "cli" {
            assert_eq!(first.plan.runtime_config.name, "phase1-generated-runtime");
            assert_eq!(
                first.plan.runtime_config.limits.max_steps_per_turn,
                Some(321)
            );
            assert!(first.plan.runtime_config.diagnostics.trace_enabled);
            assert_eq!(
                first.plan.runtime_config.diagnostics.log_level,
                LogLevel::Debug
            );
            let build_plan = first.build_plan_json.as_ref().unwrap();
            assert!(build_plan.contains("phase1-generated-runtime"));
            assert!(build_plan.contains("\"max_steps_per_turn\": 321"));
            assert!(build_plan.contains("\"trace_enabled\": true"));
            assert!(build_plan.contains("\"log_level\": \"Debug\""));
            let runtime = first.runtime_source.as_ref().unwrap();
            assert!(runtime.contains("\"phase1-generated-runtime\".to_string()"));
            assert!(runtime.contains("max_steps_per_turn: Some(321u64)"));
            assert!(runtime.contains("trace_enabled: true"));
            assert!(runtime.contains("log_level: LogLevel::Debug"));
        }

        println!("MECH_NATIVE_PROJECT={}", project_root.display());
    }
}

#![cfg(feature = "standard-hosts")]

mod support;

use mech_runtime::LogLevel;
use support::*;

#[test]
fn materialize_every_generated_project() {
    let temporary = tempfile::tempdir().unwrap();
    for generated in generated_cases() {
        let fixture = temporary.path().join(format!("{}.mecb", generated.case));
        std::fs::write(&fixture, generated.bytecode).unwrap();
        let first = run_owner(
            generated.profile,
            RunnerAction::Generate,
            generated.case,
            &fixture,
            generated.binary_name,
            false,
        );
        let second = run_owner(
            generated.profile,
            RunnerAction::Generate,
            generated.case,
            &fixture,
            generated.binary_name,
            false,
        );
        assert_eq!(first, second);
        let project_root = first.project_root.as_ref().unwrap();
        assert!(project_root.join("Cargo.lock").is_file());

        if generated.case == "cli" {
            assert_eq!(first.plan.runtime_config.name, "native-generated-runtime");
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
            assert!(build_plan.contains("native-generated-runtime"));
            assert!(build_plan.contains("\"max_steps_per_turn\": 321"));
            assert!(build_plan.contains("\"trace_enabled\": true"));
            assert!(build_plan.contains("\"log_level\": \"Debug\""));
            let runtime = first.runtime_source.as_ref().unwrap();
            assert!(runtime.contains("\"native-generated-runtime\".to_string()"));
            assert!(runtime.contains("max_steps_per_turn: Some(321u64)"));
            assert!(runtime.contains("trace_enabled: true"));
            assert!(runtime.contains("log_level: LogLevel::Debug"));
        }

        println!("MECH_NATIVE_PROJECT={}", project_root.display());
    }
}

#![cfg(feature = "full-hosts")]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

pub mod support;

use mech_runtime::LogLevel;
use support::*;

#[test]
fn materialize_every_generated_project() {
    let temporary = tempfile::tempdir().unwrap();
    let selected_case = std::env::var("MECH_NATIVE_GENERATED_CASE").ok();
    let mut matched = false;
    for generated in generated_cases().into_iter().filter(|generated| {
        selected_case
            .as_deref()
            .is_none_or(|selected| selected == generated.case)
    }) {
        matched = true;
        eprintln!(
            "MECH_NATIVE_GENERATION_PROGRESS case={} profile={:?} progress=start",
            generated.case, generated.profile
        );
        let fixture = temporary.path().join(format!("{}.mecb", generated.case));
        std::fs::write(&fixture, generated.bytecode).unwrap();
        let result = run_owner(
            generated.profile,
            RunnerAction::Generate,
            generated.case,
            &fixture,
            generated.binary_name,
            false,
        );
        let project_root = result.project_root.as_ref().unwrap();
        assert!(project_root.join("Cargo.lock").is_file());

        if generated.case == "cli" {
            assert_eq!(result.plan.runtime_config.name, "native-generated-runtime");
            assert_eq!(
                result.plan.runtime_config.limits.max_steps_per_turn,
                Some(321)
            );
            assert!(result.plan.runtime_config.diagnostics.trace_enabled);
            assert_eq!(
                result.plan.runtime_config.diagnostics.log_level,
                LogLevel::Debug
            );
            let build_plan = result.build_plan_json.as_ref().unwrap();
            assert!(build_plan.contains("native-generated-runtime"));
            assert!(build_plan.contains("\"max_steps_per_turn\": 321"));
            assert!(build_plan.contains("\"trace_enabled\": true"));
            assert!(build_plan.contains("\"log_level\": \"Debug\""));
            let runtime = result.runtime_source.as_ref().unwrap();
            assert!(runtime.contains("\"native-generated-runtime\".to_string()"));
            assert!(runtime.contains("max_steps_per_turn: Some(321u64)"));
            assert!(runtime.contains("trace_enabled: true"));
            assert!(runtime.contains("log_level: LogLevel::Debug"));
        }

        println!(
            "MECH_NATIVE_PROJECT_CASE={}\t{}\t{}",
            generated.case,
            generated.binary_name,
            project_root.display()
        );
        println!("MECH_NATIVE_PROJECT={}", project_root.display());
        eprintln!(
            "MECH_NATIVE_GENERATION_PROGRESS case={} profile={:?} progress=complete",
            generated.case, generated.profile
        );
    }
    assert!(
        selected_case.is_none() || matched,
        "MECH_NATIVE_GENERATED_CASE did not name a generated case"
    );
}

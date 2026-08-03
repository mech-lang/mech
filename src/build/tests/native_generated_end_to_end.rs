#![cfg(feature = "standard-hosts")]

mod support;

use support::*;

#[test]
fn every_generated_application_fixture_builds_and_executes() {
    let temporary = tempfile::tempdir().unwrap();
    let selected = std::env::var("MECH_NATIVE_GENERATED_CASE").ok();
    for generated in generated_cases() {
        if selected
            .as_deref()
            .is_some_and(|selected| selected != generated.case)
        {
            continue;
        }
        let fixture = temporary.path().join(format!("{}.mecb", generated.case));
        std::fs::write(&fixture, generated.bytecode).unwrap();
        let action = if generated.case == "actor" {
            // Actor host functions execute in an actor turn with a capability
            // grant; a generated application has no synthetic actor context.
            // Still build its complete graph to prove native linkage.
            RunnerAction::BuildOnly
        } else {
            RunnerAction::Build
        };
        let result = run_owner(
            generated.profile,
            action,
            generated.case,
            fixture,
            generated.binary_name,
            false,
        );
        if generated.case == "actor" {
            assert!(result.executable.unwrap().is_file());
            assert!(
                result
                    .runtime_source
                    .unwrap()
                    .contains("install_actor_state_id")
            );
        } else {
            assert_eq!(
                result.stdout.unwrap().trim(),
                generated.expected_stdout,
                "{}",
                generated.case,
            );
        }
    }
}

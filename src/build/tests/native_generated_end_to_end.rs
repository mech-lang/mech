#![cfg(feature = "full-hosts")]

pub mod support;

use mech_build::PlannedApplicationRequirement;
use support::*;

#[test]
fn every_generated_application_fixture_builds_and_executes() {
    let temporary = tempfile::tempdir().unwrap();
    let selected = std::env::var("MECH_NATIVE_GENERATED_CASE").ok();
    for generated in generated_cases() {
        if selected.as_deref().is_some_and(|selected| {
            !selected
                .split(',')
                .map(str::trim)
                .any(|selected| selected == generated.case)
        }) {
            continue;
        }
        let fixture = temporary.path().join(format!("{}.mecb", generated.case));
        std::fs::write(&fixture, generated.bytecode).unwrap();
        let result = run_owner(
            generated.profile,
            RunnerAction::Build,
            generated.case,
            fixture,
            generated.binary_name,
            false,
        );
        assert_eq!(
            result.stdout.unwrap().trim(),
            generated.expected_stdout,
            "{}",
            generated.case,
        );
        if let Some((requested, operation, host_instance, host_context)) = match generated.case {
            "cli" => Some(("cli://stdout", "write", "cli", "stdout")),
            "console" => Some(("console://output", "write", "console", "output")),
            "robot-arm" => Some(("robot://arm/commands", "move", "arm", "commands")),
            _ => None,
        } {
            assert!(
                result
                    .plan
                    .application_requirements
                    .iter()
                    .any(|requirement| {
                        matches!(
                            requirement,
                            PlannedApplicationRequirement::Resource { request, owner }
                                if request.base_uri == requested
                                    && request.operation == operation
                                    && owner.host_instance == host_instance
                                    && owner.host_context == host_context
                        )
                    })
            );
            assert!(result.plan.run_grants.iter().any(|grant| {
                grant.host_instance == host_instance && grant.host_context == host_context
            }));
        }
    }
}

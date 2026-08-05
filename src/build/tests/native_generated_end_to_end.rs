#![cfg(feature = "standard-hosts")]

mod support;

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
        if let Some((requested, host_instance, host_context)) = match generated.case {
            "cli" => Some(("cli://stdout", "cli", "stdout")),
            "console" => Some(("console://output", "console", "output")),
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
                                    && owner.host_instance == host_instance
                                    && owner.host_context == host_context
                        )
                    })
            );
            assert!(result.plan.run_grants.iter().any(|grant| {
                grant.host_instance == host_instance && grant.host_context == host_context
            }));
        }
        if generated.case.starts_with("actor-") {
            let runtime = result.runtime_source.unwrap();
            for installer in [
                "install_actor_message_kind",
                "install_actor_message_payload",
                "install_actor_state_id",
                "install_actor_state_get",
                "install_actor_state_put",
            ] {
                assert!(runtime.contains(installer));
            }
        }
    }
}

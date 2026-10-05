#![cfg(feature = "full-hosts")]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

pub mod support;

use std::collections::BTreeSet;

use mech_build::PlannedApplicationRequirement;
use support::*;

#[test]
fn every_generated_application_fixture_builds_and_executes() {
    let temporary = tempfile::tempdir().unwrap();
    let selected = std::env::var("MECH_NATIVE_GENERATED_CASE").ok();
    let requested = selected.as_deref().map(|selected| {
        let requested = selected
            .split(',')
            .map(str::trim)
            .filter(|case| !case.is_empty())
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        assert!(
            !requested.is_empty(),
            "MECH_NATIVE_GENERATED_CASE must name at least one generated case"
        );
        requested
    });
    let mut matched = BTreeSet::new();
    for generated in generated_cases() {
        if requested
            .as_ref()
            .is_some_and(|requested| !requested.contains(generated.case))
        {
            continue;
        }
        matched.insert(generated.case.to_owned());
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
    if let Some(requested) = requested {
        assert_eq!(
            matched, requested,
            "MECH_NATIVE_GENERATED_CASE included an unknown or retired generated case"
        );
    }
}

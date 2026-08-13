#![cfg(feature = "full-hosts")]

mod support;

use mech_build::PlannedApplicationRequirement;
use support::*;

#[test]
fn cli_hosted_native_application_builds_and_emits_once() {
    let temporary = tempfile::tempdir().unwrap();
    let alias_case = generated_cli_alias_case();
    let bytecode = temporary.path().join("cli-alias.mecb");
    std::fs::write(&bytecode, alias_case.bytecode).unwrap();
    let result = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Build,
        "cli",
        bytecode,
        "native_cli_hosted",
        false,
    );

    assert_exact_mech_packages(
        &result.plan,
        &["mech-core", "mech-engine", "mech-runtime", "mech-terminal"],
    );
    assert_eq!(result.plan.runtime_config.name, "native-generated-runtime");
    assert_eq!(
        result.plan.runtime_config.limits.max_steps_per_turn,
        Some(321)
    );
    assert!(result.plan.runtime_config.diagnostics.trace_enabled);
    assert!(
        result
            .plan
            .application_requirements
            .iter()
            .any(|requirement| {
                matches!(
                    requirement,
                    PlannedApplicationRequirement::Resource {
                        request,
                        owner,
                    } if request.base_uri == "cli://stdout"
                        && request.context_name == "stdout"
                        && owner.host_instance == "cli"
                        && owner.canonical_base_uri == "cli://cli/stdout"
                )
            })
    );
    assert_eq!(result.plan.run_grants.len(), 1);
    assert_eq!(result.plan.run_grants[0].host_instance, "cli");
    assert_eq!(result.plan.run_grants[0].host_context, "stdout");
    let runtime = result.runtime_source.unwrap();
    assert!(runtime.contains("mech_terminal::CliHostFactory::new"));
    assert_eq!(runtime.matches("CliHostFactory::new").count(), 1);

    let output = result.stdout.unwrap();
    assert_eq!(output.matches("phase1-hosted-ok").count(), 1);
}

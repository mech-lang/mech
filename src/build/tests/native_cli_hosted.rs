#![cfg(feature = "standard-hosts")]

mod support;

use support::*;

#[test]
fn cli_hosted_native_application_builds_and_emits_once() {
    let result = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Build,
        "cli",
        fixture_path("cli-stdout.mecb"),
        "native_cli_hosted",
        false,
    );

    assert_exact_mech_packages(
        &result.plan,
        &["mech-core", "mech-engine", "mech-host-cli", "mech-runtime"],
    );
    assert_eq!(result.plan.runtime_config.name, "native-generated-runtime");
    assert_eq!(
        result.plan.runtime_config.limits.max_steps_per_turn,
        Some(321)
    );
    assert!(result.plan.runtime_config.diagnostics.trace_enabled);
    let runtime = result.runtime_source.unwrap();
    assert!(runtime.contains("mech_host_cli::CliHostFactory::new"));
    assert_eq!(runtime.matches("CliHostFactory::new").count(), 1);

    let output = result.stdout.unwrap();
    assert_eq!(output.matches("phase1-hosted-ok").count(), 1);
}

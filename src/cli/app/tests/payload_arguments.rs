use super::super::{build_cli, root_flags};
use crate::cli::run::RunInputMode;
use crate::cli::run_options::{RunCliArgs, prepare_run_options};
use crate::cli::runtime_plan::build_run_execution_plan;
use std::ffi::OsString;

fn parse(arguments: Vec<OsString>) -> (clap::ArgMatches, RunCliArgs) {
    let matches = build_cli().try_get_matches_from(arguments).unwrap();
    let args = RunCliArgs::from_matches(
        root_flags(&matches),
        &matches,
        matches.subcommand_matches("run"),
    )
    .unwrap();
    (matches, args)
}

fn arguments(values: &[&str]) -> Vec<OsString> {
    values.iter().map(|value| OsString::from(*value)).collect()
}

#[test]
fn explicit_and_bare_runs_have_the_same_exact_payload_boundary() {
    for prefix in [
        vec!["mech", "run", "program.mec"],
        vec!["mech", "program.mec"],
    ] {
        let mut command = arguments(&prefix);
        command.extend(arguments(&[
            "--",
            "",
            "--",
            "-2",
            "--runtime-info",
            "other.mec",
        ]));
        let (_, parsed) = parse(command);
        assert_eq!(
            parsed.input_mode,
            RunInputMode::Paths(vec!["program.mec".into()])
        );
        assert_eq!(
            parsed.payload_arguments,
            arguments(&["", "--", "-2", "--runtime-info", "other.mec"])
        );
        assert!(!parsed.runtime_info);
        assert_eq!(parsed.explicit_run_command, prefix.contains(&"run"));
    }
}

#[test]
fn source_selectors_are_not_guessed_to_be_payload_without_the_separator() {
    for values in [
        vec!["mech", "run", "first.mec", "second.mec"],
        vec!["mech", "first.mec", "second.mec"],
    ] {
        let (_, parsed) = parse(arguments(&values));
        assert_eq!(
            parsed.input_mode,
            RunInputMode::Paths(vec!["first.mec".into(), "second.mec".into()])
        );
        assert!(parsed.payload_arguments.is_empty());
    }
    assert!(
        build_cli()
            .try_get_matches_from(["mech", "run", "program.mec", "--not-a-control"])
            .is_err()
    );
}

#[test]
fn payload_flags_cannot_select_backends_or_grant_capabilities() {
    let (matches, parsed) = parse(arguments(&[
        "mech",
        "run",
        "--no-config",
        "--deny-default-capabilities",
        "program.mec",
        "--",
        "--capabilities",
        ":cli/args",
        "--cap-root",
        "outside",
        "--config",
        "secret.mcfg",
        "--backend",
        "not-a-backend",
        "--rounds-per-step",
        "999",
        "--runtime-info",
    ]));
    assert!(!parsed.cli_capability_selection.include_defaults);
    assert!(parsed.cli_capability_selection.profiles.is_empty());
    assert!(parsed.backend_override.is_none());
    assert!(parsed.rounds_per_step.is_none());
    assert!(!parsed.runtime_info);
    let run = matches.subcommand_matches("run").unwrap();
    assert!(run.get_flag("no_config"));
    assert!(run.get_one::<String>("config").is_none());
    let filesystem = crate::cli::capabilities::FilesystemCapabilityArgs::from_matches(run);
    assert!(filesystem.cap_roots.is_empty());
    assert_eq!(
        parsed.payload_arguments,
        arguments(&[
            "--capabilities",
            ":cli/args",
            "--cap-root",
            "outside",
            "--config",
            "secret.mcfg",
            "--backend",
            "not-a-backend",
            "--rounds-per-step",
            "999",
            "--runtime-info",
        ])
    );
}

#[test]
fn prepared_options_and_execution_plan_keep_payload_separate_from_inline_source() {
    let payload = arguments(&["", "looks-like-a-source.mec", "--trace"]);
    let mut command = arguments(&["mech", "run", "--no-config", "value := 1", "--"]);
    command.extend(payload.clone());
    let (matches, parsed) = parse(command);
    assert_eq!(
        parsed.input_mode,
        RunInputMode::InlineSource("value := 1".into())
    );
    let prepared = prepare_run_options(parsed, matches.subcommand_matches("run").unwrap()).unwrap();
    assert_eq!(prepared.payload_arguments, payload);
    let plan = build_run_execution_plan(prepared).unwrap();
    assert_eq!(plan.payload_arguments, payload);
    assert!(plan.run_paths.is_empty());
    assert!(!plan.missing_run_options);
    assert!(!plan.runtime_config.diagnostics.trace_enabled);
}

#[test]
fn targetless_payload_is_not_an_accidental_repl_or_a_source_selector() {
    assert!(
        build_cli()
            .try_get_matches_from(["mech", "--", "payload"])
            .is_err()
    );
    // Explicit run can still obtain its target from run.paths in configuration.
    let (_, parsed) = parse(arguments(&["mech", "run", "--", "payload"]));
    assert_eq!(parsed.input_mode, RunInputMode::Empty);
    assert!(parsed.explicit_run_command);
    assert_eq!(parsed.payload_arguments, arguments(&["payload"]));
}

#[cfg(unix)]
#[test]
fn non_unicode_payload_survives_parsing_preparation_and_execution_planning() {
    use std::os::unix::ffi::OsStringExt;
    let raw = OsString::from_vec(vec![0xff, 0x80, b'-']);
    for prefix in [
        vec!["mech", "run", "--no-config", "value := 1"],
        vec!["mech", "--no-config", "value := 1"],
    ] {
        let mut command = arguments(&prefix);
        command.extend([OsString::from("--"), raw.clone()]);
        let (matches, parsed) = parse(command);
        assert_eq!(parsed.payload_arguments, vec![raw.clone()]);
        let controls = matches.subcommand_matches("run").unwrap_or(&matches);
        let prepared = prepare_run_options(parsed, controls).unwrap();
        let plan = build_run_execution_plan(prepared).unwrap();
        assert_eq!(plan.payload_arguments, vec![raw.clone()]);
    }
}

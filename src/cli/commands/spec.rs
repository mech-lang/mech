use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command};
use mech_core::*;

use crate::cli::outcome::CliOutcome;

const DEFAULT_SPEC: &str = "spec/platform.mspec";
const DEFAULT_JSON: &str = "target/spec/results.json";
const DEFAULT_HTML: &str = "target/spec/conformance.html";
const DEFAULT_STORE: &str = ".mech/determinism";

pub(crate) fn command() -> Command {
    Command::new("spec")
        .about("Check and replay the executable Mech platform specification.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(report_command(Command::new("check").about(
            "Observe the v0.4 resident executor and evaluate all claims.",
        )))
        .subcommand(
            report_command(
                Command::new("record")
                    .about("Check the system and record a content-addressed evidence bundle."),
            )
            .arg(
                Arg::new("store")
                    .long("store")
                    .value_name("PATH")
                    .help("Content-addressed determinism store.")
                    .default_value(DEFAULT_STORE),
            ),
        )
        .subcommand(
            Command::new("replay")
                .about("Replay judgments from a stored bundle without rerunning the runtime.")
                .arg(
                    Arg::new("bundle_hash")
                        .value_name("BUNDLE_HASH")
                        .help("Bundle hash printed by `mech spec record`.")
                        .required(true),
                )
                .arg(
                    Arg::new("store")
                        .long("store")
                        .value_name("PATH")
                        .help("Content-addressed determinism store.")
                        .default_value(DEFAULT_STORE),
                ),
        )
        .subcommand(
            Command::new("demo")
                .about("Run intentional semantic, architecture, backend, and document failures.")
                .arg(
                    Arg::new("spec_path")
                        .value_name("SPEC")
                        .help("Primary executable .mspec document.")
                        .default_value(DEFAULT_SPEC),
                )
                .arg(
                    Arg::new("output_path")
                        .short('o')
                        .long("out")
                        .value_name("PATH")
                        .help("Machine-readable demonstration result path.")
                        .default_value("target/spec/demonstration.json"),
                )
                .arg(
                    Arg::new("store")
                        .long("store")
                        .value_name("PATH")
                        .help("Store used to prove mutant replay determinism.")
                        .default_value("target/spec/demo-determinism"),
                ),
        )
}

fn report_command(command: Command) -> Command {
    command
        .arg(
            Arg::new("spec_path")
                .value_name("SPEC")
                .help("Primary executable .mspec document.")
                .default_value(DEFAULT_SPEC),
        )
        .arg(
            Arg::new("output_path")
                .short('o')
                .long("out")
                .value_name("PATH")
                .help("Machine-readable JSON result path.")
                .default_value(DEFAULT_JSON),
        )
        .arg(
            Arg::new("html_path")
                .long("html")
                .value_name("PATH")
                .help("Linked human-readable HTML result path.")
                .default_value(DEFAULT_HTML),
        )
}

pub(crate) enum SpecOptions {
    Check(ReportOptions),
    Record {
        report: ReportOptions,
        store: PathBuf,
    },
    Replay {
        bundle_hash: String,
        store: PathBuf,
    },
    Demo {
        spec_path: PathBuf,
        output_path: PathBuf,
        store: PathBuf,
    },
}

pub(crate) struct ReportOptions {
    spec_path: PathBuf,
    output_path: PathBuf,
    html_path: PathBuf,
}

impl SpecOptions {
    pub(crate) fn from_matches(matches: &ArgMatches) -> MResult<Self> {
        match matches.subcommand() {
            Some(("check", check)) => Ok(Self::Check(ReportOptions::from_matches(check))),
            Some(("record", record)) => Ok(Self::Record {
                report: ReportOptions::from_matches(record),
                store: value_path(record, "store"),
            }),
            Some(("replay", replay)) => Ok(Self::Replay {
                bundle_hash: replay
                    .get_one::<String>("bundle_hash")
                    .expect("bundle_hash is required")
                    .clone(),
                store: value_path(replay, "store"),
            }),
            Some(("demo", demo)) => Ok(Self::Demo {
                spec_path: value_path(demo, "spec_path"),
                output_path: value_path(demo, "output_path"),
                store: value_path(demo, "store"),
            }),
            _ => Err(spec_error(mech_spec::SpecError::new(
                "missing `mech spec` subcommand",
            ))),
        }
    }
}

impl ReportOptions {
    fn from_matches(matches: &ArgMatches) -> Self {
        Self {
            spec_path: value_path(matches, "spec_path"),
            output_path: value_path(matches, "output_path"),
            html_path: value_path(matches, "html_path"),
        }
    }
}

fn value_path(matches: &ArgMatches, name: &str) -> PathBuf {
    PathBuf::from(
        matches
            .get_one::<String>(name)
            .unwrap_or_else(|| panic!("{name} has a default")),
    )
}

pub(crate) fn run(options: SpecOptions) -> MResult<CliOutcome> {
    match options {
        SpecOptions::Check(options) => {
            let report = mech_spec::check(&options.spec_path).map_err(spec_error)?;
            write_reports(&report, &options)?;
            println!("{}", report.render_text());
            println!("  json:        {}", options.output_path.display());
            println!("  html:        {}", options.html_path.display());
            Ok(CliOutcome::exit(if report.passed { 0 } else { 1 }))
        }
        SpecOptions::Record { report, store } => {
            let bundle = mech_spec::record(&report.spec_path, &store).map_err(spec_error)?;
            write_reports(&bundle.report, &report)?;
            println!("{}", bundle.report.render_text());
            println!("  bundle:      {}", bundle.bundle_hash);
            println!("  manifest:    {}", bundle.manifest_path.display());
            println!("  json:        {}", report.output_path.display());
            println!("  html:        {}", report.html_path.display());
            Ok(CliOutcome::exit(if bundle.report.passed { 0 } else { 1 }))
        }
        SpecOptions::Replay { bundle_hash, store } => {
            let replay = mech_spec::replay(&bundle_hash, &store).map_err(spec_error)?;
            println!("{}", replay.render_text());
            Ok(CliOutcome::exit(if replay.passed { 0 } else { 1 }))
        }
        SpecOptions::Demo {
            spec_path,
            output_path,
            store,
        } => {
            let report = mech_spec::demonstrate(&spec_path, &store).map_err(spec_error)?;
            mech_spec::write_demonstration_report(&report, &output_path).map_err(spec_error)?;
            println!("{}", report.render_text());
            println!("  report: {}", output_path.display());
            Ok(CliOutcome::exit(if report.passed { 0 } else { 1 }))
        }
    }
}

fn write_reports(report: &mech_spec::CheckReport, options: &ReportOptions) -> MResult<()> {
    mech_spec::write_json_report(report, &options.output_path).map_err(spec_error)?;
    mech_spec::write_html_report(report, &options.html_path).map_err(spec_error)
}

fn spec_error(error: mech_spec::SpecError) -> MechError {
    MechError::new(
        GenericError {
            msg: error.to_string(),
        },
        None,
    )
}

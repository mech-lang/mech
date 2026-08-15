use std::path::PathBuf;

use clap::{Arg, ArgMatches, Command};
use mech_core::*;

use crate::cli::outcome::CliOutcome;

pub(crate) fn command() -> Command {
    Command::new("spec")
        .about("Check the executable Mech platform specification.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Collect v0.4 resident-executor observations and evaluate contracts.")
                .arg(
                    Arg::new("spec_path")
                        .help("Specification tree to evaluate.")
                        .default_value("spec"),
                )
                .arg(
                    Arg::new("output_path")
                        .short('o')
                        .long("out")
                        .help("JSON result path.")
                        .default_value("target/spec/results.json"),
                ),
        )
}

pub(crate) struct SpecCheckOptions {
    spec_path: PathBuf,
    output_path: PathBuf,
}

impl SpecCheckOptions {
    pub(crate) fn from_matches(matches: &ArgMatches) -> MResult<Self> {
        let check = matches.subcommand_matches("check").ok_or_else(|| {
            MechError::new(
                GenericError {
                    msg: "missing `mech spec` subcommand".to_string(),
                },
                None,
            )
        })?;
        Ok(Self {
            spec_path: PathBuf::from(
                check
                    .get_one::<String>("spec_path")
                    .expect("spec_path has a default"),
            ),
            output_path: PathBuf::from(
                check
                    .get_one::<String>("output_path")
                    .expect("output_path has a default"),
            ),
        })
    }
}

pub(crate) fn run(options: SpecCheckOptions) -> MResult<CliOutcome> {
    let report = mech_spec::check(&options.spec_path).map_err(spec_error)?;
    mech_spec::write_json_report(&report, &options.output_path).map_err(spec_error)?;
    println!("{}", report.render_text());
    println!("  report:      {}", options.output_path.display());
    Ok(CliOutcome::exit(if report.passed { 0 } else { 1 }))
}

fn spec_error(error: mech_spec::SpecError) -> MechError {
    MechError::new(
        GenericError {
            msg: error.to_string(),
        },
        None,
    )
}

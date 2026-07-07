use clap::{ArgMatches, Command};
use colored::*;
use mech_core::*;

use crate::cli::outcome::CliOutcome;

pub(crate) fn command() -> Command {
    crate::cli::bundle_web::bundle_web_command()
}

pub(crate) fn add_config_args(command: Command) -> Command {
    crate::cli::bundle_web::add_config_args(command)
}

pub(crate) struct BundleWebCliOptions {
    pub matches: ArgMatches,
}

impl BundleWebCliOptions {
    pub(crate) fn from_matches(matches: &ArgMatches) -> MResult<Self> {
        Ok(Self {
            matches: matches.clone(),
        })
    }
}

pub(crate) fn run(options: BundleWebCliOptions) -> MResult<CliOutcome> {
    let matches = &options.matches;
    let badge = "[Mech Bundle]".truecolor(34, 204, 187);

    let loaded = crate::cli::bundle_web::load_bundle_web_config(matches)?;
    println!("{badge} Loading config… {}", loaded.path.display());

    let options = crate::cli::bundle_web::effective_bundle_web_options(matches, loaded)?;
    let result = crate::bundle_web_project(options)?;

    println!("{badge} Bundle written: {}", result.output_dir.display());
    println!("{badge} Sources bundled: {}", result.source_count);

    Ok(CliOutcome::success())
}

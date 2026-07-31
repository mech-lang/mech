use clap::Command;
use colored::*;
use mech_core::*;

use crate::cli::outcome::CliOutcome;

pub(crate) fn command() -> Command {
    crate::cli::bundle_web::bundle_web_command()
}

pub(crate) fn add_config_args(command: Command) -> Command {
    crate::cli::bundle_web::add_config_args(command)
}

pub(crate) use crate::cli::bundle_web::BundleWebCliArgs;

pub(crate) struct BundleWebPlan {
    pub options: crate::BundleWebOptions,
}

pub(crate) fn prepare(args: BundleWebCliArgs) -> MResult<BundleWebPlan> {
    let loaded = crate::cli::bundle_web::load_bundle_web_config_from_args(&args)?;
    println!(
        "{} Loading config… {}",
        "[Mech Bundle]".truecolor(34, 204, 187),
        loaded.path.display()
    );
    let options = crate::cli::bundle_web::effective_bundle_web_options_from_args(&args, loaded)?;
    Ok(BundleWebPlan { options })
}

pub(crate) fn run(options: BundleWebPlan) -> MResult<CliOutcome> {
    let badge = "[Mech Bundle]".truecolor(34, 204, 187);
    let result = crate::bundle_web_project(options.options)?;

    println!("{badge} Bundle written: {}", result.output_dir.display());
    println!("{badge} Sources bundled: {}", result.source_count);

    Ok(CliOutcome::success())
}

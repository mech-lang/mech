use clap::{Arg, ArgAction, ArgMatches, Command};
use mech_core::*;

#[cfg(any(feature = "serve", feature = "run"))]
use crate::cli::capabilities;
#[cfg(any(feature = "build", feature = "serve", feature = "run"))]
use crate::cli::config;
use crate::cli::outcome::{CliOutcome, RootFlags};
#[cfg(any(feature = "formatter", feature = "serve"))]
use crate::cli::resources::WebResourceDefaults;

#[cfg(feature = "distribution-full")]
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (full)");
#[cfg(all(feature = "distribution-standard", not(feature = "distribution-full")))]
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (standard)");
#[cfg(not(any(feature = "distribution-standard", feature = "distribution-full")))]
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (custom)");
const ROOT_LOGO: &str = "Mech";
#[cfg(feature = "serve")]
const FILESYSTEM_CAPABILITY_FLAGS_UNSUPPORTED: &str = "filesystem capability flags are only supported by `mech run`, bare run inputs, and `mech serve`";

pub(crate) fn terminate_process(code: i32) -> ! {
    std::process::exit(code)
}

pub fn run() -> MResult<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

pub(crate) fn build_cli() -> Command {
    let cli_command = Command::new("Mech")
        .subcommand_precedence_over_arg(true)
        .version(VERSION)
        .author("Corey Montella corey@mech-lang.org")
        .about(ROOT_LOGO)
        .arg(
            Arg::new("mech_paths")
                .help(
                    "Source .mec files, .mecb bytecode files, project folders, or inline Mech code",
                )
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("repl")
                .long("repl")
                .help("Start the interactive Mech REPL")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("nofun")
                .long("nofun")
                .help("Use a plain inline REPL (also MECH_NOFUN=1 or MECH_REPL_STYLE=plain)")
                .long_help(
                    "Use a bare inline REPL without colors, decorative formatting, animations, Mika, or the Mech logo. Also selected by MECH_NOFUN=1, MECH_REPL_STYLE=plain, or TERM=dumb. Rich mode respects NO_COLOR, CLICOLOR=0, and CI.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .help("Suppress automatic REPL source and value echo")
                .long_help(
                    "Suppress automatic REPL source and value echo for every entry. Explicit commands, program output, and diagnostics remain visible. Also selected by MECH_REPL_QUIET=1 or MECH_QUIET=1. A trailing semicolon suppresses one entry without enabling quiet mode.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Print debug info")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("rounds-per-step")
                .long("rounds-per-step")
                .value_name("ROUNDS")
                .value_parser(crate::cli::rounds_per_step_value_parser())
                .help("Sets the number of rounds per step. Must be a positive integer.")
                .required(false),
        )
        .arg(
            Arg::new("trace")
                .long("trace")
                .help("Print trace output for state-machine arms and function calls")
                .action(ArgAction::SetTrue),
        );

    #[cfg(feature = "formatter")]
    let cli_command = cli_command.subcommand(crate::cli::commands::format::command());
    #[cfg(feature = "build")]
    let cli_command = cli_command.subcommand(crate::cli::commands::build::command());
    #[cfg(feature = "run")]
    let cli_command = cli_command.subcommand(crate::cli::commands::run::command());
    #[cfg(feature = "serve")]
    let cli_command = cli_command.subcommand(crate::cli::commands::serve::command());
    #[cfg(feature = "run")]
    let cli_command = crate::cli::commands::run::add_cli_host_capability_args(cli_command);
    #[cfg(feature = "bundle_web")]
    let cli_command = cli_command.subcommand(crate::cli::commands::bundle_web::command());
    #[cfg(any(feature = "serve", feature = "run"))]
    let cli_command = capabilities::add_filesystem_capability_args(cli_command);
    #[cfg(any(feature = "build", feature = "serve", feature = "run"))]
    let cli_command = config::add_config_args(cli_command);
    #[cfg(all(feature = "bundle_web", not(feature = "serve")))]
    let cli_command = crate::cli::commands::bundle_web::add_config_args(cli_command);

    cli_command
}

fn root_flags(cli_matches: &ArgMatches) -> RootFlags {
    RootFlags {
        debug: cli_matches.get_flag("debug"),
        trace: cli_matches.get_flag("trace"),
        rounds_per_step: cli_matches.get_one::<usize>("rounds-per-step").copied(),
    }
}

async fn async_main() -> MResult<()> {
    let cli_matches = build_cli().get_matches();
    let outcome = dispatch(cli_matches).await?;
    apply_outcome(outcome)
}

pub(crate) async fn dispatch(cli_matches: ArgMatches) -> MResult<CliOutcome> {
    let flags = root_flags(&cli_matches);
    #[cfg(any(feature = "formatter", feature = "serve"))]
    let resources = WebResourceDefaults::for_current_package();

    #[cfg(feature = "bundle_web")]
    if let Some(bundle_matches) = cli_matches.subcommand_matches("bundle-web") {
        reject_filesystem_capability_args(bundle_matches)?;
        let args =
            crate::cli::commands::bundle_web::BundleWebCliArgs::from_matches(bundle_matches)?;
        let plan = crate::cli::commands::bundle_web::prepare(args)?;
        return crate::cli::commands::bundle_web::run(plan);
    }

    #[cfg(feature = "serve")]
    if let Some(serve_matches) = cli_matches.subcommand_matches("serve") {
        let args = crate::cli::serve_options::ServeCliArgs::from_matches(serve_matches);
        let plan = crate::cli::commands::serve::prepare(args, serve_matches, resources)?;
        return crate::cli::commands::serve::run(plan).await;
    }

    #[cfg(feature = "build")]
    if let Some(matches) = cli_matches.subcommand_matches("build") {
        reject_filesystem_capability_args(matches)?;
        let options =
            crate::cli::commands::build::BuildOptions::from_matches(flags, &cli_matches, matches)?;
        return crate::cli::commands::build::run(options);
    }

    #[cfg(feature = "formatter")]
    if let Some(matches) = cli_matches.subcommand_matches("format") {
        reject_filesystem_capability_args(matches)?;
        let options = crate::cli::commands::format::FormatOptions::from_matches(
            flags,
            &cli_matches,
            matches,
            resources,
        )?;
        return crate::cli::commands::format::run(options).await;
    }

    // A targetless invocation is the interactive resident REPL. It must be
    // recognized before root arguments fall through to the production run command.
    #[cfg(feature = "run")]
    if is_repl_invocation(&cli_matches) {
        return crate::cli::commands::repl::run(
            cli_matches.get_flag("nofun"),
            cli_matches.get_flag("quiet"),
        );
    }

    // Historical CLI behavior treats unmatched root arguments as run inputs.
    #[cfg(feature = "run")]
    {
        let args = crate::cli::run_options::RunCliArgs::from_matches(
            flags,
            &cli_matches,
            cli_matches.subcommand_matches("run"),
        )?;
        let config_matches = cli_matches
            .subcommand_matches("run")
            .unwrap_or(&cli_matches);
        let options = crate::cli::run_options::prepare_run_options(args, config_matches)?;
        let plan = crate::cli::runtime_plan::build_run_execution_plan(options)?;
        let outcome = crate::cli::commands::run::run(plan)?;
        if !matches!(outcome, CliOutcome::Success) {
            return Ok(outcome);
        }
    }

    Ok(CliOutcome::success())
}

#[cfg(feature = "run")]
fn is_repl_invocation(cli_matches: &ArgMatches) -> bool {
    cli_matches.subcommand_name().is_none()
        && cli_matches.get_many::<String>("mech_paths").is_none()
}

#[cfg(feature = "serve")]
fn reject_filesystem_capability_args(matches: &ArgMatches) -> MResult<()> {
    if capabilities::filesystem_capability_args_present(matches) {
        return Err(MechError::new(
            GenericError {
                msg: FILESYSTEM_CAPABILITY_FLAGS_UNSUPPORTED.to_string(),
            },
            None,
        ));
    }
    Ok(())
}

#[cfg(all(
    not(feature = "serve"),
    any(feature = "build", feature = "bundle_web", feature = "formatter")
))]
fn reject_filesystem_capability_args(_matches: &ArgMatches) -> MResult<()> {
    Ok(())
}

fn apply_outcome(outcome: CliOutcome) -> MResult<()> {
    match outcome {
        CliOutcome::Success => Ok(()),
        CliOutcome::Exit(code) => terminate_process(code),
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

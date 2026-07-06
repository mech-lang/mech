use clap::{Arg, ArgAction, Command};
use mech_core::*;

#[cfg(any(feature = "serve", feature = "run"))]
use crate::cli::capabilities;
#[cfg(any(feature = "serve", feature = "run"))]
use crate::cli::config;
use crate::cli::resources::{MECHJS, MECHWASM, SHIMHTML};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ROOT_LOGO: &str = "Mech";

pub(crate) fn terminate_process(code: i32) -> ! {
    std::process::exit(code)
}

pub fn run() -> Result<(), MechError> {
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
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Print debug info")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tree")
                .short('e')
                .long("tree")
                .help("Print parse tree")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("time")
                .short('t')
                .long("time")
                .help("Measure how long the programs takes to execute.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("rounds-per-step")
                .long("rounds-per-step")
                .value_name("ROUNDS")
                .help("Sets the number of rounds per step (10_000)")
                .required(false),
        )
        .arg(
            Arg::new("trace")
                .long("trace")
                .help("Print trace output for state-machine arms and function calls")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("repl")
                .short('r')
                .long("repl")
                .help("Start REPL")
                .action(ArgAction::SetTrue),
        );

    #[cfg(feature = "formatter")]
    let cli_command = cli_command.subcommand(crate::cli::commands::format::command());
    #[cfg(feature = "build")]
    let cli_command = cli_command.subcommand(crate::cli::commands::build::command());
    #[cfg(feature = "test")]
    let cli_command = cli_command.subcommand(crate::cli::commands::test::command());
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
    #[cfg(any(feature = "serve", feature = "run"))]
    let cli_command = config::add_config_args(cli_command);
    #[cfg(all(feature = "bundle_web", not(feature = "serve")))]
    let cli_command = crate::cli::commands::bundle_web::add_config_args(cli_command);

    cli_command
}

async fn async_main() -> Result<(), MechError> {
    let cli_matches = build_cli().get_matches();

    let debug_flag = cli_matches.get_flag("debug");
    let tree_flag = cli_matches.get_flag("tree");
    let mut repl_flag = cli_matches.get_flag("repl");
    let time_flag = cli_matches.get_flag("time");
    let trace_flag = cli_matches.get_flag("trace");
    let root_rounds_per_step = cli_matches
        .get_one::<String>("rounds-per-step")
        .and_then(|s| s.parse::<usize>().ok());

    let shim_backup_url =
        "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/shim.html"
            .to_string();
    let stylesheet_backup_url =
        "https://raw.githubusercontent.com/mech-lang/mech/refs/heads/main/include/style.css"
            .to_string();
    let wasm_backup_url = format!(
        "https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm_bg.wasm.br",
        VERSION
    );
    let js_backup_url = format!(
        "https://github.com/mech-lang/mech/releases/download/v{}-beta/mech_wasm.js",
        VERSION
    );

    #[cfg(feature = "bundle_web")]
    if let Some(bundle_matches) = cli_matches.subcommand_matches("bundle-web") {
        crate::cli::commands::bundle_web::run(bundle_matches)?;
        return Ok(());
    }

    #[cfg(feature = "serve")]
    if let Some(serve_matches) = cli_matches.subcommand_matches("serve") {
        let exit_code = crate::cli::commands::serve::run(
            serve_matches,
            crate::cli::commands::serve::ServeResources {
                stylesheet_backup_url: stylesheet_backup_url.as_str(),
                shim_backup_url: shim_backup_url.as_str(),
                wasm_backup_url: wasm_backup_url.as_str(),
                js_backup_url: js_backup_url.as_str(),
                shim_html: SHIMHTML,
                mech_wasm: MECHWASM,
                mech_js: MECHJS,
            },
        )
        .await?;
        if exit_code != 0 {
            terminate_process(exit_code);
        }
    }

    #[cfg(feature = "test")]
    if let Some(matches) = cli_matches.subcommand_matches("test") {
        let exit_code =
            crate::cli::commands::test::run(matches, tree_flag, debug_flag, time_flag, trace_flag)?;
        terminate_process(exit_code);
    }

    #[cfg(feature = "build")]
    if let Some(matches) = cli_matches.subcommand_matches("build") {
        crate::cli::commands::build::run(
            &cli_matches,
            matches,
            tree_flag,
            time_flag,
            trace_flag,
            root_rounds_per_step,
        )?;
        return Ok(());
    }

    #[cfg(feature = "formatter")]
    if let Some(matches) = cli_matches.subcommand_matches("format") {
        crate::cli::commands::format::run(
            &cli_matches,
            matches,
            stylesheet_backup_url.as_str(),
            shim_backup_url.as_str(),
            SHIMHTML,
        )
        .await?;
        return Ok(());
    }

    #[cfg(feature = "run")]
    let mut repl_runtime_config = None;
    #[cfg(all(feature = "run", feature = "repl"))]
    let mut repl_seed_program = None;

    #[cfg(feature = "run")]
    {
        let run_outcome = crate::cli::commands::run::run(
            &cli_matches,
            cli_matches.subcommand_matches("run"),
            crate::cli::commands::run::RunRootFlags {
                debug: debug_flag,
                trace: trace_flag,
                time: time_flag,
                repl: repl_flag,
                root_rounds_per_step,
            },
        )?;
        if let Some(exit_code) = run_outcome.exit_code {
            terminate_process(exit_code);
        }
        repl_flag = run_outcome.repl_flag;
        repl_runtime_config = run_outcome.repl_runtime_config;
        #[cfg(all(feature = "run", feature = "repl"))]
        {
            repl_seed_program = run_outcome.repl_seed_program;
        }
    }

    #[cfg(feature = "repl")]
    if repl_flag {
        crate::cli::commands::repl::run(crate::cli::commands::repl::ReplStartup {
            #[cfg(feature = "run")]
            runtime_config: repl_runtime_config,
            #[cfg(all(feature = "run", feature = "repl"))]
            seed_program: repl_seed_program,
        })?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "app/tests/mod.rs"]
mod tests;

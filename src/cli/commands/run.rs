use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use colored::*;
use mech_core::*;
use mech_program::*;
use mech_runtime::{FS_LIST, FS_READ, MECH_TOOL_SUBJECT};

use crate::cli::capabilities;
use crate::cli::outcome::CliOutcome;
use crate::cli::run::{RunInputMode, new_cli_runtime, run_cli_source, run_cli_source_code};
use crate::cli::run_options::RunOptions;
use crate::cli::runtime_plan::{RunExecutionPlan, build_run_execution_plan};
use crate::source_discovery::{
    DedupePolicy, DiscoveryOptions, MissingPathPolicy, RelativePathPolicy, collect_sources,
};

pub(crate) fn command() -> Command {
    Command::new("run")
    .about("Run Mech source files, project inputs, or inline Mech code.")
    .arg(Arg::new("mech_run_paths")
      .help("Source .mec files, project folders, or inline Mech code.")
      .required(false)
      .action(ArgAction::Append))
    .arg(Arg::new("debug")
      .short('d')
      .long("debug")
      .help("Print debug info")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("time")
      .short('t')
      .long("time")
      .help("Measure how long the program takes to execute.")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("rounds-per-step")
      .long("rounds-per-step")
      .value_name("ROUNDS")
      .help("Sets the number of rounds per step. Overrides runtime.limits.max-steps-per-turn.")
      .required(false))
    .arg(Arg::new("trace")
      .long("trace")
      .help("Print trace output for state-machine arms and function calls")
      .action(ArgAction::SetTrue))
}

pub(crate) fn add_cli_host_capability_args(command: Command) -> Command {
    command.args(crate::cli::run::cli_host_capability_args())
}

const RUN_EXTENSIONS: &[&str] = &["mec", "🤖", "mecb", "mdoc", "mpkg", "m", "csv", "js"];
const RUN_DIRECTORY_EXTENSIONS: &[&str] = &["mec", "🤖", "mdoc", "mpkg"];
const SKIP_SOURCE_DIRS: &[&str] = &["target", ".git", "dist", "out"];

pub(crate) fn collect_run_targets(path: &Path) -> MResult<Vec<PathBuf>> {
    let mut ids = mech_runtime::DefaultIdGenerator::new();
    let mut authority = mech_runtime::HostFilesystemAuthority::new(
        MECH_TOOL_SUBJECT,
        mech_runtime::SharedCapabilityKernel::new(),
    );
    let root = if path.is_dir() {
        path
    } else {
        path.parent().unwrap_or_else(|| Path::new("."))
    };
    authority.grant_path(&mut ids, root, true, [FS_READ, FS_LIST])?;
    collect_run_targets_with_capabilities(path, authority.kernel())
}

fn collect_run_targets_with_capabilities(
    path: &Path,
    kernel: &mech_runtime::SharedCapabilityKernel,
) -> MResult<Vec<PathBuf>> {
    if path.is_file() {
        let mut kernel = kernel.clone();
        mech_runtime::check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_READ, path)?;
    } else if path.is_dir() {
        let mut kernel = kernel.clone();
        mech_runtime::check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_LIST, path)?;
    }
    let entries = collect_sources(
        &[path.to_path_buf()],
        path,
        DiscoveryOptions {
            allowed_file_extensions: RUN_EXTENSIONS,
            recursive_file_extensions: RUN_DIRECTORY_EXTENSIONS,
            skip_dir_names: SKIP_SOURCE_DIRS,
            follow_file_symlinks: true,
            follow_dir_symlinks: false,
            missing_path_policy: MissingPathPolicy::SkipBrokenSymlink,
            dedupe_policy: DedupePolicy::LogicalPath,
            relative_path_policy: RelativePathPolicy::ErrorOutsideBase,
        },
    )?;
    let mut out = entries
        .into_iter()
        .map(|entry| entry.logical_path)
        .collect::<Vec<_>>();
    out.sort();
    Ok(out)
}

pub(crate) fn run(options: RunOptions) -> MResult<CliOutcome> {
    let plan = build_run_execution_plan(options)?;
    execute_plan(plan)
}

fn print_value(value: &Value) {
    println!("{}", value.kind());
    #[cfg(feature = "pretty_print")]
    println!("{}", value.pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", value);
}

fn execute_plan(plan: RunExecutionPlan) -> MResult<CliOutcome> {
    let repl_runtime_config = Some(plan.runtime_config.clone());
    let mut runtime = new_cli_runtime(
        plan.runtime_config,
        &plan.cli_grants,
        &plan.configured_hosts,
        &plan.configured_run_grants,
    )?;
    capabilities::install_file_resolver(
        &mut runtime,
        &plan.filesystem_access,
        &std::env::current_dir()?,
    )?;

    if let RunInputMode::InlineSource(source) = &plan.input_mode {
        match run_cli_source(&mut runtime, source.trim()) {
            Ok(value) => {
                print_value(&value);
                return Ok(CliOutcome::exit(0));
            }
            Err(err) => {
                println!("{} {:#?}", "[Error]".truecolor(246, 98, 78), err);
                return Ok(CliOutcome::exit(1));
            }
        }
    }

    let result: MResult<Value> = if plan.run_paths.is_empty() {
        Ok(Value::Empty)
    } else {
        let fs_kernel = plan.filesystem_access.kernel.clone();
        let mut last = Value::Empty;
        for p in &plan.run_paths {
            for target in collect_run_targets_with_capabilities(Path::new(p), &fs_kernel)? {
                let src = mech_runtime::read_runtime_source_file_with_capabilities(
                    &target,
                    Some(&fs_kernel),
                    Some(MECH_TOOL_SUBJECT),
                )?;
                last = run_cli_source_code(&mut runtime, &src)?;
            }
        }
        Ok(last)
    };

    let repl_flag = plan.repl_requested || plan.missing_run_options;
    match &result {
        Ok(value) if repl_flag => {
            #[cfg(all(feature = "run", feature = "repl"))]
            {
                return Ok(CliOutcome::EnterRepl(
                    crate::cli::commands::repl::ReplStartup {
                        runtime_config: repl_runtime_config,
                        seed_program: Some(runtime.take_program()),
                    },
                ));
            }
            #[cfg(not(feature = "repl"))]
            {
                print_value(value);
                return Ok(CliOutcome::exit(0));
            }
        }
        Ok(value) => {
            print_value(value);
            Ok(CliOutcome::exit(0))
        }
        Err(err) => {
            crate::cli::diagnostics::print_mech_error(err);
            Ok(CliOutcome::exit(1))
        }
    }
}

#[cfg(test)]
mod command_outcome_tests {
    use super::*;

    #[test]
    fn run_command_outcome_reports_exit_code_without_exiting_process() {
        let outcome = CliOutcome::exit(0);
        assert!(matches!(outcome, CliOutcome::Exit(0)));
    }
}

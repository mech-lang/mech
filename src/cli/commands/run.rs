use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;

use clap::{Arg, ArgAction, Command};
use mech_core::*;
use mech_runtime::{FS_LIST, FS_READ, MECH_TOOL_SUBJECT};

use crate::cli::capabilities;
use crate::cli::config;
use crate::cli::outcome::CliOutcome;
use crate::cli::run::{
    RunInputMode, cli_module_options, new_cli_runtime, run_cli_root_module_with_events,
    run_cli_source_code_with_events, run_cli_source_with_events,
};
use crate::cli::runtime_plan::RunExecutionPlan;
use crate::source_discovery::{
    DedupePolicy, DiscoveryOptions, MissingPathPolicy, SkipReason, SourceDiscoveryEvent,
    collect_sources_with_events,
};
use mech_runtime::{RuntimeEvent, RuntimeEventKind, SourceKind, SourceRequest};

#[derive(Debug, Clone)]
struct CliRunError {
    operation: String,
    reason: String,
}

impl MechErrorKind for CliRunError {
    fn name(&self) -> &str { "CliRunError" }
    fn message(&self) -> String { format!("{} failed: {}", self.operation, self.reason) }
}

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
    let discovery = collect_sources_with_events(
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
        },
    )?;
    render_discovery_events(&discovery.events);
    let mut out = discovery
        .entries
        .into_iter()
        .map(|entry| entry.logical_path)
        .collect::<Vec<_>>();
    out.sort();
    Ok(out)
}

fn render_discovery_events(events: &[SourceDiscoveryEvent]) {
    for event in events {
        match event {
            SourceDiscoveryEvent::SkippedBrokenSymlink { path } => {
                println!("[Mech Run] Skipped broken symlink: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedSymlinkedDirectory { path } => {
                println!("[Mech Run] Skipped symlinked directory: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedFileSymlink { path } => {
                println!("[Mech Run] Skipped file symlink: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedUnsupportedExtension { path } => {
                println!("[Mech Run] Skipped unsupported source: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedDirectory { path, reason } => match reason {
                SkipReason::SkippedByName => {
                    println!("[Mech Run] Skipped directory: {}", path.display())
                }
                SkipReason::AlreadyVisited => println!(
                    "[Mech Run] Skipped already visited directory: {}",
                    path.display()
                ),
            },
        }
    }
}

pub(crate) fn run(plan: RunExecutionPlan) -> MResult<CliOutcome> {
    execute_plan(plan)
}

fn render_capability_events(events: &[capabilities::FilesystemCapabilityEvent]) {
    for event in events {
        match event {
            capabilities::FilesystemCapabilityEvent::DefaultGrant {
                path, operations, ..
            } => println!(
                "[Mech Run] Default filesystem grant: {} ({})",
                path.display(),
                operations.join(",")
            ),
            capabilities::FilesystemCapabilityEvent::CliGrant {
                source_flag,
                path,
                operations,
                ..
            } => println!(
                "[Mech Run] {source_flag} filesystem grant: {} ({})",
                path.display(),
                operations.join(",")
            ),
            capabilities::FilesystemCapabilityEvent::ConfigGrant {
                path, operations, ..
            } => println!(
                "[Mech Run] Config filesystem grant: {} ({})",
                path.display(),
                operations.join(",")
            ),
            capabilities::FilesystemCapabilityEvent::NoGrants => {
                println!("[Mech Run] No filesystem grants configured.")
            }
        }
    }
}

fn render_config_event(event: &config::ConfigLoadEvent) {
    match event {
        config::ConfigLoadEvent::DisabledByFlag => println!("[Mech Run] Config loading disabled."),
        config::ConfigLoadEvent::LoadedExplicit { path } => {
            println!("[Mech Run] Loading config… {}", path.display())
        }
        config::ConfigLoadEvent::LoadedDiscovered { path } => {
            println!("[Mech Run] Loading config… {}", path.display())
        }
        config::ConfigLoadEvent::NotFound => {}
    }
}

fn print_value(value: &Value) {
    println!("{}", value.kind());
    #[cfg(feature = "pretty_print")]
    println!("{}", value.pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", value);
}

fn print_run_runtime_events(events: &[RuntimeEvent]) {
    for event in events {
        if let RuntimeEventKind::ProgramProfiled { duration_ns, .. } = &event.kind {
            println!("Cycle Time: {:.3}ms", *duration_ns as f64 / 1_000_000.0);
        }
    }
}

fn execute_plan(plan: RunExecutionPlan) -> MResult<CliOutcome> {
    render_config_event(&plan.config_event);
    render_capability_events(&plan.filesystem_access.events);
    #[cfg(feature = "repl")]
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

    let result: MResult<Value> = match &plan.input_mode {
        RunInputMode::InlineSource(source) => {
            run_cli_source_with_events(&mut runtime, source.trim())
                .map(|(value, events)| {
                    print_run_runtime_events(&events);
                    value
                })
        }
        _ => {
            if plan.run_paths.is_empty() {
                Ok(Value::Empty)
            } else {
                let fs_kernel = plan.filesystem_access.kernel.clone();
                let mut last = Value::Empty;
                for p in &plan.run_paths {
                    for target in collect_run_targets_with_capabilities(Path::new(p), &fs_kernel)? {
                        let (value, events) = if SourceKind::from_path(&target) == SourceKind::Mech {
                            let canonical_target = target.canonicalize().map_err(|error| {
                                MechError::new(
                                    CliRunError {
                                        operation: "canonicalize_run_target".to_string(),
                                        reason: format!("{}: {}", target.display(), error),
                                    },
                                    None,
                                )
                            })?;
                            run_cli_root_module_with_events(
                                &mut runtime,
                                SourceRequest::new(canonical_target.to_string_lossy().to_string()),
                                cli_module_options(),
                            )?
                        } else {
                            let src = mech_runtime::read_runtime_source_file_with_capabilities(
                                &target,
                                Some(&fs_kernel),
                                Some(MECH_TOOL_SUBJECT),
                            )?;
                            run_cli_source_code_with_events(&mut runtime, &src)?
                        };
                        print_run_runtime_events(&events);
                        last = value;
                    }
                }
                Ok(last)
            }
        }
    };

    let repl_flag = plan.repl_requested || plan.missing_run_options;
    match result {
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
                print_value(&value);
                return Ok(CliOutcome::exit(0));
            }
        }
        Ok(value) => {
            if should_run_live(&runtime) {
                run_live_runtime(&mut runtime)?;
            } else {
                print_value(&value);
            }
            Ok(CliOutcome::exit(0))
        }
        Err(err) => Err(err),
    }
}

fn should_run_live(runtime: &mech_runtime::MechRuntime) -> bool {
    runtime.has_driven_live_input_bindings()
}

fn run_live_runtime(runtime: &mut mech_runtime::MechRuntime) -> MResult<()> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_handler = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        stop_for_handler.store(true, Ordering::SeqCst);
    }).map_err(|error| MechError::new(CliRunError { operation: "ctrlc_handler".to_string(), reason: error.to_string() }, None))?;

    runtime.start_input_drivers()?;
    let run_result = run_live_loop(runtime, &stop);
    let stop_result = runtime.stop_input_drivers();
    let shutdown_result = runtime.shutdown();

    match (run_result, stop_result, shutdown_result) {
        (Err(error), _, _) => Err(error),
        (Ok(()), Err(error), _) => Err(error),
        (Ok(()), Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(()), Ok(())) => Ok(()),
    }
}

fn run_live_loop(runtime: &mut mech_runtime::MechRuntime, stop: &AtomicBool) -> MResult<()> {
    const MAX_DRAIN_PER_TURN: usize = 64;
    const IDLE_SLEEP: Duration = Duration::from_millis(10);

    while !stop.load(Ordering::SeqCst) {
        if runtime.pending_host_input_count()? == 0 {
            std::thread::sleep(IDLE_SLEEP);
            continue;
        }
        runtime.drain_host_inputs(MAX_DRAIN_PER_TURN)?;
    }
    Ok(())
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

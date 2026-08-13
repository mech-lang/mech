use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use clap::{Arg, ArgAction, Command};
use mech_core::*;
use mech_runtime::{FS_LIST, FS_READ, MECH_TOOL_SUBJECT};

use crate::cli::capabilities;
use crate::cli::config;
use crate::cli::outcome::CliOutcome;
use crate::cli::run::{
    RunInputMode, cli_module_options, new_cli_runtime_with_source_resolver_and_host_factories,
    run_cli_root_module_with_events, run_cli_source_code_with_events, run_cli_source_with_events,
};
use crate::cli::runtime_plan::RunExecutionPlan;
use crate::source_discovery::{
    DedupePolicy, DiscoveryOptions, MissingPathPolicy, SkipReason, SourceDiscoveryEvent,
    collect_sources_with_events,
};
use mech_runtime::{
    RuntimeEvent, RuntimeEventKind, RuntimeValueSnapshot, SourceKind, SourceRequest,
};

#[derive(Debug, Clone)]
struct CliRunError {
    operation: String,
    reason: String,
}

impl MechErrorKind for CliRunError {
    fn name(&self) -> &str {
        "CliRunError"
    }
    fn message(&self) -> String {
        format!("{} failed: {}", self.operation, self.reason)
    }
}

pub(crate) fn command() -> Command {
    let command = Command::new("run")
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
      .value_parser(crate::cli::rounds_per_step_value_parser())
      .help("Sets the number of rounds per step. Must be a positive integer. Overrides runtime.limits.max-steps-per-turn.")
      .required(false))
    .arg(Arg::new("trace")
      .long("trace")
      .help("Print trace output for state-machine arms and function calls")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("resident")
      .long("resident")
      .help("Require production resident execution")
      .conflicts_with("legacy")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("legacy")
      .long("legacy")
      .help("Use the legacy executor without resident planning")
      .conflicts_with("resident")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("runtime-info")
      .long("runtime-info")
      .help("Print final production routing diagnostics as JSON")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("max-live-turns")
      .long("max-live-turns")
      .value_name("TURNS")
      .value_parser(crate::cli::rounds_per_step_value_parser())
      .help("Stop after this many accepted live turns")
      .required(false));
    #[cfg(feature = "repl")]
    let command = command.arg(
        Arg::new("repl")
            .short('r')
            .long("repl")
            .help("Enter a runtime-backed REPL after running the selected inputs")
            .action(ArgAction::SetTrue),
    );
    command
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

pub(crate) fn collect_run_targets_with_capabilities(
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

fn print_value(value: &RuntimeValueSnapshot) {
    println!("{}", value.kind());
    #[cfg(feature = "pretty_print")]
    println!("{}", value.to_value().pretty_print());
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
    if plan
        .loaded_config
        .as_ref()
        .and_then(|config| config.document.run.as_ref())
        .and_then(|run| run.executor.as_ref())
        .is_some()
    {
        #[cfg(feature = "gpu_executor_native")]
        return crate::cli::executor::run(&plan);
        #[cfg(not(feature = "gpu_executor_native"))]
        return Err(MechError::new(
            CliRunError {
                operation: "select_executor".to_string(),
                reason: "this project configures run.executor; rebuild Mech with `--features gpu_executor_native`".to_string(),
            },
            None,
        )
        .with_compiler_loc());
    }
    #[cfg(feature = "gpu_executor_native")]
    let host_factories = crate::cli::executor::configured_gpu_host_factory(&plan)?
        .into_iter()
        .collect();
    #[cfg(not(feature = "gpu_executor_native"))]
    let host_factories = {
        if plan
            .configured_hosts
            .iter()
            .any(|host| host.provider == "gpu")
        {
            return Err(MechError::new(
                CliRunError {
                    operation: "initialize_gpu_host".to_owned(),
                    reason: "this project configures a GPU host; rebuild Mech with `--features gpu_executor_native`".to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Vec::new()
    };
    let mut runtime = new_cli_runtime_with_source_resolver_and_host_factories(
        plan.runtime_config,
        &plan.cli_grants,
        &plan.configured_hosts,
        &plan.configured_run_grants,
        host_factories,
        mech_runtime::FileSourceResolver::new(&std::env::current_dir()?)
            .with_capabilities(plan.filesystem_access.kernel.clone(), MECH_TOOL_SUBJECT),
    )?;

    let load_options = mech_runtime::RuntimeProgramLoadOptions {
        routing: plan.resident_routing,
        durability: plan.resident_durability,
    };

    if (plan.repl_requested || plan.missing_run_options)
        && plan.resident_routing == mech_runtime::ResidentRoutingPolicy::RequireResident
    {
        return Err(MechError::new(
            mech_runtime::ResidentRouteFailure {
                class: mech_runtime::ResidentRouteFailureClass::ReplUnsupported,
                reason: "resident REPL mutation is not supported in D4".to_string(),
            },
            None,
        ));
    }

    let result: MResult<RuntimeValueSnapshot> = match &plan.input_mode {
        RunInputMode::InlineSource(source) if !plan.repl_requested => runtime
            .load_source_program(source.trim(), load_options)
            .map(|outcome| outcome.initial_value),
        RunInputMode::InlineSource(source) => {
            run_cli_source_with_events(&mut runtime, source.trim()).map(|(value, events)| {
                print_run_runtime_events(&events);
                value
            })
        }
        _ => {
            if plan.run_paths.is_empty() {
                Ok(RuntimeValueSnapshot::empty())
            } else {
                let fs_kernel = plan.filesystem_access.kernel.clone();
                let mut targets = Vec::new();
                for p in &plan.run_paths {
                    targets.extend(collect_run_targets_with_capabilities(
                        Path::new(p),
                        &fs_kernel,
                    )?);
                }
                if targets.len() == 1
                    && SourceKind::from_path(&targets[0]).is_executable_mech()
                    && !plan.repl_requested
                {
                    let canonical_target = targets[0].canonicalize().map_err(|error| {
                        MechError::new(
                            CliRunError {
                                operation: "canonicalize_run_target".to_string(),
                                reason: format!("{}: {}", targets[0].display(), error),
                            },
                            None,
                        )
                    })?;
                    runtime
                        .load_root_program(
                            SourceRequest::from_filesystem_path(&canonical_target)?,
                            cli_module_options(),
                            load_options,
                        )
                        .map(|outcome| outcome.initial_value)
                } else {
                    if targets.len() > 1
                        && plan.resident_routing
                            == mech_runtime::ResidentRoutingPolicy::RequireResident
                    {
                        return Err(MechError::new(
                            mech_runtime::ResidentRouteFailure {
                                class: mech_runtime::ResidentRouteFailureClass::MultipleRootsUnsupported,
                                reason: "multiple independent roots cannot be resident-routed in D4"
                                    .to_string(),
                            },
                            None,
                        ));
                    }
                    let legacy_turns = targets.len() as u64;
                    let mut last = RuntimeValueSnapshot::empty();
                    for target in targets {
                        let (value, events) = if SourceKind::from_path(&target) == SourceKind::Mech
                        {
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
                                SourceRequest::from_filesystem_path(&canonical_target)?,
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
                    runtime
                        .record_legacy_program_route(plan.resident_routing, legacy_turns.max(1))?;
                    Ok(last)
                }
            }
        }
    };

    let repl_flag = plan.repl_requested || plan.missing_run_options;
    match result {
        Ok(value) if repl_flag => {
            if runtime.program_route() == mech_runtime::RuntimeProgramRoute::None {
                runtime.record_legacy_program_route(plan.resident_routing, 1)?;
            }
            #[cfg(all(feature = "run", feature = "repl"))]
            {
                let _ = value;
                return Ok(CliOutcome::EnterRepl(
                    crate::cli::commands::repl::ReplStartup {
                        runtime: Some(runtime),
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
            if should_run_live(&runtime)? {
                run_live_runtime(&mut runtime, plan.max_live_turns)?;
            } else {
                print_value(&value);
            }
            if plan.runtime_info {
                println!("MECH_RUNTIME_INFO {}", runtime_info_json(&runtime));
            }
            Ok(CliOutcome::exit(0))
        }
        Err(err) => Err(err),
    }
}

fn should_run_live(runtime: &mech_runtime::MechRuntime) -> MResult<bool> {
    runtime.has_driven_live_input_bindings()
}

fn run_live_runtime(
    runtime: &mut mech_runtime::MechRuntime,
    max_live_turns: Option<usize>,
) -> MResult<()> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_handler = Arc::clone(&stop);
    ctrlc::set_handler(move || {
        stop_for_handler.store(true, Ordering::SeqCst);
    })
    .map_err(|error| {
        MechError::new(
            CliRunError {
                operation: "ctrlc_handler".to_string(),
                reason: error.to_string(),
            },
            None,
        )
    })?;

    runtime.start_input_drivers()?;
    let run_result = run_live_loop(runtime, &stop, max_live_turns);
    let stop_result = runtime.stop_input_drivers();
    let shutdown_result = runtime.shutdown();

    match (run_result, stop_result, shutdown_result) {
        (Err(error), _, _) => Err(error),
        (Ok(()), Err(error), _) => Err(error),
        (Ok(()), Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(()), Ok(())) => Ok(()),
    }
}

fn run_live_loop(
    runtime: &mut mech_runtime::MechRuntime,
    stop: &AtomicBool,
    max_live_turns: Option<usize>,
) -> MResult<()> {
    const MAX_DRAIN_PER_TURN: usize = 64;
    const IDLE_SLEEP: Duration = Duration::from_millis(10);
    let mut legacy_accepted_live_turns = 0usize;

    while !stop.load(Ordering::SeqCst) {
        let execution = runtime.program_execution_info();
        let accepted_live_turns = match execution.route {
            mech_runtime::RuntimeProgramRoute::Legacy => legacy_accepted_live_turns as u64,
            _ => execution.resident_accepted_turns,
        };
        if max_live_turns.is_some_and(|limit| accepted_live_turns >= limit as u64) {
            break;
        }
        if runtime.pending_host_input_count()? == 0 {
            std::thread::sleep(IDLE_SLEEP);
            continue;
        }
        let outcomes = runtime.drain_host_inputs(MAX_DRAIN_PER_TURN)?;
        if execution.route == mech_runtime::RuntimeProgramRoute::Legacy {
            let accepted = outcomes
                .iter()
                .filter(|outcome| outcome.turn.is_some())
                .count();
            legacy_accepted_live_turns = legacy_accepted_live_turns.saturating_add(accepted);
            runtime.record_legacy_live_turns(accepted as u64)?;
        }
    }
    Ok(())
}

fn runtime_info_json(runtime: &mech_runtime::MechRuntime) -> serde_json::Value {
    let info = runtime.program_execution_info();
    let route = match info.route {
        mech_runtime::RuntimeProgramRoute::None => "none",
        mech_runtime::RuntimeProgramRoute::Legacy => "legacy",
        mech_runtime::RuntimeProgramRoute::ResidentPure => "resident-pure",
        mech_runtime::RuntimeProgramRoute::ResidentExternal => "resident-external",
    };
    let routing_policy = match info.policy {
        mech_runtime::ResidentRoutingPolicy::PreferResident => "prefer-resident",
        mech_runtime::ResidentRoutingPolicy::RequireResident => "require-resident",
        mech_runtime::ResidentRoutingPolicy::LegacyOnly => "legacy-only",
    };
    let revision = info.program_revision.map(|revision| {
        revision
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    });
    serde_json::json!({
        "route": route,
        "routing_policy": routing_policy,
        "program_revision": revision,
        "plan_generation": info.plan_generation.map(|generation| generation.get().saturating_add(1)),
        "layout_generation": info.layout_generation.map(|generation| generation.get().saturating_add(1)),
        "requirements": info.requirement_count,
        "observations": info.observation_count,
        "effects": info.effect_count,
        "resident_accepted_turns": info.resident_accepted_turns,
        "resident_rejected_turns": info.resident_rejected_turns,
        "coalesced_host_packets": info.coalesced_host_packets,
        "ignored_host_packets": info.ignored_host_packets,
        "legacy_turns": info.legacy_turns,
    })
}

#[cfg(test)]
mod command_outcome_tests {
    use super::*;

    #[test]
    fn run_command_outcome_reports_exit_code_without_exiting_process() {
        let outcome = CliOutcome::exit(0);
        assert!(matches!(outcome, CliOutcome::Exit(0)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn run_command_preserves_non_utf8_source_path() {
        use crate::cli::capabilities::{FilesystemCapabilityArgs, build_filesystem_runtime_access};
        use crate::cli::config::ConfigLoadEvent;
        use crate::cli::host_grants::CliHostCapabilitySelection;
        use crate::cli::run::RunInputMode;
        use crate::cli::run_options::PreparedRunOptions;
        use crate::cli::runtime_plan::build_run_execution_plan;
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        struct CurrentDirGuard {
            previous: PathBuf,
            _lock: std::sync::MutexGuard<'static, ()>,
        }

        impl CurrentDirGuard {
            fn enter(path: &Path) -> Self {
                let lock = crate::cli::lock_current_dir();
                let previous = std::env::current_dir().unwrap();
                std::env::set_current_dir(path).unwrap();
                Self {
                    previous,
                    _lock: lock,
                }
            }
        }

        impl Drop for CurrentDirGuard {
            fn drop(&mut self) {
                std::env::set_current_dir(&self.previous).unwrap();
            }
        }

        let root = std::env::temp_dir().join(format!(
            "mech-run-non-utf8-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join(OsString::from_vec(b"run-\xFF.mec".to_vec()));
        std::fs::write(&source, "answer := 42\nanswer\n").unwrap();

        let guard = CurrentDirGuard::enter(&root);
        let filesystem_access =
            build_filesystem_runtime_access(&FilesystemCapabilityArgs::default(), None).unwrap();
        let plan = build_run_execution_plan(PreparedRunOptions {
            input_mode: RunInputMode::Paths(vec![".".to_string()]),
            explicit_run_command: true,
            debug: false,
            trace: false,
            time: false,
            repl: false,
            rounds_per_step: None,
            resident_routing_override: None,
            runtime_info: false,
            max_live_turns: None,
            loaded_config: None,
            config_event: ConfigLoadEvent::NotFound,
            cli_capability_selection: CliHostCapabilitySelection::default(),
            filesystem_access,
        })
        .unwrap();

        let outcome = execute_plan(plan).unwrap();

        assert!(matches!(outcome, CliOutcome::Exit(0)));
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }
}

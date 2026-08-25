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
};
use crate::cli::runtime_plan::RunExecutionPlan;
use crate::source_discovery::{
    DiscoveryOptions, MissingPathPolicy, SkipReason, SourceDiscoveryEvent,
    collect_sources_with_events,
};
use mech_runtime::{RuntimeValueSnapshot, SourceKind, SourceRequest};

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
    .arg(Arg::new("runtime-info")
      .long("runtime-info")
      .help("Print final production routing diagnostics as JSON")
      .action(ArgAction::SetTrue))
    .arg(Arg::new("max-live-turns")
      .long("max-live-turns")
      .value_name("TURNS")
      .value_parser(crate::cli::rounds_per_step_value_parser())
      .help("Stop after this many accepted live turns")
      .required(false))
    .arg(Arg::new("backend")
      .long("backend")
      .value_name("BACKEND")
      .value_parser(crate::cli::STABLE_COMPUTE_BACKEND_SELECTORS)
      .help("Override the configured compute-host backend")
      .required(false));
    command
}

pub(crate) fn add_cli_host_capability_args(command: Command) -> Command {
    command.args(crate::cli::run::cli_host_capability_args())
}

const RUN_EXTENSIONS: &[&str] = &["mec", "🤖", "mecb"];
const RUN_DIRECTORY_EXTENSIONS: &[&str] = &["mec", "🤖"];
const SKIP_SOURCE_DIRS: &[&str] = &["target", ".git", "dist", "out"];

#[cfg(test)]
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

fn enforce_production_resident_target_shape(targets: &[PathBuf]) -> MResult<()> {
    if targets.len() > 1 {
        return Err(MechError::new(
            mech_runtime::ResidentRouteFailure {
                class: mech_runtime::ResidentRouteFailureClass::MultipleRootsUnsupported,
                reason: "production execution accepts exactly one resident program root"
                    .to_string(),
            },
            None,
        ));
    }
    if targets.len() != 1 || !SourceKind::from_path(&targets[0]).is_executable_mech() {
        return Err(MechError::new(
            mech_runtime::ResidentRouteFailure {
                class: mech_runtime::ResidentRouteFailureClass::SemanticUnsupported,
                reason: "production execution requires exactly one .mec or .mecb root".to_string(),
            },
            None,
        ));
    }
    Ok(())
}

fn print_value(value: &RuntimeValueSnapshot) {
    println!("{}", value.kind());
    #[cfg(feature = "pretty_print")]
    println!("{}", value.to_value().pretty_print());
    #[cfg(not(feature = "pretty_print"))]
    println!("{:#?}", value);
}

fn execute_plan(plan: RunExecutionPlan) -> MResult<CliOutcome> {
    render_config_event(&plan.config_event);
    render_capability_events(&plan.filesystem_access.events);
    if plan.missing_run_options {
        return Err(MechError::new(
            mech_runtime::ResidentRouteFailure {
                class: mech_runtime::ResidentRouteFailureClass::SemanticUnsupported,
                reason: "the production run command requires a resident program target".to_string(),
            },
            None,
        ));
    }

    let resolver = mech_runtime::FileSourceResolver::new(&std::env::current_dir()?)
        .with_capabilities(plan.filesystem_access.kernel.clone(), MECH_TOOL_SUBJECT);
    let targets = collect_execution_targets(&plan)?;
    let configured_compute_hosts = plan
        .configured_hosts
        .iter()
        .filter(|host| host.provider == "compute")
        .count();
    if plan.backend_override.is_some() && configured_compute_hosts == 0 {
        return Err(MechError::new(
            CliRunError {
                operation: "select_compute_backend".to_owned(),
                reason: "--backend requires one configured compute host".to_owned(),
            },
            None,
        )
        .with_compiler_loc());
    }

    #[cfg(feature = "compute_backends_native")]
    let compiled_compute = if crate::cli::compute::configured_compute_host(&plan)?.is_some() {
        Some(match &plan.input_mode {
            RunInputMode::InlineSource(source) => {
                crate::cli::compute::compile_inline_compute_application(
                    &plan,
                    source,
                    resolver.clone(),
                )?
            }
            RunInputMode::Paths(_) | RunInputMode::Empty => {
                let target = targets
                    .as_ref()
                    .and_then(|targets| targets.first())
                    .ok_or_else(|| {
                        MechError::new(
                            CliRunError {
                                operation: "compile_compute_application".to_owned(),
                                reason: "a configured compute host requires one source root"
                                    .to_owned(),
                            },
                            None,
                        )
                    })?;
                if SourceKind::from_path(target) == SourceKind::MechBytecode {
                    return Err(MechError::new(
                        CliRunError {
                            operation: "compile_compute_application".to_owned(),
                            reason: "mixed compute bytecode loading is not available in this build"
                                .to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let canonical_target = canonical_run_target(target)?;
                crate::cli::compute::compile_root_compute_application(
                    &plan,
                    SourceRequest::from_filesystem_path(canonical_target)?,
                    resolver.clone(),
                )?
            }
        })
    } else {
        None
    };
    #[cfg(not(feature = "compute_backends_native"))]
    let compiled_compute: Option<mech_engine::ProgramArtifact> = {
        if configured_compute_hosts != 0
            || plan
                .configured_hosts
                .iter()
                .any(|host| host.provider == "gpu")
        {
            return Err(MechError::new(
                CliRunError {
                    operation: "initialize_compute_host".to_owned(),
                    reason: "this project configures a compute host; rebuild Mech with `--features compute_backends_native`".to_owned(),
                },
                None,
            )
            .with_compiler_loc());
        }
        None
    };

    #[cfg(feature = "compute_backends_native")]
    let (host_factories, coordinator) = match compiled_compute {
        Some(compiled) => (vec![compiled.factory], Some(compiled.coordinator)),
        None => (Vec::new(), None),
    };
    #[cfg(not(feature = "compute_backends_native"))]
    let (host_factories, coordinator) = (Vec::new(), compiled_compute);
    let mut runtime = new_cli_runtime_with_source_resolver_and_host_factories(
        plan.runtime_config.clone(),
        &plan.cli_grants,
        &plan.configured_hosts,
        &plan.configured_run_grants,
        host_factories,
        resolver,
    )?;

    let result: MResult<RuntimeValueSnapshot> = if let Some(coordinator) = coordinator {
        runtime
            .load_compiled_program(coordinator, plan.resident_durability)
            .map(|outcome| outcome.initial_value)
    } else {
        match &plan.input_mode {
            RunInputMode::InlineSource(source) => runtime
                .load_source_program(source.trim(), plan.resident_durability)
                .map(|outcome| outcome.initial_value),
            _ => {
                if targets.is_none() {
                    Ok(RuntimeValueSnapshot::empty())
                } else {
                    let targets = targets.as_ref().expect("target presence was checked");
                    let canonical_target = canonical_run_target(&targets[0])?;
                    runtime
                        .load_root_program(
                            SourceRequest::from_filesystem_path(canonical_target)?,
                            cli_module_options(),
                            plan.resident_durability,
                        )
                        .map(|outcome| outcome.initial_value)
                }
            }
        }
    };

    match result {
        Ok(value) => {
            if should_run_live(&runtime)? {
                if let Some(value) = run_live_runtime(&mut runtime, plan.max_live_turns)? {
                    print_value(&value);
                }
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

fn collect_execution_targets(plan: &RunExecutionPlan) -> MResult<Option<Vec<PathBuf>>> {
    if matches!(plan.input_mode, RunInputMode::InlineSource(_)) || plan.run_paths.is_empty() {
        return Ok(None);
    }
    let fs_kernel = plan.filesystem_access.kernel.clone();
    let mut targets = Vec::new();
    for path in &plan.run_paths {
        targets.extend(collect_run_targets_with_capabilities(
            Path::new(path),
            &fs_kernel,
        )?);
    }
    enforce_production_resident_target_shape(&targets)?;
    Ok(Some(targets))
}

fn canonical_run_target(target: &Path) -> MResult<PathBuf> {
    target.canonicalize().map_err(|error| {
        MechError::new(
            CliRunError {
                operation: "canonicalize_run_target".to_string(),
                reason: format!("{}: {error}", target.display()),
            },
            None,
        )
    })
}

fn should_run_live(runtime: &mech_runtime::MechRuntime) -> MResult<bool> {
    runtime.has_driven_live_input_bindings()
}

fn run_live_runtime(
    runtime: &mut mech_runtime::MechRuntime,
    max_live_turns: Option<usize>,
) -> MResult<Option<RuntimeValueSnapshot>> {
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
    let output_result = match &run_result {
        Ok(()) => runtime.program_output_value(),
        Err(_) => Ok(None),
    };
    let stop_result = runtime.stop_input_drivers();
    let shutdown_result = runtime.shutdown();

    match (run_result, output_result, stop_result, shutdown_result) {
        (Err(error), _, _, _) => Err(error),
        (Ok(()), Err(error), _, _) => Err(error),
        (Ok(()), Ok(_), Err(error), _) => Err(error),
        (Ok(()), Ok(_), Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(value), Ok(()), Ok(())) => Ok(value),
    }
}

fn run_live_loop(
    runtime: &mut mech_runtime::MechRuntime,
    stop: &AtomicBool,
    max_live_turns: Option<usize>,
) -> MResult<()> {
    const IDLE_SLEEP: Duration = Duration::from_millis(10);

    let mut completed_live_turns = 0usize;
    while !stop.load(Ordering::SeqCst) {
        if max_live_turns.is_some_and(|limit| completed_live_turns >= limit) {
            break;
        }
        if runtime.pending_host_input_count()? == 0 {
            std::thread::sleep(IDLE_SLEEP);
            continue;
        }
        let drain_limit = live_drain_limit(max_live_turns, completed_live_turns);
        let outcomes = runtime.drain_host_inputs(drain_limit)?;
        completed_live_turns =
            completed_live_turns.saturating_add(successful_live_turn_count(&outcomes));
    }
    Ok(())
}

fn live_drain_limit(max_live_turns: Option<usize>, completed_live_turns: usize) -> usize {
    const MAX_DRAIN_PER_TURN: usize = 64;
    max_live_turns
        .map(|limit| limit.saturating_sub(completed_live_turns))
        .unwrap_or(MAX_DRAIN_PER_TURN)
        .min(MAX_DRAIN_PER_TURN)
}

fn successful_live_turn_count(outcomes: &[mech_runtime::RuntimeHostInputOutcome]) -> usize {
    outcomes
        .iter()
        .filter(|outcome| outcome.resident_turn.is_some())
        .count()
}

fn runtime_info_json(runtime: &mech_runtime::MechRuntime) -> serde_json::Value {
    let info = runtime.program_execution_info();
    let route = match info.route {
        mech_runtime::RuntimeProgramRoute::None => "none",
        mech_runtime::RuntimeProgramRoute::ResidentPure => "resident-pure",
        mech_runtime::RuntimeProgramRoute::ResidentExternal => "resident-external",
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
        "routing_policy": "require-resident",
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

    #[test]
    fn production_run_command_exposes_no_executor_selection_flags() {
        for flag in ["--resident", "--legacy"] {
            let error = command()
                .try_get_matches_from(["run", flag, "program.mec"])
                .expect_err("shipping run command must not expose executor selection");
            assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
        }
    }

    #[test]
    fn run_command_accepts_compute_backend_override() {
        let matches = command()
            .try_get_matches_from(["run", "--backend", "cpu-scalar", "program.mec"])
            .unwrap();
        assert_eq!(
            matches.get_one::<String>("backend").map(String::as_str),
            Some("cpu-scalar")
        );
    }

    #[test]
    fn run_command_rejects_experimental_compute_backend_override() {
        let error = command()
            .try_get_matches_from(["run", "--backend", "cpu-jit", "program.mec"])
            .expect_err("shipping run command must reject experimental backends");
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn require_resident_rejects_empty_and_legacy_only_target_shapes() {
        let empty = enforce_production_resident_target_shape(&[]).unwrap_err();
        assert_eq!(empty.kind_name(), "ResidentRouteFailure");

        let document = [PathBuf::from("program.mdoc")];
        let legacy_only = enforce_production_resident_target_shape(&document).unwrap_err();
        let failure = legacy_only
            .kind_as::<mech_runtime::ResidentRouteFailure>()
            .unwrap();
        assert_eq!(
            failure.class,
            mech_runtime::ResidentRouteFailureClass::SemanticUnsupported
        );
    }

    #[test]
    fn require_resident_accepts_one_source_or_bytecode_root_only() {
        assert!(enforce_production_resident_target_shape(&[PathBuf::from("program.mec")]).is_ok());
        assert!(enforce_production_resident_target_shape(&[PathBuf::from("program.mecb")]).is_ok());
        let multiple = enforce_production_resident_target_shape(&[
            PathBuf::from("a.mec"),
            PathBuf::from("b.mec"),
        ])
        .unwrap_err();
        assert_eq!(
            multiple
                .kind_as::<mech_runtime::ResidentRouteFailure>()
                .unwrap()
                .class,
            mech_runtime::ResidentRouteFailureClass::MultipleRootsUnsupported
        );
    }

    #[test]
    fn live_turn_limit_counts_resident_turn_outcomes() {
        let outcomes = [
            mech_runtime::RuntimeHostInputOutcome {
                update_count: 1,
                ignored_update_count: 0,
                binding_count: 1,
                resident_turn: Some(mech_runtime::ResidentExternalTurnOutcome::Accepted {
                    turn: mech_runtime::TurnId::new(1).unwrap(),
                    receipt_sequence: mech_runtime::LedgerSequence::new(1).unwrap(),
                    delivery_failures: Vec::new().into_boxed_slice(),
                }),
            },
            mech_runtime::RuntimeHostInputOutcome {
                update_count: 1,
                ignored_update_count: 1,
                binding_count: 0,
                resident_turn: None,
            },
        ];
        assert_eq!(successful_live_turn_count(&outcomes), 1);
    }

    #[test]
    fn live_drain_limit_never_exceeds_the_remaining_turn_budget() {
        assert_eq!(live_drain_limit(Some(2), 0), 2);
        assert_eq!(live_drain_limit(Some(2), 1), 1);
        assert_eq!(live_drain_limit(Some(2), 2), 0);
        assert_eq!(live_drain_limit(Some(100), 0), 64);
        assert_eq!(live_drain_limit(None, 9), 64);
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
            rounds_per_step: None,
            runtime_info: false,
            max_live_turns: None,
            backend_override: None,
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

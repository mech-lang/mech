use mech_core::*;
use mech_runtime::{FS_LIST, FS_READ, HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};

use crate::cli::host_grants;
use crate::cli::run::{RunInputMode, effective_run_runtime_config};
use crate::cli::run_options::PreparedRunOptions;
use crate::generate_uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RunPlanEvent {
    AddedDefaultPathGrant {
        path: std::path::PathBuf,
        operations: Vec<&'static str>,
    },
}

pub(crate) struct RunExecutionPlan {
    pub runtime_config: RuntimeConfig,
    pub input_mode: RunInputMode,
    pub run_paths: Vec<String>,
    pub repl_requested: bool,
    pub missing_run_options: bool,
    pub loaded_config: Option<crate::LoadedMechConfig>,
    pub cli_grants: crate::cli::host_grants::EffectiveCliHostGrants,
    pub configured_hosts: Vec<HostInstanceConfig>,
    pub configured_run_grants: Vec<RunResourceGrantConfig>,
    pub filesystem_access: crate::cli::capabilities::FilesystemRuntimeAccess,
    pub config_event: crate::cli::config::ConfigLoadEvent,
    pub events: Vec<RunPlanEvent>,
}

pub(crate) fn build_run_execution_plan(options: PreparedRunOptions) -> MResult<RunExecutionPlan> {
    let uuid = generate_uuid();
    let input_mode = options.input_mode;
    let loaded_config = options.loaded_config;
    let runtime_config = effective_run_runtime_config(
        loaded_config.as_ref(),
        format!("program-{}", uuid),
        options.debug,
        options.trace,
        options.time,
        options.rounds_per_step,
    )?;

    let cli_grants = host_grants::effective_cli_host_grants(
        loaded_config.as_ref(),
        options.cli_capability_selection,
    )?;
    let configured_hosts = loaded_config
        .as_ref()
        .map(|loaded| loaded.document.hosts.clone())
        .unwrap_or_default();
    let configured_run_grants = loaded_config
        .as_ref()
        .and_then(|loaded| loaded.document.run.as_ref())
        .map(|run| run.grants.clone())
        .unwrap_or_default();

    let mut filesystem_access = options.filesystem_access;
    let config_event = options.config_event;

    let explicit_run_command = options.explicit_run_command;
    let run_paths = match &input_mode {
        RunInputMode::Paths(paths) => paths.clone(),
        RunInputMode::Empty | RunInputMode::InlineSource(_) => Vec::new(),
    };
    let effective_options = if matches!(input_mode, RunInputMode::InlineSource(_)) {
        None
    } else {
        crate::cli::run_options::effective_run_options(
            run_paths,
            loaded_config.as_ref(),
            explicit_run_command,
        )?
    };
    let missing_run_options = effective_options.is_none();
    let run_paths = effective_options
        .map(|options| options.paths)
        .unwrap_or_default();

    let mut events = Vec::new();

    if !options.no_default_capabilities {
        let mut ids = mech_runtime::DefaultIdGenerator::new();
        for p in &run_paths {
            let path = std::path::Path::new(p);
            let grant_path = if path.is_dir() {
                path
            } else {
                path.parent().unwrap_or_else(|| std::path::Path::new("."))
            };
            let operations = vec![
                FS_READ,
                FS_LIST,
                mech_runtime::FS_RESOLVE,
                mech_runtime::FS_IMPORT,
            ];
            filesystem_access.authority.grant_path(
                &mut ids,
                grant_path,
                true,
                operations.iter().copied(),
            )?;
            events.push(RunPlanEvent::AddedDefaultPathGrant {
                path: grant_path.to_path_buf(),
                operations,
            });
        }
    }
    filesystem_access.kernel = filesystem_access.authority.kernel().clone();

    Ok(RunExecutionPlan {
        runtime_config,
        input_mode,
        run_paths,
        repl_requested: options.repl,
        missing_run_options,
        loaded_config,
        cli_grants,
        configured_hosts,
        configured_run_grants,
        filesystem_access,
        config_event,
        events,
    })
}

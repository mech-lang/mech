use clap::ArgMatches;
use colored::*;
use mech_core::*;
use mech_runtime::{
    FS_LIST, FS_READ, HostInstanceConfig, MECH_TOOL_SUBJECT, RunResourceGrantConfig, RuntimeConfig,
};

use crate::cli::commands::run::RunOptions;
use crate::cli::run::{
    RunInputMode, classify_run_inputs, cli_host_capability_selection, effective_run_runtime_config,
};
use crate::cli::{capabilities, config};
use crate::generate_uuid;

pub(crate) struct RunExecutionPlan {
    pub runtime_config: RuntimeConfig,
    pub input_mode: RunInputMode,
    pub run_paths: Vec<String>,
    pub repl_requested: bool,
    pub missing_run_options: bool,
    pub loaded_config: Option<crate::LoadedMechConfig>,
    pub cli_grants: crate::cli::config::EffectiveCliHostGrants,
    pub configured_hosts: Vec<HostInstanceConfig>,
    pub configured_run_grants: Vec<RunResourceGrantConfig>,
    pub filesystem_access: crate::cli::capabilities::FilesystemRuntimeAccess,
}

pub(crate) fn build_run_execution_plan(options: RunOptions) -> MResult<RunExecutionPlan> {
    let uuid = generate_uuid();
    let input_mode = classify_run_inputs(options.inputs.clone());
    let config_inputs: Vec<String> = match &input_mode {
        RunInputMode::Paths(paths) => paths.clone(),
        RunInputMode::Empty | RunInputMode::InlineSource(_) => Vec::new(),
    };
    let loaded_config = config::load_run_cli_config(&options.config_matches, &config_inputs)?;
    let runtime_config = effective_run_runtime_config(
        loaded_config.as_ref(),
        format!("program-{}", uuid),
        options.debug,
        options.trace,
        options.time,
        options.rounds_per_step,
    )?;

    let cli_capability_selection =
        cli_host_capability_selection(&options.root_matches, options.run_matches.as_ref());
    let cli_grants =
        config::effective_cli_host_grants(loaded_config.as_ref(), cli_capability_selection)?;
    let configured_hosts = loaded_config
        .as_ref()
        .map(|loaded| loaded.document.hosts.clone())
        .unwrap_or_default();
    let configured_run_grants = loaded_config
        .as_ref()
        .and_then(|loaded| loaded.document.run.as_ref())
        .map(|run| run.grants.clone())
        .unwrap_or_default();

    let badge = "[Mech Run]".truecolor(34, 204, 187);
    let mut filesystem_access = capabilities::build_filesystem_runtime_access(
        &options.config_matches,
        loaded_config.as_ref(),
        &badge,
    )?;

    let explicit_run_command = options.explicit_run_command;
    let run_paths = match &input_mode {
        RunInputMode::Paths(paths) => paths.clone(),
        RunInputMode::Empty | RunInputMode::InlineSource(_) => Vec::new(),
    };
    let effective_options = if matches!(input_mode, RunInputMode::InlineSource(_)) {
        None
    } else {
        config::effective_run_options(run_paths, loaded_config.as_ref(), explicit_run_command)?
    };
    let missing_run_options = effective_options.is_none();
    let run_paths = effective_options
        .map(|options| options.paths)
        .unwrap_or_default();

    if !options.no_default_capabilities {
        let mut ids = mech_runtime::DefaultIdGenerator::new();
        for p in &run_paths {
            let path = std::path::Path::new(p);
            let grant_path = if path.is_dir() {
                path
            } else {
                path.parent().unwrap_or_else(|| std::path::Path::new("."))
            };
            filesystem_access.authority.grant_path(
                &mut ids,
                grant_path,
                true,
                [
                    FS_READ,
                    FS_LIST,
                    mech_runtime::FS_RESOLVE,
                    mech_runtime::FS_IMPORT,
                ],
            )?;
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
    })
}

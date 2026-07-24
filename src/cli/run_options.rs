use clap::ArgMatches;
use mech_core::*;

use crate::cli::outcome::RootFlags;
use crate::cli::run::{RunInputMode, classify_run_inputs, cli_host_capability_selection};
use crate::cli::{capabilities, config, host_grants};
use crate::{LoadedMechConfig, resolve_config_path};
use std::path::{Path, PathBuf};

pub(crate) struct RunCliArgs {
    pub input_mode: RunInputMode,
    pub explicit_run_command: bool,
    pub debug: bool,
    pub trace: bool,
    pub time: bool,
    pub repl: bool,
    pub rounds_per_step: Option<usize>,
    pub cli_capability_selection: host_grants::CliHostCapabilitySelection,
}

impl RunCliArgs {
    pub(crate) fn from_matches(
        root: RootFlags,
        root_matches: &ArgMatches,
        run_matches: Option<&ArgMatches>,
    ) -> MResult<Self> {
        let inputs: Vec<String> = if let Some(run_matches) = run_matches {
            run_matches
                .get_many::<String>("mech_run_paths")
                .map_or(vec![], |files| files.map(|file| file.to_string()).collect())
        } else if let Some(m) = root_matches.get_many::<String>("mech_paths") {
            m.map(|s| s.to_string()).collect()
        } else {
            vec![]
        };
        Ok(Self {
            input_mode: classify_run_inputs(inputs),
            explicit_run_command: run_matches.is_some(),
            debug: root.debug || run_matches.map(|m| m.get_flag("debug")).unwrap_or(false),
            trace: root.trace || run_matches.map(|m| m.get_flag("trace")).unwrap_or(false),
            time: root.time || run_matches.map(|m| m.get_flag("time")).unwrap_or(false),
            repl: root.repl,
            rounds_per_step: run_matches
                .and_then(|m| m.get_one::<String>("rounds-per-step"))
                .and_then(|s| s.parse::<usize>().ok())
                .or(root.rounds_per_step),
            cli_capability_selection: cli_host_capability_selection(root_matches, run_matches),
        })
    }
}

pub(crate) struct PreparedRunOptions {
    pub input_mode: RunInputMode,
    pub explicit_run_command: bool,
    pub debug: bool,
    pub trace: bool,
    pub time: bool,
    pub repl: bool,
    pub rounds_per_step: Option<usize>,
    pub loaded_config: Option<crate::LoadedMechConfig>,
    pub config_event: config::ConfigLoadEvent,
    pub cli_capability_selection: host_grants::CliHostCapabilitySelection,
    pub filesystem_access: capabilities::FilesystemRuntimeAccess,
}

pub(crate) fn prepare_run_options(
    args: RunCliArgs,
    config_matches: &ArgMatches,
) -> MResult<PreparedRunOptions> {
    let config_inputs: Vec<String> = match &args.input_mode {
        RunInputMode::Paths(paths) => paths.clone(),
        RunInputMode::Empty | RunInputMode::InlineSource(_) => Vec::new(),
    };
    let loaded = config::load_cli_config_report_with_inputs(config_matches, &config_inputs)?;
    let loaded_config = loaded.config;
    let capability_args = capabilities::FilesystemCapabilityArgs::from_matches(config_matches);
    let filesystem_access =
        capabilities::build_filesystem_runtime_access(&capability_args, loaded_config.as_ref())?;
    Ok(PreparedRunOptions {
        input_mode: args.input_mode,
        explicit_run_command: args.explicit_run_command,
        debug: args.debug,
        trace: args.trace,
        time: args.time,
        repl: args.repl,
        rounds_per_step: args.rounds_per_step,
        loaded_config,
        config_event: loaded.event,
        cli_capability_selection: args.cli_capability_selection,
        filesystem_access,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveRunOptions {
    pub paths: Vec<String>,
}

pub fn effective_run_options(
    cli_paths: Vec<String>,
    config: Option<&LoadedMechConfig>,
    explicit_run_command: bool,
) -> MResult<Option<EffectiveRunOptions>> {
    let config_path_to_string = |loaded: &LoadedMechConfig, path: &Path| {
        resolve_config_path(&loaded.base_dir, path)
            .to_string_lossy()
            .to_string()
    };

    let config_paths = config
        .and_then(|loaded| {
            loaded.document.run.as_ref().map(|run| {
                run.paths
                    .iter()
                    .map(|path| config_path_to_string(loaded, path))
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();

    let mut effective_cli_paths = cli_paths;
    let had_cli_selector = !effective_cli_paths.is_empty();

    if let Some(loaded) = config {
        if let Some(project_dir) = loaded.discovered_project_dir.as_ref() {
            if effective_cli_paths.len() == 1 {
                let current_dir = std::env::current_dir()?;
                let input = PathBuf::from(&effective_cli_paths[0]);
                let input_path = if input.is_absolute() {
                    input
                } else {
                    current_dir.join(input)
                };

                if input_path.exists()
                    && input_path.is_dir()
                    && input_path.canonicalize()? == *project_dir
                {
                    effective_cli_paths.clear();
                }
            }
        }
    }

    let paths = if !effective_cli_paths.is_empty() {
        effective_cli_paths
    } else if explicit_run_command || had_cli_selector {
        config_paths
    } else {
        Vec::new()
    };

    if paths.is_empty() {
        if explicit_run_command {
            return Err(MechError::new(
                GenericError {
                    msg: "no run inputs supplied; pass path(s) or configure run.paths".to_string(),
                },
                None,
            )
            .with_compiler_loc());
        }

        return Ok(None);
    }

    Ok(Some(EffectiveRunOptions { paths }))
}

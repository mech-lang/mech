use clap::ArgMatches;
use colored::*;
use mech_core::*;

use crate::cli::outcome::RootFlags;
use crate::cli::run::{
    RunInputMode, classify_run_inputs, cli_host_capability_selection,
};
use crate::cli::{capabilities, config};

pub(crate) struct RunOptions {
    pub input_mode: RunInputMode,
    pub explicit_run_command: bool,
    pub debug: bool,
    pub trace: bool,
    pub time: bool,
    pub repl: bool,
    pub rounds_per_step: Option<usize>,
    pub no_default_capabilities: bool,
    pub loaded_config: Option<crate::LoadedMechConfig>,
    pub cli_capability_selection: config::CliHostCapabilitySelection,
    pub filesystem_access: capabilities::FilesystemRuntimeAccess,
}

impl RunOptions {
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
        let config_matches = run_matches.unwrap_or(root_matches);
        let input_mode = classify_run_inputs(inputs);
        let config_inputs: Vec<String> = match &input_mode {
            RunInputMode::Paths(paths) => paths.clone(),
            RunInputMode::Empty | RunInputMode::InlineSource(_) => Vec::new(),
        };
        let loaded_config = config::load_run_cli_config(config_matches, &config_inputs)?;
        let badge = "[Mech Run]".truecolor(34, 204, 187);
        let filesystem_access = capabilities::build_filesystem_runtime_access(
            config_matches,
            loaded_config.as_ref(),
            &badge,
        )?;

        Ok(Self {
            input_mode,
            explicit_run_command: run_matches.is_some(),
            debug: root.debug || run_matches.map(|m| m.get_flag("debug")).unwrap_or(false),
            trace: root.trace || run_matches.map(|m| m.get_flag("trace")).unwrap_or(false),
            time: root.time || run_matches.map(|m| m.get_flag("time")).unwrap_or(false),
            repl: root.repl,
            rounds_per_step: run_matches
                .and_then(|m| m.get_one::<String>("rounds-per-step"))
                .and_then(|s| s.parse::<usize>().ok())
                .or(root.rounds_per_step),
            no_default_capabilities: config_matches.get_flag("no_default_capabilities"),
            loaded_config,
            cli_capability_selection: cli_host_capability_selection(root_matches, run_matches),
            filesystem_access,
        })
    }
}

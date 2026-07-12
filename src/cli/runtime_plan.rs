use mech_core::*;
use mech_runtime::{HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};

use crate::cli::host_grants;
use crate::cli::run::{RunInputMode, effective_run_runtime_config};
use crate::cli::run_options::PreparedRunOptions;
use crate::generate_uuid;

pub(crate) struct RunExecutionPlan {
    pub runtime_config: RuntimeConfig,
    pub input_mode: RunInputMode,
    pub run_paths: Vec<String>,
    pub repl_requested: bool,
    pub missing_run_options: bool,
    pub cli_grants: crate::cli::host_grants::EffectiveCliHostGrants,
    pub configured_hosts: Vec<HostInstanceConfig>,
    pub configured_run_grants: Vec<RunResourceGrantConfig>,
    pub filesystem_access: crate::cli::capabilities::FilesystemRuntimeAccess,
    pub config_event: crate::cli::config::ConfigLoadEvent,
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

    filesystem_access.kernel = filesystem_access.authority.kernel().clone();

    Ok(RunExecutionPlan {
        runtime_config,
        input_mode,
        run_paths,
        repl_requested: options.repl,
        missing_run_options,
          cli_grants,
        configured_hosts,
        configured_run_grants,
        filesystem_access,
        config_event,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::capabilities::{FilesystemCapabilityArgs, build_filesystem_runtime_access};
    use crate::cli::config::ConfigLoadEvent;
    use crate::cli::host_grants::CliHostCapabilitySelection;
    use mech_runtime::{FS_IMPORT, FS_READ, FS_RESOLVE, MECH_TOOL_SUBJECT, check_fs_capability};
    use std::time::{SystemTime, UNIX_EPOCH};

    static CURRENT_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct CurrentDirGuard {
        previous: std::path::PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl CurrentDirGuard {
        fn enter(path: &std::path::Path) -> Self {
            let lock = CURRENT_DIR_LOCK.lock().unwrap();
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

    fn temp_root(label: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-runtime-plan-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    #[test]
    fn explicit_cap_root_does_not_gain_run_input_parent_grant() {
        let root = temp_root("explicit-cap-root");
        let allowed = root.join("allowed");
        let outside = root.join("outside");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let run_input = outside.join("main.mec");
        std::fs::write(&run_input, "x := 1\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);

        let filesystem_access = build_filesystem_runtime_access(
            &FilesystemCapabilityArgs {
                cap_roots: vec![std::path::PathBuf::from("allowed")],
                ..FilesystemCapabilityArgs::default()
            },
            None,
        )
        .unwrap();

        let plan = build_run_execution_plan(PreparedRunOptions {
            input_mode: RunInputMode::Paths(vec!["outside/main.mec".to_string()]),
            explicit_run_command: true,
            debug: false,
            trace: false,
            time: false,
            repl: false,
            rounds_per_step: None,
            loaded_config: None,
            config_event: ConfigLoadEvent::NotFound,
            cli_capability_selection: CliHostCapabilitySelection::default(),
            filesystem_access,
        })
        .unwrap();

        for operation in [FS_READ, FS_RESOLVE, FS_IMPORT] {
            let mut kernel = plan.filesystem_access.kernel.clone();
            assert!(
                check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, operation, &run_input).is_err()
            );
        }

        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }
}

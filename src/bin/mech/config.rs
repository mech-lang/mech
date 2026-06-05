use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use mech::*;
use mech_core::*;
use mech_runtime::{
    ConfigProfileOptions, DEFAULT_CONFIG_FILENAME, LogLevel, MechConfigDocument, RuntimeConfig,
    RuntimeConfigPatch,
};

pub fn add_config_args(command: Command) -> Command {
    command
        .arg(
            Arg::new("config")
                .long("config")
                .value_name("PATH")
                .num_args(1)
                .global(true)
                .help("Load configuration from a Mech config file."),
        )
        .arg(
            Arg::new("no_config")
                .long("no-config")
                .action(ArgAction::SetTrue)
                .global(true)
                .help("Disable automatic Mech config loading."),
        )
}

#[derive(Clone, Debug)]
pub struct LoadedMechConfig {
    pub path: PathBuf,
    pub base_dir: PathBuf,
    pub document: MechConfigDocument,
}

pub fn load_cli_config(matches: &clap::ArgMatches) -> MResult<Option<LoadedMechConfig>> {
    if matches.get_flag("no_config") {
        return Ok(None);
    }

    let current_dir = std::env::current_dir()?;
    let path = if let Some(path) = matches.get_one::<String>("config") {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            current_dir.join(path)
        };
        path.canonicalize()?
    } else {
        let path = current_dir.join(DEFAULT_CONFIG_FILENAME);
        if !path.exists() {
            return Ok(None);
        }
        path.canonicalize()?
    };

    let base_dir = path.parent().unwrap_or(&current_dir).to_path_buf();
    let source = std::fs::read_to_string(&path)?;
    let document = mech_runtime::parse_config_document(
        path.display().to_string(),
        &source,
        ConfigProfileOptions::default(),
    )?;
    println!("[Mech Config] Loaded config: {}", path.display());
    Ok(Some(LoadedMechConfig {
        path,
        base_dir,
        document,
    }))
}

pub fn normalize_config_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        PathBuf::from(path.to_string_lossy().replace('\\', "/"))
    }
}

pub fn resolve_config_path(base_dir: &Path, path: &Path) -> PathBuf {
    let path = normalize_config_path(path);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn require_config_file(field: &str, path: &Path) -> MResult<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "configuration error: {field} must be an existing file: {}",
                path.display()
            ),
        )
        .into())
    }
}

fn require_config_dir(field: &str, path: &Path) -> MResult<()> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "configuration error: {field} must be an existing directory: {}",
                path.display()
            ),
        )
        .into())
    }
}

fn require_config_wasm_package(field: &str, path: &Path) -> MResult<()> {
    require_config_dir(field, path)?;

    let wasm = path.join("mech_wasm_bg.wasm.br");
    if !wasm.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "configuration error: {field} is missing required file: {}",
                wasm.display()
            ),
        )
        .into());
    }

    let js = path.join("mech_wasm.js");
    if !js.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "configuration error: {field} is missing required file: {}",
                js.display()
            ),
        )
        .into());
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveServeOptions {
    pub address: String,
    pub port: String,
    pub paths: Vec<String>,
    pub stylesheet_paths: Vec<String>,
    pub shim_path: String,
    pub wasm_pkg: String,
}

pub fn effective_serve_options(
    serve_matches: &clap::ArgMatches,
    config: Option<&LoadedMechConfig>,
) -> MResult<EffectiveServeOptions> {
    let serve_config = config.and_then(|loaded| loaded.document.serve.as_ref());
    let config_path_to_string = |loaded: &LoadedMechConfig, path: &Path| {
        resolve_config_path(&loaded.base_dir, path)
            .to_string_lossy()
            .to_string()
    };

    let address = serve_matches
        .get_one::<String>("address")
        .cloned()
        .or_else(|| serve_config.and_then(|serve| serve.address.clone()))
        .unwrap_or_else(|| "127.0.0.1".to_string());

    let port = serve_matches
        .get_one::<String>("port")
        .cloned()
        .or_else(|| serve_config.and_then(|serve| serve.port.map(|port| port.to_string())))
        .unwrap_or_else(|| "8081".to_string());

    let cli_shim = serve_matches.get_one::<String>("shim").cloned();
    let config_shim = config.and_then(|loaded| {
        loaded.document.serve.as_ref().and_then(|serve| {
            serve
                .shim
                .as_ref()
                .map(|path| config_path_to_string(loaded, path))
        })
    });
    let shim_path = cli_shim
        .clone()
        .or_else(|| config_shim.clone())
        .unwrap_or_default();
    if cli_shim.is_none() {
        if let Some(path) = config_shim.as_ref() {
            require_config_file("serve.shim", Path::new(path))?;
        }
    }

    let cli_wasm = serve_matches.get_one::<String>("wasm").cloned();
    let config_wasm = config.and_then(|loaded| {
        loaded.document.serve.as_ref().and_then(|serve| {
            serve
                .wasm
                .as_ref()
                .map(|path| config_path_to_string(loaded, path))
        })
    });
    let wasm_pkg = cli_wasm
        .clone()
        .or_else(|| config_wasm.clone())
        .unwrap_or_default();
    if cli_wasm.is_none() {
        if let Some(path) = config_wasm.as_ref() {
            require_config_wasm_package("serve.wasm", Path::new(path))?;
        }
    }

    let config_stylesheets: Vec<String> = config
        .and_then(|loaded| {
            loaded.document.serve.as_ref().map(|serve| {
                serve
                    .stylesheets
                    .iter()
                    .map(|path| config_path_to_string(loaded, path))
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();
    for path in &config_stylesheets {
        require_config_file("serve.stylesheets", Path::new(path))?;
    }
    let cli_stylesheets = serve_matches
        .get_many::<String>("stylesheet")
        .into_iter()
        .flatten()
        .cloned();
    let mut stylesheet_paths = config_stylesheets;
    stylesheet_paths.extend(cli_stylesheets);

    let cli_paths: Vec<String> = serve_matches
        .get_many::<String>("mech_serve_file_paths")
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let paths = if !cli_paths.is_empty() {
        cli_paths
    } else {
        config
            .and_then(|loaded| {
                loaded.document.serve.as_ref().map(|serve| {
                    serve
                        .paths
                        .iter()
                        .map(|path| config_path_to_string(loaded, path))
                        .collect()
                })
            })
            .filter(|paths: &Vec<String>| !paths.is_empty())
            .unwrap_or_default()
    };

    Ok(EffectiveServeOptions {
        address,
        port,
        paths,
        stylesheet_paths,
        shim_path,
        wasm_pkg,
    })
}

pub fn apply_runtime_config_patch(
    mut base: RuntimeConfig,
    patch: &RuntimeConfigPatch,
) -> MResult<RuntimeConfig> {
    if let Some(name) = &patch.name {
        base.name = name.clone();
    }

    if let Some(value) = patch.limits.max_steps_per_turn {
        base.limits.max_steps_per_turn = Some(value);
    }
    if let Some(value) = patch.limits.max_turn_duration_ms {
        base.limits.max_turn_duration_ms = Some(value);
    }
    if let Some(value) = patch.limits.max_memory_bytes {
        base.limits.max_memory_bytes = Some(value);
    }
    if let Some(value) = patch.limits.max_tasks {
        base.limits.max_tasks = Some(value);
    }
    if let Some(value) = patch.limits.max_actors {
        base.limits.max_actors = Some(value);
    }
    if let Some(value) = patch.limits.max_actor_mailbox_len {
        base.limits.max_actor_mailbox_len = Some(value);
    }
    if let Some(value) = patch.limits.max_source_bytes {
        base.limits.max_source_bytes = Some(value);
    }
    if let Some(value) = patch.limits.max_in_memory_events {
        base.limits.max_in_memory_events = Some(value);
    }

    if let Some(value) = patch.diagnostics.trace_enabled {
        base.diagnostics.trace_enabled = value;
    }
    if let Some(value) = patch.diagnostics.profile_enabled {
        base.diagnostics.profile_enabled = value;
    }
    if let Some(value) = patch.diagnostics.debug_enabled {
        base.diagnostics.debug_enabled = value;
    }
    if let Some(value) = &patch.diagnostics.log_level {
        base.diagnostics.log_level = match value.as_str() {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            other => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown runtime.diagnostics.log-level `{other}`"),
                )
                .into());
            }
        };
    }

    base.validate()?;
    Ok(base)
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct CurrentDirGuard {
        previous: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl CurrentDirGuard {
        fn enter(path: &std::path::Path) -> Self {
            let lock = crate::CURRENT_DIR_LOCK.lock().unwrap();
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

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-cli-config-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn command() -> Command {
        add_config_args(
            Command::new("mech").subcommand(
                Command::new("serve")
                    .arg(
                        Arg::new("mech_serve_file_paths")
                            .required(false)
                            .action(ArgAction::Append),
                    )
                    .arg(Arg::new("port").long("port").num_args(1))
                    .arg(Arg::new("address").long("address").num_args(1))
                    .arg(
                        Arg::new("stylesheet")
                            .long("stylesheet")
                            .num_args(1..)
                            .action(ArgAction::Append),
                    )
                    .arg(Arg::new("shim").long("shim").num_args(1))
                    .arg(Arg::new("wasm").long("wasm").num_args(1)),
            ),
        )
    }

    fn matches(args: &[&str]) -> clap::ArgMatches {
        command().try_get_matches_from(args).unwrap()
    }

    fn parse_document(source: &str) -> MechConfigDocument {
        mech_runtime::parse_config_document(
            "test.mcfg".to_string(),
            source,
            ConfigProfileOptions::default(),
        )
        .unwrap()
    }

    fn loaded_config(source: &str) -> LoadedMechConfig {
        LoadedMechConfig {
            path: PathBuf::from("test.mcfg"),
            base_dir: PathBuf::new(),
            document: parse_document(source),
        }
    }

    fn loaded_config_at(base_dir: PathBuf, source: &str) -> LoadedMechConfig {
        LoadedMechConfig {
            path: base_dir.join("mech.mcfg"),
            base_dir,
            document: parse_document(source),
        }
    }

    fn create_wasm_pkg(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("mech_wasm_bg.wasm.br"), b"wasm").unwrap();
        std::fs::write(path.join("mech_wasm.js"), b"js").unwrap();
    }

    fn error_text(result: MResult<EffectiveServeOptions>) -> String {
        format!("{:?}", result.unwrap_err())
    }

    #[test]
    fn explicit_config_loads() {
        let root = temp_root("explicit");
        {
            let _guard = CurrentDirGuard::enter(&root);
            std::fs::write("custom.mcfg", "config := {serve: {port: 9090}}\n").unwrap();
            let matches = matches(&["mech", "--config", "custom.mcfg", "serve"]);
            let config = load_cli_config(matches.subcommand_matches("serve").unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(config.document.serve.unwrap().port, Some(9090));
            assert_eq!(config.path, root.join("custom.mcfg"));
            assert_eq!(config.base_dir, root);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn default_config_auto_loads() {
        let root = temp_root("auto");
        {
            let _guard = CurrentDirGuard::enter(&root);
            std::fs::write(DEFAULT_CONFIG_FILENAME, "config := {serve: {port: 9090}}\n").unwrap();
            let matches = matches(&["mech", "serve"]);
            assert!(
                load_cli_config(matches.subcommand_matches("serve").unwrap())
                    .unwrap()
                    .is_some()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_config_disables_auto_load() {
        let root = temp_root("disabled");
        {
            let _guard = CurrentDirGuard::enter(&root);
            std::fs::write(DEFAULT_CONFIG_FILENAME, "config := {serve: {port: 9090}}\n").unwrap();
            let matches = matches(&["mech", "--no-config", "serve"]);
            assert!(
                load_cli_config(matches.subcommand_matches("serve").unwrap())
                    .unwrap()
                    .is_none()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cli_scalar_options_override_config() {
        let config = loaded_config(r#"config := {serve: {address: "127.0.0.1", port: 8081}}"#);
        let matches = matches(&["mech", "serve", "--address", "0.0.0.0", "--port", "9090"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(effective.address, "0.0.0.0");
        assert_eq!(effective.port, "9090");
    }

    #[test]
    fn config_serve_paths_used_when_cli_paths_absent() {
        let config = loaded_config(r#"config := {serve: {paths: ["docs/reference"]}}"#);
        let matches = matches(&["mech", "serve"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(effective.paths, vec!["docs/reference"]);
    }

    #[test]
    fn cli_serve_paths_override_config_paths() {
        let config = loaded_config(r#"config := {serve: {paths: ["docs/reference"]}}"#);
        let matches = matches(&["mech", "serve", "examples/working"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(effective.paths, vec!["examples/working"]);
    }

    #[test]
    fn stylesheets_combine() {
        let root = temp_root("stylesheets-combine");
        std::fs::write(root.join("a.css"), "body {}").unwrap();
        let config = loaded_config_at(
            root.clone(),
            r#"config := {serve: {stylesheets: ["a.css"]}}"#,
        );
        let matches = matches(&["mech", "serve", "--stylesheet", "b.css"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(
            effective.stylesheet_paths,
            vec![
                root.join("a.css").to_string_lossy().to_string(),
                "b.css".to_string()
            ]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_relative_serve_paths_resolve_from_config_file_directory() {
        let root = temp_root("relative-serve-paths");
        let subdir = root.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(subdir.join("style.css"), "body {}").unwrap();
        std::fs::write(subdir.join("shim.html"), "<html></html>").unwrap();
        create_wasm_pkg(&subdir.join("pkg"));
        std::fs::write(
            subdir.join(DEFAULT_CONFIG_FILENAME),
            r#"config := {
  serve: {
    paths: ["app.mec"]
    stylesheets: ["style.css"]
    shim: "shim.html"
    wasm: "pkg"
  }
}
"#,
        )
        .unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = matches(&["mech", "--config", "subdir/mech.mcfg", "serve"]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let config = load_cli_config(serve_matches).unwrap().unwrap();
            let effective = effective_serve_options(serve_matches, Some(&config)).unwrap();
            assert_eq!(
                effective.paths,
                vec![subdir.join("app.mec").to_string_lossy().to_string()]
            );
            assert_eq!(
                effective.stylesheet_paths,
                vec![subdir.join("style.css").to_string_lossy().to_string()]
            );
            assert_eq!(
                effective.shim_path,
                subdir.join("shim.html").to_string_lossy().to_string()
            );
            assert_eq!(
                effective.wasm_pkg,
                subdir.join("pkg").to_string_lossy().to_string()
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cli_paths_override_config_paths_and_remain_cwd_relative() {
        let root = temp_root("cli-paths-cwd-relative");
        let subdir = root.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(
            subdir.join(DEFAULT_CONFIG_FILENAME),
            r#"config := {serve: {paths: ["app.mec"]}}"#,
        )
        .unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = matches(&["mech", "--config", "subdir/mech.mcfg", "serve", "other.mec"]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let config = load_cli_config(serve_matches).unwrap().unwrap();
            let effective = effective_serve_options(serve_matches, Some(&config)).unwrap();
            assert_eq!(effective.paths, vec!["other.mec"]);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cli_shim_and_wasm_override_config_and_remain_cwd_relative() {
        let root = temp_root("cli-shim-wasm-cwd-relative");
        let subdir = root.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(
            subdir.join(DEFAULT_CONFIG_FILENAME),
            r#"config := {serve: {shim: "shim.html", wasm: "pkg"}}"#,
        )
        .unwrap();
        {
            let _guard = CurrentDirGuard::enter(&root);
            let matches = matches(&[
                "mech",
                "--config",
                "subdir/mech.mcfg",
                "serve",
                "--shim",
                "cli-shim.html",
                "--wasm",
                "cli-pkg",
            ]);
            let serve_matches = matches.subcommand_matches("serve").unwrap();
            let config = load_cli_config(serve_matches).unwrap().unwrap();
            let effective = effective_serve_options(serve_matches, Some(&config)).unwrap();
            assert_eq!(effective.shim_path, "cli-shim.html");
            assert_eq!(effective.wasm_pkg, "cli-pkg");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_config_shim_is_ignored_when_cli_shim_overrides() {
        let config = loaded_config(r#"config := {serve: {shim: "missing-shim.html"}}"#);
        let matches = matches(&["mech", "serve", "--shim", "local.html"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(effective.shim_path, "local.html");
    }

    #[test]
    fn missing_config_wasm_is_ignored_when_cli_wasm_overrides() {
        let config = loaded_config(r#"config := {serve: {wasm: "missing-pkg"}}"#);
        let matches = matches(&["mech", "serve", "--wasm", "local-pkg"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(effective.wasm_pkg, "local-pkg");
    }

    #[test]
    fn missing_config_stylesheet_fails_even_with_cli_stylesheet() {
        let config = loaded_config(r#"config := {serve: {stylesheets: ["missing.css"]}}"#);
        let matches = matches(&["mech", "serve", "--stylesheet", "local.css"]);
        let error = error_text(effective_serve_options(
            matches.subcommand_matches("serve").unwrap(),
            Some(&config),
        ));
        assert!(error.contains("configuration error: serve.stylesheets must be an existing file"));
    }

    #[test]
    fn config_stylesheet_directory_is_rejected() {
        let root = temp_root("stylesheet-dir");
        std::fs::create_dir_all(root.join("style.css")).unwrap();
        let config = loaded_config_at(
            root.clone(),
            r#"config := {serve: {stylesheets: ["style.css"]}}"#,
        );
        let matches = matches(&["mech", "serve"]);
        let error = error_text(effective_serve_options(
            matches.subcommand_matches("serve").unwrap(),
            Some(&config),
        ));
        assert!(error.contains("serve.stylesheets must be an existing file"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_shim_directory_is_rejected() {
        let root = temp_root("shim-dir");
        std::fs::create_dir_all(root.join("shim.html")).unwrap();
        let config = loaded_config_at(root.clone(), r#"config := {serve: {shim: "shim.html"}}"#);
        let matches = matches(&["mech", "serve"]);
        let error = error_text(effective_serve_options(
            matches.subcommand_matches("serve").unwrap(),
            Some(&config),
        ));
        assert!(error.contains("serve.shim must be an existing file"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_wasm_file_is_rejected() {
        let root = temp_root("wasm-file");
        std::fs::write(root.join("pkg"), b"not a dir").unwrap();
        let config = loaded_config_at(root.clone(), r#"config := {serve: {wasm: "pkg"}}"#);
        let matches = matches(&["mech", "serve"]);
        let error = error_text(effective_serve_options(
            matches.subcommand_matches("serve").unwrap(),
            Some(&config),
        ));
        assert!(error.contains("serve.wasm must be an existing directory"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_wasm_directory_missing_required_files_is_rejected() {
        let root = temp_root("wasm-missing-files");
        std::fs::create_dir_all(root.join("pkg")).unwrap();
        let config = loaded_config_at(root.clone(), r#"config := {serve: {wasm: "pkg"}}"#);
        let matches = matches(&["mech", "serve"]);
        let error = error_text(effective_serve_options(
            matches.subcommand_matches("serve").unwrap(),
            Some(&config),
        ));
        assert!(error.contains("serve.wasm is missing required file"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_wasm_directory_with_required_files_is_accepted() {
        let root = temp_root("wasm-complete");
        create_wasm_pkg(&root.join("pkg"));
        let config = loaded_config_at(root.clone(), r#"config := {serve: {wasm: "pkg"}}"#);
        let matches = matches(&["mech", "serve"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(
            effective.wasm_pkg,
            root.join("pkg").to_string_lossy().to_string()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_relative_paths_normalize_backslashes_with_validation() {
        let root = temp_root("normalize-backslashes");
        std::fs::create_dir_all(root.join("include")).unwrap();
        std::fs::write(root.join("include/style.css"), "body {}").unwrap();
        std::fs::write(root.join("include/shim.html"), "<html></html>").unwrap();
        create_wasm_pkg(&root.join("web/pkg"));
        let config = loaded_config_at(
            root.clone(),
            r#"config := {serve: {paths: ["docs\\reference"], stylesheets: ["include\\style.css"], shim: "include\\shim.html", wasm: "web\\pkg"}}"#,
        );
        let matches = matches(&["mech", "serve"]);
        let effective =
            effective_serve_options(matches.subcommand_matches("serve").unwrap(), Some(&config))
                .unwrap();
        assert_eq!(
            effective.paths,
            vec![root.join("docs/reference").to_string_lossy().to_string()]
        );
        assert_eq!(
            effective.stylesheet_paths,
            vec![root.join("include/style.css").to_string_lossy().to_string()]
        );
        assert_eq!(
            effective.shim_path,
            root.join("include/shim.html").to_string_lossy().to_string()
        );
        assert_eq!(
            effective.wasm_pkg,
            root.join("web/pkg").to_string_lossy().to_string()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_config_patch_rejects_unknown_log_level() {
        let mut patch = RuntimeConfigPatch::default();
        patch.diagnostics.log_level = Some("verbose".to_string());
        assert!(apply_runtime_config_patch(RuntimeConfig::default(), &patch).is_err());
    }
    #[test]
    fn runtime_config_patch_applies() {
        let config = loaded_config(
            r#"config := {runtime: {name: "configured", limits: {max-steps-per-turn: 42}, diagnostics: {trace-enabled: true, log-level: "debug"}}}"#,
        );
        let runtime =
            apply_runtime_config_patch(RuntimeConfig::default(), &config.document.runtime).unwrap();
        assert_eq!(runtime.name, "configured");
        assert_eq!(runtime.limits.max_steps_per_turn, Some(42));
        assert!(runtime.diagnostics.trace_enabled);
        assert_eq!(runtime.diagnostics.log_level, LogLevel::Debug);
    }
}

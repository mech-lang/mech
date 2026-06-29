use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use crate::*;
use mech_core::*;
use mech_runtime::parse_host_context_target;

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

pub fn load_cli_config(matches: &clap::ArgMatches) -> MResult<Option<LoadedMechConfig>> {
  let project_inputs: Vec<String> = matches
    .get_many::<String>("mech_serve_file_paths")
    .into_iter()
    .flatten()
    .cloned()
    .collect();
  load_cli_config_with_inputs(matches, &project_inputs, "Server")
}

pub fn load_cli_config_with_inputs(
  matches: &clap::ArgMatches,
  project_inputs: &[String],
  label: &str,
) -> MResult<Option<LoadedMechConfig>> {
  let no_config = matches.get_flag("no_config");
  let explicit_config = matches.get_one::<String>("config").map(|path| path.as_str());
  let current_dir = std::env::current_dir()?;
  let loaded = crate::load_optional_mech_config(
    &current_dir,
    explicit_config,
    no_config,
    project_inputs,
  )?;
  if let Some(config) = loaded.as_ref() {
    println!("[Mech {label}] Loading config… {}", config.path.display());
  }
  Ok(loaded)
}

pub fn load_run_cli_config(
  matches: &clap::ArgMatches,
  run_inputs: &[String],
) -> MResult<Option<LoadedMechConfig>> {
  load_cli_config_with_inputs(matches, run_inputs, "Run")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveCliHostGrants {
  pub env_read_paths: Vec<String>,
  pub stdout_write_paths: Vec<String>,
  pub stderr_write_paths: Vec<String>,
}

impl EffectiveCliHostGrants {
  pub fn empty() -> Self {
    Self {
      env_read_paths: Vec::new(),
      stdout_write_paths: Vec::new(),
      stderr_write_paths: Vec::new(),
    }
  }
}

impl Default for EffectiveCliHostGrants {
  fn default() -> Self {
    Self::empty()
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliHostCapabilitySelection {
  pub include_defaults: bool,
  pub profiles: Vec<String>,
}

impl Default for CliHostCapabilitySelection {
  fn default() -> Self {
    Self {
      include_defaults: true,
      profiles: Vec::new(),
    }
  }
}

const CLI_PROFILE_ENV: &str = ":cli/env";
const CLI_PROFILE_STDOUT: &str = ":cli/stdout";
const CLI_PROFILE_STDERR: &str = ":cli/stderr";

const DEFAULT_CLI_CAPABILITY_PROFILES: &[&str] = &[
  CLI_PROFILE_ENV,
  CLI_PROFILE_STDOUT,
  CLI_PROFILE_STDERR,
];

pub fn effective_cli_host_grants(
  config: Option<&LoadedMechConfig>,
  selection: CliHostCapabilitySelection,
) -> MResult<EffectiveCliHostGrants> {
  let mut grants = EffectiveCliHostGrants::empty();
  let has_explicit_cli_config_grants = match config {
    Some(config) => has_explicit_cli_run_grants(config)?,
    None => false,
  };

  if selection.include_defaults && !has_explicit_cli_config_grants {
    for profile in DEFAULT_CLI_CAPABILITY_PROFILES {
      grant_cli_profile(&mut grants, profile)?;
    }
  }

  for profile in &selection.profiles {
    grant_cli_profile(&mut grants, profile)?;
  }

  Ok(grants)
}

pub fn has_explicit_cli_run_grants(config: &LoadedMechConfig) -> MResult<bool> {
  let Some(run) = config.document.run.as_ref() else {
    return Ok(false);
  };

  let mut cli_instances = std::collections::BTreeSet::from(["cli".to_string()]);
  for host in &config.document.hosts {
    if host.provider == "cli" {
      cli_instances.insert(host.name.clone());
    }
  }

  for grant in &run.grants {
    let (instance, context) = parse_host_context_target(&grant.target)?;
    if cli_instances.contains(instance) && matches!(context, "env" | "stdout" | "stderr") {
      return Ok(true);
    }
  }

  Ok(false)
}

fn grant_cli_profile(
  grants: &mut EffectiveCliHostGrants,
  profile: &str,
) -> MResult<()> {
  match profile {
    CLI_PROFILE_ENV => {
      if !grants.env_read_paths.iter().any(|path| path == "*") {
        grants.env_read_paths.clear();
        grants.env_read_paths.push("*".to_string());
      }
      Ok(())
    }
    CLI_PROFILE_STDOUT => {
      union_string(&mut grants.stdout_write_paths, "text");
      union_string(&mut grants.stdout_write_paths, "line");
      Ok(())
    }
    CLI_PROFILE_STDERR => {
      union_string(&mut grants.stderr_write_paths, "text");
      union_string(&mut grants.stderr_write_paths, "line");
      Ok(())
    }
    other => Err(MechError::new(
      GenericError {
        msg: format!("unknown CLI capability profile `{other}`"),
      },
      None,
    ).with_compiler_loc()),
  }
}

fn union_string(paths: &mut Vec<String>, value: &str) {
  if !paths.iter().any(|path| path == value) {
    paths.push(value.to_string());
  }
}

fn intersect_env_paths(current: &[String], configured: &[String]) -> Vec<String> {
  if current.is_empty() || configured.is_empty() {
    return Vec::new();
  }
  if current.iter().any(|path| path == "*") {
    return configured.to_vec();
  }
  if configured.iter().any(|path| path == "*") {
    return current.to_vec();
  }
  intersect_paths(current, configured)
}

fn intersect_paths(current: &[String], configured: &[String]) -> Vec<String> {
  current
    .iter()
    .filter(|path| configured.iter().any(|configured_path| configured_path == *path))
    .cloned()
    .collect()
}

fn validated_cli_stream_paths(where_: &str, paths: &[String]) -> MResult<Vec<String>> {
  let mut out = Vec::new();
  for path in paths {
    match path.as_str() {
      "text" | "line" => out.push(path.clone()),
      other => {
        return Err(MechError::new(
          GenericError {
            msg: format!("{where_} cannot grant unsupported CLI stream path `{other}`"),
          },
          None,
        ).with_compiler_loc());
      }
    }
  }
  Ok(out)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveRunOptions {
  pub paths: Vec<String>,
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
      ).with_compiler_loc());
    }

    return Ok(None);
  }

  Ok(Some(EffectiveRunOptions { paths }))
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

  let mut cli_paths: Vec<String> = serve_matches
    .get_many::<String>("mech_serve_file_paths")
    .into_iter()
    .flatten()
    .cloned()
    .collect();

  if let Some(loaded) = config {
    if let Some(project_dir) = loaded.discovered_project_dir.as_ref() {
      if cli_paths.len() == 1 {
        let current_dir = std::env::current_dir()?;
        let input = PathBuf::from(&cli_paths[0]);
        let input_path = if input.is_absolute() {
          input
        } else {
          current_dir.join(input)
        };

        if input_path.exists()
          && input_path.is_dir()
          && input_path.canonicalize()? == *project_dir
        {
          cli_paths.clear();
        }
      }
    }
  }

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

#[cfg(test)]
mod config_tests {
  use super::*;
  use mech_runtime::{ConfigProfileOptions, MechConfigDocument, DEFAULT_CONFIG_FILENAME};
  use std::time::{SystemTime, UNIX_EPOCH};

  struct CurrentDirGuard {
    previous: PathBuf,
    _lock: std::sync::MutexGuard<'static, ()>,
  }

  impl CurrentDirGuard {
    fn enter(path: &std::path::Path) -> Self {
      let lock = crate::cli::CURRENT_DIR_LOCK.lock().unwrap();
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
      discovered_project_dir: None,
    }
  }

  fn loaded_config_at(base_dir: PathBuf, source: &str) -> LoadedMechConfig {
    LoadedMechConfig {
      path: base_dir.join("mech.mcfg"),
      base_dir,
      document: parse_document(source),
      discovered_project_dir: None,
    }
  }

  fn loaded_run_project_config_at(base_dir: PathBuf, source: &str, project_dir: PathBuf) -> LoadedMechConfig {
    LoadedMechConfig {
      path: base_dir.join("mech.mcfg"),
      base_dir,
      document: parse_document(source),
      discovered_project_dir: Some(project_dir),
    }
  }

  fn create_wasm_pkg(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    std::fs::write(path.join("mech_wasm_bg.wasm.br"), b"wasm").unwrap();
    std::fs::write(path.join("mech_wasm.js"), b"js").unwrap();
  }

  fn create_serve_project_with_config_name(root: &Path, config_name: &str) -> PathBuf {
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("index.html"), "<html></html>").unwrap();
    std::fs::write(project.join("demo.mec"), "# demo\n").unwrap();
    std::fs::write(
      project.join(config_name),
      r#"config := {
  serve: {
  paths: ["demo.mec"]
  shim: "index.html"
  }
}
"#,
    )
    .unwrap();
    project
  }

  fn create_serve_project(root: &Path) -> PathBuf {
    create_serve_project_with_config_name(root, DEFAULT_CONFIG_FILENAME)
  }

  fn error_text(result: MResult<EffectiveServeOptions>) -> String {
    format!("{:?}", result.unwrap_err())
  }

  fn run_error_text(result: MResult<Option<EffectiveRunOptions>>) -> String {
    format!("{:?}", result.unwrap_err())
  }

  #[test]
  fn config_run_paths_used_when_cli_paths_absent() {
    let config = loaded_config(r#"config := {run: {paths: ["foo.mec", "bar.mec"]}}"#);
    let effective = effective_run_options(vec![], Some(&config), true)
      .unwrap()
      .unwrap();
    assert_eq!(effective.paths, vec!["foo.mec", "bar.mec"]);
  }

  #[test]
  fn cli_run_paths_override_config_paths() {
    let config = loaded_config(r#"config := {run: {paths: ["foo.mec"]}}"#);
    let effective = effective_run_options(vec!["cli.mec".to_string()], Some(&config), true)
      .unwrap()
      .unwrap();
    assert_eq!(effective.paths, vec!["cli.mec"]);
  }

  #[test]
  fn run_project_directory_uses_config_paths_not_selector_path() {
    let root = temp_root("run-project-uses-config-paths");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let config = loaded_run_project_config_at(
      project.clone(),
      r#"config := {run: {paths: ["demo.mec"]}}"#,
      project.canonicalize().unwrap(),
    );
    {
      let _guard = CurrentDirGuard::enter(&root);
      let effective = effective_run_options(vec!["project".to_string()], Some(&config), false)
        .unwrap()
        .unwrap();
      assert_eq!(
        effective.paths,
        vec![project.join("demo.mec").to_string_lossy().to_string()]
      );
    }
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn implicit_run_without_inputs_returns_none() {
    let options = effective_run_options(vec![], None, false).unwrap();
    assert_eq!(options, None);
  }

  #[test]
  fn implicit_run_without_cli_selector_ignores_config_run_paths() {
    let config = loaded_config(r#"config := {run: {paths: ["foo.mec"]}}"#);
    let options = effective_run_options(vec![], Some(&config), false).unwrap();
    assert_eq!(options, None);
  }

  #[test]
  fn explicit_run_without_inputs_and_without_config_paths_errors() {
    let msg = run_error_text(effective_run_options(vec![], None, true));
    assert!(msg.contains("no run inputs supplied"));
  }





  #[test]
  fn serve_project_directory_uses_config_paths_not_selector_path() {
    let root = temp_root("serve-project-uses-config-paths");
    let project = create_serve_project(&root);
    {
      let _guard = CurrentDirGuard::enter(&root);
      let matches = matches(&["mech", "serve", "project"]);
      let serve_matches = matches.subcommand_matches("serve").unwrap();
      let loaded = load_cli_config(serve_matches).unwrap().unwrap();
      let options = effective_serve_options(serve_matches, Some(&loaded)).unwrap();
      assert_eq!(
        options.paths,
        vec![project.join("demo.mec").to_string_lossy().to_string()]
      );
      assert_eq!(
        options.shim_path,
        project.join("index.html").to_string_lossy().to_string()
      );
      assert!(!options.paths.iter().any(|path| path == "project"));
    }
    std::fs::remove_dir_all(root).unwrap();
  }




  #[test]
  fn project_directory_without_mech_config_falls_back_to_current_dir_config() {
    let root = temp_root("project-without-config-falls-back");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
      root.join(DEFAULT_CONFIG_FILENAME),
      "config := {serve: {port: 9090}}\n",
    )
    .unwrap();
    {
      let _guard = CurrentDirGuard::enter(&root);
      let matches = matches(&["mech", "serve", "project"]);
      let loaded = load_cli_config(matches.subcommand_matches("serve").unwrap())
        .unwrap()
        .unwrap();
      assert_eq!(loaded.path, root.join(DEFAULT_CONFIG_FILENAME));
      assert_eq!(loaded.discovered_project_dir, None);
    }
    std::fs::remove_dir_all(root).unwrap();
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

  fn stdout_selection() -> CliHostCapabilitySelection {
    CliHostCapabilitySelection {
      include_defaults: false,
      profiles: vec![":cli/stdout".to_string()],
    }
  }

  #[test]
  fn default_cli_host_grants_include_default_envelope() {
    let grants = effective_cli_host_grants(None, CliHostCapabilitySelection::default()).unwrap();
    assert_eq!(grants.env_read_paths, vec!["*".to_string()]);
    assert_eq!(grants.stdout_write_paths, vec!["text".to_string(), "line".to_string()]);
    assert_eq!(grants.stderr_write_paths, vec!["text".to_string(), "line".to_string()]);
  }

  #[test]
  fn explicit_stdout_only_selection_grants_only_stdout() {
    let grants = effective_cli_host_grants(None, stdout_selection()).unwrap();
    assert!(grants.env_read_paths.is_empty());
    assert_eq!(grants.stdout_write_paths, vec!["text".to_string(), "line".to_string()]);
    assert!(grants.stderr_write_paths.is_empty());
  }

  #[test]
  fn explicit_stdout_and_env_selection_grants_stdout_and_env_only() {
    let grants = effective_cli_host_grants(
      None,
      CliHostCapabilitySelection {
        include_defaults: false,
        profiles: vec![":cli/stdout".to_string(), ":cli/env".to_string()],
      },
    )
    .unwrap();
    assert_eq!(grants.env_read_paths, vec!["*".to_string()]);
    assert_eq!(grants.stdout_write_paths, vec!["text".to_string(), "line".to_string()]);
    assert!(grants.stderr_write_paths.is_empty());
  }

  #[test]
  fn unknown_cli_capability_profile_errors() {
    let error = effective_cli_host_grants(
      None,
      CliHostCapabilitySelection {
        include_defaults: false,
        profiles: vec![":quxx".to_string()],
      },
    )
    .unwrap_err();
    assert!(format!("{error:?}").contains("unknown CLI capability profile `:quxx`"));
  }

  #[test]
  fn explicit_config_run_grants_suppress_implicit_defaults() {
    let root = temp_root("cli-stdout-line");
    let config = loaded_config_at(
      root.clone(),
      r#"config := { run: { grants: [{target: "cli/stdout", operations: ["write"], paths: ["line"]}] } }"#,
    );
    let grants = effective_cli_host_grants(Some(&config), CliHostCapabilitySelection::default()).unwrap();
    assert!(grants.env_read_paths.is_empty());
    assert!(grants.stdout_write_paths.is_empty());
    assert!(grants.stderr_write_paths.is_empty());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn deny_default_capabilities_still_suppresses_defaults_without_config_grants() {
    let grants = effective_cli_host_grants(
      None,
      CliHostCapabilitySelection {
        include_defaults: false,
        profiles: Vec::new(),
      },
    ).unwrap();
    assert!(grants.env_read_paths.is_empty());
    assert!(grants.stdout_write_paths.is_empty());
    assert!(grants.stderr_write_paths.is_empty());
  }

  #[test]
  fn explicit_cli_profiles_remain_additive_with_config_grants() {
    let root = temp_root("cli-stdout-line-additive");
    let config = loaded_config_at(
      root.clone(),
      r#"config := { run: { grants: [{target: "cli/stdout", operations: ["write"], paths: ["line"]}] } }"#,
    );
    let grants = effective_cli_host_grants(Some(&config), stdout_selection()).unwrap();
    assert!(grants.env_read_paths.is_empty());
    assert_eq!(grants.stdout_write_paths, vec!["text".to_string(), "line".to_string()]);
    assert!(grants.stderr_write_paths.is_empty());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn explicit_cli_run_grants_detect_configured_cli_aliases() {
    let root = temp_root("cli-alias-explicit");
    let config = loaded_config_at(
      root.clone(),
      r#"config := { hosts: [{name: "term", provider: "cli", settings: {}}] run: { grants: [{target: "term/stderr", operations: ["write"], paths: ["line"]}] } }"#,
    );
    assert!(has_explicit_cli_run_grants(&config).unwrap());
    std::fs::remove_dir_all(root).unwrap();
  }

}

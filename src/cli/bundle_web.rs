use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use mech_core::*;

use crate::{
  discover_project_config, load_mech_config_path, require_config_file, resolve_config_path,
  resolve_project_dir_input, BundleWebOptions, LoadedMechConfig,
};

fn expand_serve_source_paths(base_dir: &Path, paths: &[PathBuf]) -> MResult<Vec<PathBuf>> {
  let mut visited = BTreeSet::new();
  let mut out = Vec::new();
  for path in paths {
    let resolved = resolve_config_path(base_dir, path);
    collect_serve_source_path(&resolved, &mut visited, &mut out)?;
  }
  out.sort();
  out.dedup();
  Ok(out)
}

fn collect_serve_source_path(path: &Path, visited: &mut BTreeSet<PathBuf>, out: &mut Vec<PathBuf>) -> MResult<()> {
  let metadata = std::fs::symlink_metadata(path)?;
  if metadata.file_type().is_symlink() { return Ok(()); }
  if metadata.is_file() {
    require_file("serve.paths", path)?;
    if is_mech_source(path) { out.push(path.to_path_buf()); }
    return Ok(());
  }
  if !metadata.is_dir() { return Ok(()); }
  let canonical = path.canonicalize()?;
  if !visited.insert(canonical) { return Ok(()); }
  let mut entries = std::fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
  entries.sort_by_key(|entry| entry.path());
  for entry in entries { collect_serve_source_path(&entry.path(), visited, out)?; }
  Ok(())
}

fn is_mech_source(path: &Path) -> bool {
  matches!(path.extension().and_then(OsStr::to_str), Some("mec" | "🤖"))
}

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

pub fn bundle_web_command() -> clap::Command {
  Command::new("bundle-web")
    .about("Bundle a Mech browser project into static web assets.")
    .arg(
      Arg::new("project_path")
        .help("Project directory to bundle.")
        .required(true),
    )
    .arg(
      Arg::new("out")
        .short('o')
        .long("out")
        .value_name("OUTPUT_DIR")
        .num_args(1)
        .required(true)
        .help("Destination directory for static bundle output."),
    )
    .arg(
      Arg::new("shim")
        .short('m')
        .long("shim")
        .value_name("PATH")
        .num_args(1)
        .help("HTML shim to use as the bundle index."),
    )
    .arg(
      Arg::new("stylesheet")
        .short('s')
        .long("stylesheet")
        .value_name("PATH")
        .num_args(1..)
        .action(ArgAction::Append)
        .help("Additional stylesheet to include in the bundle."),
    )
    .arg(
      Arg::new("wasm")
        .short('w')
        .long("wasm")
        .value_name("PATH")
        .num_args(1)
        .help("Path to the wasm package directory."),
    )
    .args(host_delegation_args())
}

#[cfg(feature = "host_delegation_signing")]
fn host_delegation_args() -> Vec<Arg> {
  vec![
    Arg::new("host_delegation_key").long("host-delegation-key").value_name("PATH").num_args(1),
    Arg::new("host_delegation_public_key").long("host-delegation-public-key").value_name("PATH").num_args(1),
    Arg::new("host_delegation_key_id").long("host-delegation-key-id").value_name("ID").num_args(1),
    Arg::new("host_delegation_issuer").long("host-delegation-issuer").value_name("ISSUER").num_args(1),
    Arg::new("host_delegation_subject").long("host-delegation-subject").value_name("SUBJECT").num_args(1),
    Arg::new("host_delegation_audience").long("host-delegation-audience").value_name("AUDIENCE").num_args(1),
    Arg::new("host_delegation_expires_ms").long("host-delegation-expires-ms").value_name("MS").num_args(1),
  ]
}

#[cfg(not(feature = "host_delegation_signing"))]
fn host_delegation_args() -> Vec<Arg> {
  Vec::new()
}

pub fn load_bundle_web_config(matches: &clap::ArgMatches) -> MResult<LoadedMechConfig> {
  if matches.get_flag("no_config") {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      "bundle-web requires a config; remove --no-config or pass --config",
    )
    .into());
  }

  let current_dir = std::env::current_dir()?;
  if let Some(path) = matches.get_one::<String>("config") {
    let path = resolve_current_dir_path(&current_dir, Path::new(path));
    return load_mech_config_path(path, None);
  }

  let project_path = matches
    .get_one::<String>("project_path")
    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "bundle-web requires project_path"))?;
  let project_dir = resolve_project_dir_input(project_path, &current_dir)?.ok_or_else(|| {
    let path = resolve_current_dir_path(&current_dir, Path::new(project_path));
    Error::new(
      ErrorKind::InvalidInput,
      format!("bundle-web project path must be an existing directory: {}", path.display()),
    )
  })?;

  let discovery = discover_project_config(&project_dir)?.ok_or_else(|| {
    Error::new(
      ErrorKind::InvalidInput,
      format!("bundle-web requires a project config in {}", project_dir.display()),
    )
  })?;

  load_mech_config_path(discovery.config_path, Some(discovery.project_dir))
}

pub fn effective_bundle_web_options(
  matches: &clap::ArgMatches,
  loaded: LoadedMechConfig,
) -> MResult<BundleWebOptions> {
  let current_dir = std::env::current_dir()?;
  let project_path = matches
    .get_one::<String>("project_path")
    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "bundle-web requires project_path"))?;
  let project_dir = resolve_current_dir_path(&current_dir, Path::new(project_path));
  if !project_dir.is_dir() {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      format!("bundle-web project path must be an existing directory: {}", project_dir.display()),
    )
    .into());
  }
  let project_dir = project_dir.canonicalize()?;

  let out = matches
    .get_one::<String>("out")
    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "bundle-web requires --out"))?;
  let output_dir = resolve_current_dir_path(&current_dir, Path::new(out));

  let serve_config = loaded.document.serve.as_ref();
  let source_paths = serve_config
    .map(|serve| expand_serve_source_paths(&loaded.base_dir, &serve.paths))
    .transpose()?
    .unwrap_or_default();
  if source_paths.is_empty() {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      "bundle-web requires serve.paths in the project config",
    )
    .into());
  }

  let shim_path = matches
    .get_one::<String>("shim")
    .map(|path| resolve_current_dir_path(&current_dir, Path::new(path)))
    .or_else(|| {
      serve_config
        .and_then(|serve| serve.shim.as_ref())
        .map(|path| resolve_config_path(&loaded.base_dir, path))
    })
    .ok_or_else(|| {
      Error::new(
        ErrorKind::InvalidInput,
        "bundle-web requires a shim via --shim or serve.shim",
      )
    })?;
  require_file("serve.shim", &shim_path)?;

  let mut stylesheet_paths = serve_config
    .map(|serve| {
      serve
        .stylesheets
        .iter()
        .map(|path| resolve_config_path(&loaded.base_dir, path))
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();
  stylesheet_paths.extend(
    matches
      .get_many::<String>("stylesheet")
      .into_iter()
      .flatten()
      .map(|path| resolve_current_dir_path(&current_dir, Path::new(path))),
  );
  for path in &stylesheet_paths {
    require_config_file("serve.stylesheets", path)?;
  }

  let wasm_pkg = matches
    .get_one::<String>("wasm")
    .map(|path| resolve_current_dir_path(&current_dir, Path::new(path)))
    .or_else(|| {
      serve_config
        .and_then(|serve| serve.wasm.as_ref())
        .map(|path| resolve_config_path(&loaded.base_dir, path))
    })
    .ok_or_else(|| {
      Error::new(
        ErrorKind::InvalidInput,
        "bundle-web requires a wasm package via --wasm or serve.wasm",
      )
    })?;
  require_bundle_wasm_package(&wasm_pkg)?;

  #[cfg(feature = "host_delegation_signing")]
  let host_config_injection = host_delegation_signing_options(
    matches,
    &loaded,
    &format!("browser://bundle/{}", loaded.document.runtime.name.clone().unwrap_or_else(|| "mech".to_string())),
  )?;
  #[cfg(not(feature = "host_delegation_signing"))]
  let host_config_injection = None;

  Ok(BundleWebOptions {
    project_dir,
    output_dir,
    source_paths,
    shim_path,
    stylesheet_paths,
    wasm_pkg,
    loaded_config: loaded,
    host_config_injection,
  })
}


#[cfg(feature = "host_delegation_signing")]
fn host_delegation_signing_options(
  matches: &clap::ArgMatches,
  loaded: &LoadedMechConfig,
  default_audience: &str,
) -> MResult<Option<crate::HostAuthorityInjection>> {
  let Some(private_key) = matches.get_one::<String>("host_delegation_key") else {
    return Ok(None);
  };
  let public_key = matches
    .get_one::<String>("host_delegation_public_key")
    .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "--host-delegation-public-key is required with --host-delegation-key"))?;
  let current_dir = std::env::current_dir()?;
  let options = crate::HostDelegationSigningOptions {
    private_key_path: resolve_current_dir_path(&current_dir, Path::new(private_key)),
    public_key_path: resolve_current_dir_path(&current_dir, Path::new(public_key)),
    key_id: matches.get_one::<String>("host_delegation_key_id").cloned().unwrap_or_else(|| "dev".to_string()),
    issuer: matches.get_one::<String>("host_delegation_issuer").cloned().unwrap_or_else(|| "host://mech-cli".to_string()),
    subject: matches.get_one::<String>("host_delegation_subject").cloned().unwrap_or_else(|| "wasm://browser".to_string()),
    audience: matches.get_one::<String>("host_delegation_audience").cloned().unwrap_or_else(|| default_audience.to_string()),
    expires_ms: matches.get_one::<String>("host_delegation_expires_ms").map(|value| value.parse()).transpose().map_err(|_| Error::new(ErrorKind::InvalidInput, "--host-delegation-expires-ms must be an integer"))?,
  };
  let runtime_config = crate::apply_runtime_config_patch(
    mech_runtime::RuntimeConfig::default(),
    &loaded.document.runtime,
  )?;
  let host_config = crate::web_runtime_injection_config_from_document(
    &loaded.document,
    &runtime_config,
  )?;
  let now_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))?
    .as_millis() as u64;
  crate::signed_browser_runtime_injection_config(host_config, &options, now_ms).map(Some)
}

fn resolve_current_dir_path(current_dir: &Path, path: &Path) -> PathBuf {
  if path.is_absolute() {
    path.to_path_buf()
  } else {
    current_dir.join(path)
  }
}

fn require_file(field: &str, path: &Path) -> MResult<()> {
  if path.is_file() {
    Ok(())
  } else {
    Err(Error::new(
      ErrorKind::InvalidInput,
      format!("configuration error: {field} must be an existing file: {}", path.display()),
    )
    .into())
  }
}

fn require_bundle_wasm_package(path: &Path) -> MResult<()> {
  if !path.is_dir() {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      format!("configuration error: serve.wasm must be an existing directory: {}", path.display()),
    )
    .into());
  }

  for file in ["mech_wasm.js", "mech_wasm_bg.wasm"] {
    let required = path.join(file);
    if !required.is_file() {
      return Err(Error::new(
        ErrorKind::InvalidInput,
        format!("configuration error: serve.wasm is missing required file: {}", required.display()),
      )
      .into());
    }
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::cli::CURRENT_DIR_LOCK;
  use std::time::{SystemTime, UNIX_EPOCH};

  struct CurrentDirGuard {
    previous: PathBuf,
    _lock: std::sync::MutexGuard<'static, ()>,
  }

  impl CurrentDirGuard {
    fn enter(path: &Path) -> Self {
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

  fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-cli-bundle-web-{name}-{}",
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
  }

  fn command() -> Command {
    add_config_args(Command::new("mech").subcommand(bundle_web_command()))
  }

  fn matches(args: &[&str]) -> clap::ArgMatches {
    command().try_get_matches_from(args).unwrap()
  }

  fn bundle_matches(args: &[&str]) -> clap::ArgMatches {
    matches(args)
      .subcommand_matches("bundle-web")
      .unwrap()
      .clone()
  }

  fn write_project(root: &Path, name: &str, runtime_name: &str) -> PathBuf {
    let project = root.join(name);
    std::fs::create_dir_all(project.join("pkg")).unwrap();
    std::fs::write(project.join("index.html"), "<html><head></head></html>").unwrap();
    std::fs::write(project.join("demo.mec"), "x := 1\n").unwrap();
    std::fs::write(project.join("style.css"), "body {}\n").unwrap();
    std::fs::write(project.join("pkg/mech_wasm.js"), "export {};\n").unwrap();
    std::fs::write(project.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
    std::fs::write(
      project.join("demo.mcfg"),
      format!(
        r#"config := {{
  runtime: {{name: "{runtime_name}"}}
  serve: {{
    paths: ["demo.mec"]
    shim: "index.html"
    stylesheets: ["style.css"]
    wasm: "pkg"
  }}
}}
"#,
      ),
    )
    .unwrap();
    project
  }

  #[test]
  fn explicit_config_wins_over_discovered_project_config() {
    let root = temp_root("explicit-wins");
    let discovered = write_project(&root, "discovered", "discovered");
    let explicit = write_project(&root, "explicit", "explicit");
    let _guard = CurrentDirGuard::enter(&root);

    let matches = bundle_matches(&[
      "mech",
      "--config",
      "explicit/demo.mcfg",
      "bundle-web",
      "discovered",
      "--out",
      "out",
    ]);
    let loaded = load_bundle_web_config(&matches).unwrap();

    assert_eq!(loaded.path, explicit.join("demo.mcfg").canonicalize().unwrap());
    assert_ne!(loaded.path, discovered.join("demo.mcfg").canonicalize().unwrap());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn no_config_errors() {
    let root = temp_root("no-config");
    write_project(&root, "project", "project");
    let _guard = CurrentDirGuard::enter(&root);

    let matches = bundle_matches(&["mech", "--no-config", "bundle-web", "project", "--out", "out"]);
    let error = format!("{:?}", load_bundle_web_config(&matches).unwrap_err());

    assert!(error.contains("bundle-web requires a config"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn missing_project_config_errors() {
    let root = temp_root("missing-config");
    std::fs::create_dir_all(root.join("project")).unwrap();
    let _guard = CurrentDirGuard::enter(&root);

    let matches = bundle_matches(&["mech", "bundle-web", "project", "--out", "out"]);
    let error = format!("{:?}", load_bundle_web_config(&matches).unwrap_err());

    assert!(error.contains("bundle-web requires a project config"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn cli_shim_stylesheet_and_wasm_override_config_paths() {
    let root = temp_root("overrides");
    write_project(&root, "project", "project");
    std::fs::create_dir_all(root.join("override-pkg")).unwrap();
    std::fs::write(root.join("override.html"), "<html><head></head></html>").unwrap();
    std::fs::write(root.join("override.css"), "html {}\n").unwrap();
    std::fs::write(root.join("override-pkg/mech_wasm.js"), "export {};\n").unwrap();
    std::fs::write(root.join("override-pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
    let _guard = CurrentDirGuard::enter(&root);

    let matches = bundle_matches(&[
      "mech",
      "bundle-web",
      "project",
      "--out",
      "out",
      "--shim",
      "override.html",
      "--stylesheet",
      "override.css",
      "--wasm",
      "override-pkg",
    ]);
    let loaded = load_bundle_web_config(&matches).unwrap();
    let options = effective_bundle_web_options(&matches, loaded).unwrap();

    assert_eq!(options.shim_path, root.join("override.html"));
    assert!(options.stylesheet_paths.contains(&root.join("override.css")));
    assert_eq!(options.wasm_pkg, root.join("override-pkg"));
    std::fs::remove_dir_all(root).unwrap();
  }
}

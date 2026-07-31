use std::path::{Path, PathBuf};

use mech_core::*;
use mech_runtime::{
  ConfigProfileOptions, LogLevel, MechConfigDocument, RuntimeConfig, RuntimeConfigPatch,
  DEFAULT_CONFIG_FILENAME,
};

#[derive(Clone, Debug)]
pub struct LoadedMechConfig {
  pub path: PathBuf,
  pub base_dir: PathBuf,
  pub document: MechConfigDocument,
  pub discovered_project_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectConfigDiscovery {
  pub config_path: PathBuf,
  pub project_dir: PathBuf,
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

pub fn discover_project_config(project_dir: &Path) -> MResult<Option<ProjectConfigDiscovery>> {
  if !project_dir.exists() || !project_dir.is_dir() {
    return Ok(None);
  }

  let project_dir = project_dir.canonicalize()?;
  let default_config_path = project_dir.join(DEFAULT_CONFIG_FILENAME);
  if default_config_path.is_file() {
    return Ok(Some(ProjectConfigDiscovery {
      config_path: default_config_path.canonicalize()?,
      project_dir,
    }));
  }

  let mut candidates = std::fs::read_dir(&project_dir)?
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| {
      path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("mcfg")
    })
    .collect::<Vec<_>>();

  candidates.sort();

  match candidates.len() {
    0 => Ok(None),
    1 => Ok(Some(ProjectConfigDiscovery {
      config_path: candidates[0].canonicalize()?,
      project_dir,
    })),
    _ => {
      let names = candidates
        .iter()
        .map(|path| {
          path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string())
        })
        .collect::<Vec<_>>()
        .join(", ");

      Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!(
          "ambiguous project config in {}: found multiple .mcfg files: {}",
          project_dir.display(),
          names
        ),
      )
      .into())
    }
  }
}

pub fn resolve_project_dir_input(input: &str, current_dir: &Path) -> MResult<Option<PathBuf>> {
  let input = PathBuf::from(input);
  let project_dir = if input.is_absolute() {
    input
  } else {
    current_dir.join(input)
  };

  if !project_dir.exists() || !project_dir.is_dir() {
    return Ok(None);
  }

  Ok(Some(project_dir.canonicalize()?))
}

pub fn discover_project_config_from_single_project_input(
  inputs: &[String],
  current_dir: &Path,
) -> MResult<Option<ProjectConfigDiscovery>> {
  if inputs.len() != 1 {
    return Ok(None);
  }

  let Some(project_dir) = resolve_project_dir_input(&inputs[0], current_dir)? else {
    return Ok(None);
  };

  discover_project_config(&project_dir)
}

pub fn load_mech_config_path(
  path: PathBuf,
  discovered_project_dir: Option<PathBuf>,
) -> MResult<LoadedMechConfig> {
  let path = path.canonicalize()?;
  let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
  let source = std::fs::read_to_string(&path)?;
  let document = mech_runtime::parse_config_document(
    path.display().to_string(),
    &source,
    ConfigProfileOptions::default(),
  )?;

  Ok(LoadedMechConfig {
    path,
    base_dir,
    document,
    discovered_project_dir,
  })
}

pub fn load_optional_mech_config(
  current_dir: &Path,
  explicit_config: Option<&str>,
  no_config: bool,
  project_inputs: &[String],
) -> MResult<Option<LoadedMechConfig>> {
  if no_config {
    return Ok(None);
  }

  if let Some(path) = explicit_config {
    let path = PathBuf::from(path);
    let path = if path.is_absolute() { path } else { current_dir.join(path) };
    return load_mech_config_path(path, None).map(Some);
  }

  if let Some(discovery) =
    discover_project_config_from_single_project_input(project_inputs, current_dir)?
  {
    return load_mech_config_path(discovery.config_path, Some(discovery.project_dir)).map(Some);
  }

  let path = current_dir.join(DEFAULT_CONFIG_FILENAME);
  if path.exists() {
    load_mech_config_path(path, None).map(Some)
  } else {
    Ok(None)
  }
}

pub fn require_config_file(field: &str, path: &Path) -> MResult<()> {
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

pub fn require_config_dir(field: &str, path: &Path) -> MResult<()> {
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

pub fn require_config_wasm_package(field: &str, path: &Path) -> MResult<()> {
  require_config_dir(field, path)?;

  let wasm = path.join("mech_wasm_bg.wasm");
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
mod tests {
  use super::*;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-project-{name}-{}",
      SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
  }

  fn create_project_with_config_name(root: &Path, config_name: &str) -> PathBuf {
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("index.html"), "<html></html>").unwrap();
    std::fs::write(project.join("demo.mec"), "# demo\n").unwrap();
    std::fs::write(project.join(config_name), "config := {runtime: {name: \"test\"}}\n").unwrap();
    project
  }

  fn create_project(root: &Path) -> PathBuf {
    create_project_with_config_name(root, DEFAULT_CONFIG_FILENAME)
  }

  #[test]
  fn discover_project_config_prefers_default_mcfg_over_other_mcfg() {
    let root = temp_root("prefers-default");
    let project = create_project_with_config_name(&root, "demo.mcfg");
    std::fs::write(project.join(DEFAULT_CONFIG_FILENAME), "config := {runtime: {name: \"test\"}}\n")
      .unwrap();

    let discovery = discover_project_config(&project).unwrap().unwrap();
    assert_eq!(discovery.config_path, project.join(DEFAULT_CONFIG_FILENAME).canonicalize().unwrap());
    assert_eq!(discovery.project_dir, project.canonicalize().unwrap());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn discover_project_config_finds_single_non_default_mcfg() {
    let root = temp_root("single-non-default");
    let project = create_project_with_config_name(&root, "demo.mcfg");

    let discovery = discover_project_config(&project).unwrap().unwrap();
    assert_eq!(discovery.config_path, project.join("demo.mcfg").canonicalize().unwrap());
    assert_eq!(discovery.project_dir, project.canonicalize().unwrap());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn discover_project_config_errors_on_multiple_non_default_mcfg() {
    let root = temp_root("ambiguous");
    let project = root.join("project");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("a.mcfg"), "config := {runtime: {name: \"test\"}}\n").unwrap();
    std::fs::write(project.join("b.mcfg"), "config := {runtime: {name: \"test\"}}\n").unwrap();

    let error = format!("{:?}", discover_project_config(&project).unwrap_err());
    assert!(error.contains("ambiguous project config"));
    assert!(error.contains("a.mcfg"));
    assert!(error.contains("b.mcfg"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn load_optional_mech_config_honors_explicit_config_over_project_dir_discovery() {
    let root = temp_root("explicit-wins");
    let project = create_project(&root);
    std::fs::write(root.join("explicit.mcfg"), "config := {runtime: {name: \"test\"}}\n").unwrap();

    let loaded = load_optional_mech_config(
      &root,
      Some("explicit.mcfg"),
      false,
      &["project".to_string()],
    )
    .unwrap()
    .unwrap();
    assert_eq!(loaded.path, root.join("explicit.mcfg"));
    assert_eq!(loaded.discovered_project_dir, None);
    assert_ne!(loaded.path, project.join(DEFAULT_CONFIG_FILENAME));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn load_optional_mech_config_returns_none_when_no_config_is_true() {
    let root = temp_root("no-config");
    create_project(&root);

    let loaded = load_optional_mech_config(&root, None, true, &["project".to_string()]).unwrap();
    assert!(loaded.is_none());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn load_optional_mech_config_falls_back_to_current_dir_default_config() {
    let root = temp_root("current-dir-default");
    std::fs::write(root.join(DEFAULT_CONFIG_FILENAME), "config := {runtime: {name: \"test\"}}\n")
      .unwrap();

    let loaded = load_optional_mech_config(&root, None, false, &[]).unwrap().unwrap();
    assert_eq!(loaded.path, root.join(DEFAULT_CONFIG_FILENAME));
    assert_eq!(loaded.discovered_project_dir, None);
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
    let document = mech_runtime::parse_config_document(
      "test.mcfg".to_string(),
      r#"config := {runtime: {name: "configured", limits: {max-steps-per-turn: 42}, diagnostics: {trace-enabled: true, log-level: "debug"}}}"#,
      ConfigProfileOptions::default(),
    )
    .unwrap();
    let runtime = apply_runtime_config_patch(RuntimeConfig::default(), &document.runtime).unwrap();
    assert_eq!(runtime.name, "configured");
    assert_eq!(runtime.limits.max_steps_per_turn, Some(42));
    assert!(runtime.diagnostics.trace_enabled);
    assert_eq!(runtime.diagnostics.log_level, LogLevel::Debug);
  }
}

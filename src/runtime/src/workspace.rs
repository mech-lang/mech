use std::{
  collections::{BTreeMap, BTreeSet, VecDeque},
  hash::{DefaultHasher, Hash, Hasher},
  path::{Path, PathBuf},
  time::SystemTime,
};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::{
  MechRuntime,
  ModuleBuildOptions,
  ModuleVersionId,
};

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceConfig {
  pub root: PathBuf,
  pub targets: Vec<RuntimeWorkspaceTarget>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceTarget {
  pub name: String,
  pub specifier: String,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorkspace {
  config: RuntimeWorkspaceConfig,
  snapshot: Option<RuntimeWorkspaceSnapshot>,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeWorkspaceSnapshot {
  pub root: PathBuf,
  pub targets: BTreeMap<String, RuntimeWorkspaceTargetSnapshot>,
  pub sources: BTreeMap<String, RuntimeWorkspaceSourceSnapshot>,
  pub import_edges: Vec<RuntimeWorkspaceImportEdge>,
  pub diagnostics: Vec<RuntimeWorkspaceDiagnostic>,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceTargetSnapshot {
  pub name: String,
  pub specifier: String,
  pub canonical_uri: String,
  pub module_version: ModuleVersionId,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceSourceSnapshot {
  pub canonical_uri: String,
  pub path: Option<PathBuf>,
  pub module_version: Option<ModuleVersionId>,
  pub content_hash: u64,
  pub modified_time: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceImportEdge {
  pub importer: ModuleVersionId,
  pub dependency: ModuleVersionId,
  pub specifier: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeWorkspaceDiagnosticSeverity {
  Error,
  Warning,
  Info,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceDiagnostic {
  pub severity: RuntimeWorkspaceDiagnosticSeverity,
  pub target: Option<String>,
  pub canonical_uri: Option<String>,
  pub message: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeWorkspaceInvalidConfig {
  pub reason: String,
}

impl MechErrorKind for RuntimeWorkspaceInvalidConfig {
  fn name(&self) -> &str {
    "RuntimeWorkspaceInvalidConfig"
  }

  fn message(&self) -> String {
    format!("invalid runtime workspace config: {}", self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeWorkspaceTargetNotFound {
  pub name: String,
}

impl MechErrorKind for RuntimeWorkspaceTargetNotFound {
  fn name(&self) -> &str {
    "RuntimeWorkspaceTargetNotFound"
  }

  fn message(&self) -> String {
    format!("runtime workspace target `{}` was not found", self.name)
  }
}

impl RuntimeWorkspaceConfig {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self {
      root: root.into(),
      targets: Vec::new(),
    }
  }

  pub fn target(
    mut self,
    name: impl Into<String>,
    specifier: impl Into<String>,
  ) -> Self {
    self.targets.push(RuntimeWorkspaceTarget {
      name: name.into(),
      specifier: specifier.into(),
    });
    self
  }
}

impl RuntimeWorkspace {
  pub fn open(mut config: RuntimeWorkspaceConfig) -> MResult<Self> {
    config.root = canonicalize_workspace_root(&config.root)?;

    let mut target_names = BTreeSet::new();
    for target in &config.targets {
      if target.name.trim().is_empty() {
        return invalid_config("target name must not be empty");
      }
      if target.specifier.trim().is_empty() {
        return invalid_config("target specifier must not be empty");
      }
      if !target_names.insert(target.name.clone()) {
        return invalid_config(format!("duplicate target name `{}`", target.name));
      }
    }

    Ok(Self {
      config,
      snapshot: None,
    })
  }

  pub fn config(&self) -> &RuntimeWorkspaceConfig {
    &self.config
  }

  pub fn snapshot(&self) -> Option<&RuntimeWorkspaceSnapshot> {
    self.snapshot.as_ref()
  }

  pub fn load(
    &mut self,
    runtime: &mut MechRuntime,
    options: ModuleBuildOptions,
  ) -> MResult<RuntimeWorkspaceSnapshot> {
    let mut snapshot = RuntimeWorkspaceSnapshot {
      root: self.config.root.clone(),
      ..RuntimeWorkspaceSnapshot::default()
    };
    let mut loaded_versions = Vec::new();

    for target in &self.config.targets {
      match runtime.resolve_and_store_module_source(target.specifier.as_str(), options.clone()) {
        Ok(Some(module_version)) => {
          let Some((module, _)) = runtime.workspace_module_records(module_version)? else {
            snapshot.diagnostics.push(target_diagnostic(
              target,
              format!("loaded module version `{}` was not found in the runtime store", module_version),
            ));
            continue;
          };
          snapshot.targets.insert(target.name.clone(), RuntimeWorkspaceTargetSnapshot {
            name: target.name.clone(),
            specifier: target.specifier.clone(),
            canonical_uri: module.name,
            module_version,
          });
          loaded_versions.push(module_version);
        }
        Ok(None) => snapshot.diagnostics.push(target_diagnostic(
          target,
          format!("workspace target `{}` could not resolve `{}`", target.name, target.specifier),
        )),
        Err(error) => snapshot.diagnostics.push(target_diagnostic(
          target,
          format!("workspace target `{}` failed to load `{}`: {:?}", target.name, target.specifier, error),
        )),
      }
    }

    collect_loaded_modules(runtime, &loaded_versions, &mut snapshot)?;

    self.snapshot = Some(snapshot.clone());
    Ok(snapshot)
  }

  pub fn target(
    &self,
    name: &str,
  ) -> Option<&RuntimeWorkspaceTargetSnapshot> {
    self.snapshot.as_ref()?.targets.get(name)
  }
}

fn canonicalize_workspace_root(root: &Path) -> MResult<PathBuf> {
  root.canonicalize().map_err(|error| {
    MechError::new(
      RuntimeWorkspaceInvalidConfig {
        reason: format!("workspace root `{}` could not be canonicalized: {}", root.display(), error),
      },
      None,
    )
  })
}

fn invalid_config<T>(reason: impl Into<String>) -> MResult<T> {
  Err(MechError::new(
    RuntimeWorkspaceInvalidConfig {
      reason: reason.into(),
    },
    None,
  ))
}

fn target_diagnostic(
  target: &RuntimeWorkspaceTarget,
  message: String,
) -> RuntimeWorkspaceDiagnostic {
  RuntimeWorkspaceDiagnostic {
    severity: RuntimeWorkspaceDiagnosticSeverity::Error,
    target: Some(target.name.clone()),
    canonical_uri: None,
    message,
  }
}

fn collect_loaded_modules(
  runtime: &MechRuntime,
  loaded_versions: &[ModuleVersionId],
  snapshot: &mut RuntimeWorkspaceSnapshot,
) -> MResult<()> {
  let mut pending = VecDeque::from(loaded_versions.to_vec());
  let mut visited = BTreeSet::new();

  while let Some(module_version) = pending.pop_front() {
    if !visited.insert(module_version) {
      continue;
    }

    let Some((module, version)) = runtime.workspace_module_records(module_version)? else {
      continue;
    };

    snapshot.sources.insert(
      module.name.clone(),
      source_snapshot(module.name, module_version),
    );

    for edge in &version.import_edges {
      snapshot.import_edges.push(RuntimeWorkspaceImportEdge {
        importer: module_version,
        dependency: edge.dependency,
        specifier: edge.import.specifier.clone(),
      });
    }

    pending.extend(version.dependencies);
  }

  Ok(())
}

fn source_snapshot(
  canonical_uri: String,
  module_version: ModuleVersionId,
) -> RuntimeWorkspaceSourceSnapshot {
  let path = file_uri_path(&canonical_uri);
  let content_hash = path
    .as_ref()
    .and_then(|path| std::fs::read(path).ok())
    .map(hash_content)
    .unwrap_or_default();
  let modified_time = path
    .as_ref()
    .and_then(|path| std::fs::metadata(path).ok())
    .and_then(|metadata| metadata.modified().ok());

  RuntimeWorkspaceSourceSnapshot {
    canonical_uri,
    path,
    module_version: Some(module_version),
    content_hash,
    modified_time,
  }
}

fn file_uri_path(canonical_uri: &str) -> Option<PathBuf> {
  let rest = canonical_uri.strip_prefix("file://")?;

  #[cfg(windows)]
  {
    file_uri_path_windows(rest)
  }

  #[cfg(not(windows))]
  {
    file_uri_path_unix(rest)
  }
}

#[cfg(not(windows))]
fn file_uri_path_unix(rest: &str) -> Option<PathBuf> {
  if rest.is_empty() {
    return None;
  }
  Some(PathBuf::from(rest))
}

#[cfg(windows)]
fn file_uri_path_windows(rest: &str) -> Option<PathBuf> {
  if rest.is_empty() {
    return None;
  }

  if let Some(path) = rest.strip_prefix("//?/") {
    return Some(PathBuf::from(format!(r"\\?\{}", path.replace('/', r"\"))));
  }

  if rest.len() >= 3
    && rest.as_bytes()[0] == b'/'
    && rest.as_bytes()[2] == b':'
  {
    return Some(PathBuf::from(rest[1..].replace('/', r"\")));
  }

  Some(PathBuf::from(rest.replace('/', r"\")))
}

fn hash_content(content: Vec<u8>) -> u64 {
  let mut hasher = DefaultHasher::new();
  content.hash(&mut hasher);
  hasher.finish()
}


#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(windows))]
  #[test]
  fn file_uri_path_converts_unix_file_uri() {
    assert_eq!(
      file_uri_path("file:///tmp/project/main.mec").unwrap(),
      PathBuf::from("/tmp/project/main.mec"),
    );
  }

  #[cfg(windows)]
  #[test]
  fn file_uri_path_converts_windows_drive_file_uri() {
    assert_eq!(
      file_uri_path("file:///C:/Users/cmont/project/main.mec").unwrap(),
      PathBuf::from(r"C:\Users\cmont\project\main.mec"),
    );
  }

  #[cfg(windows)]
  #[test]
  fn file_uri_path_converts_windows_extended_file_uri() {
    assert_eq!(
      file_uri_path("file:////?/C:/Users/cmont/project/main.mec").unwrap(),
      PathBuf::from(r"\\?\C:\Users\cmont\project\main.mec"),
    );
  }

  #[test]
  fn file_uri_path_rejects_non_file_uri() {
    assert!(file_uri_path("http://example.com/main.mec").is_none());
  }
}

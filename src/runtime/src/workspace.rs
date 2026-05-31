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
pub struct RuntimeWorkspaceRefresh {
  pub snapshot: RuntimeWorkspaceSnapshot,
  pub changes: Vec<RuntimeWorkspaceChange>,
  pub affected_targets: Vec<String>,
  pub diagnostics: Vec<RuntimeWorkspaceDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeWorkspaceChangeKind {
  Modified,
  Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceChange {
  pub canonical_uri: String,
  pub path: Option<PathBuf>,
  pub kind: RuntimeWorkspaceChangeKind,
  pub previous_hash: Option<u64>,
  pub current_hash: Option<u64>,
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
pub struct RuntimeWorkspaceNotLoaded;

impl MechErrorKind for RuntimeWorkspaceNotLoaded {
  fn name(&self) -> &str {
    "RuntimeWorkspaceNotLoaded"
  }

  fn message(&self) -> String {
    "runtime workspace must be loaded before it can be refreshed".to_string()
  }
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
    let mut targets = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for target in &self.config.targets {
      match load_target(&self.config.root, runtime, target, options.clone())? {
        Ok(snapshot) => {
          targets.insert(target.name.clone(), snapshot);
        }
        Err(diagnostic) => diagnostics.push(diagnostic),
      }
    }

    let snapshot = collect_snapshot(runtime, self.config.root.clone(), targets, diagnostics)?;
    self.snapshot = Some(snapshot.clone());
    Ok(snapshot)
  }

  pub fn refresh(
    &mut self,
    runtime: &mut MechRuntime,
    options: ModuleBuildOptions,
  ) -> MResult<RuntimeWorkspaceRefresh> {
    let Some(previous) = self.snapshot.clone() else {
      return Err(MechError::new(RuntimeWorkspaceNotLoaded, None));
    };
    let changes = changed_sources(&previous);
    if changes.is_empty() {
      return Ok(RuntimeWorkspaceRefresh {
        snapshot: previous,
        changes,
        affected_targets: Vec::new(),
        diagnostics: Vec::new(),
      });
    }

    let affected_targets = affected_targets(&previous, &changes);
    let affected = affected_targets.iter().cloned().collect::<BTreeSet<_>>();
    let mut targets = previous
      .targets
      .into_iter()
      .filter(|(name, _)| !affected.contains(name))
      .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Vec::new();

    for target in self
      .config
      .targets
      .iter()
      .filter(|target| affected.contains(&target.name))
    {
      match load_target(&self.config.root, runtime, target, options.clone())? {
        Ok(snapshot) => {
          targets.insert(target.name.clone(), snapshot);
        }
        Err(diagnostic) => diagnostics.push(diagnostic),
      }
    }

    let snapshot = collect_snapshot(
      runtime,
      self.config.root.clone(),
      targets,
      diagnostics.clone(),
    )?;
    self.snapshot = Some(snapshot.clone());
    Ok(RuntimeWorkspaceRefresh {
      snapshot,
      changes,
      affected_targets,
      diagnostics,
    })
  }

  pub fn target(
    &self,
    name: &str,
  ) -> Option<&RuntimeWorkspaceTargetSnapshot> {
    self.snapshot.as_ref()?.targets.get(name)
  }
}

fn load_target(
  root: &Path,
  runtime: &mut MechRuntime,
  target: &RuntimeWorkspaceTarget,
  options: ModuleBuildOptions,
) -> MResult<Result<RuntimeWorkspaceTargetSnapshot, RuntimeWorkspaceDiagnostic>> {
  let resolved_specifier = match workspace_target_specifier(root, &target.specifier) {
    Ok(specifier) => specifier,
    Err(error) => return Ok(Err(target_diagnostic(
      target,
      format!(
        "workspace target `{}` failed to resolve `{}` against root `{}`: {:?}",
        target.name,
        target.specifier,
        root.display(),
        error,
      ),
    ))),
  };

  let module_version = match runtime.resolve_and_store_module_source(resolved_specifier.as_str(), options) {
    Ok(Some(module_version)) => module_version,
    Ok(None) => return Ok(Err(target_diagnostic(
      target,
      format!("workspace target `{}` could not resolve `{}`", target.name, target.specifier),
    ))),
    Err(error) => return Ok(Err(target_diagnostic(
      target,
      format!("workspace target `{}` failed to load `{}`: {:?}", target.name, target.specifier, error),
    ))),
  };
  let Some((module, _)) = runtime.workspace_module_records(module_version)? else {
    return Ok(Err(target_diagnostic(
      target,
      format!("loaded module version `{}` was not found in the runtime store", module_version),
    )));
  };

  Ok(Ok(RuntimeWorkspaceTargetSnapshot {
    name: target.name.clone(),
    specifier: target.specifier.clone(),
    canonical_uri: module.name,
    module_version,
  }))
}

fn collect_snapshot(
  runtime: &MechRuntime,
  root: PathBuf,
  targets: BTreeMap<String, RuntimeWorkspaceTargetSnapshot>,
  diagnostics: Vec<RuntimeWorkspaceDiagnostic>,
) -> MResult<RuntimeWorkspaceSnapshot> {
  let loaded_versions = targets
    .values()
    .map(|target| target.module_version)
    .collect::<Vec<_>>();
  let mut snapshot = RuntimeWorkspaceSnapshot {
    root,
    targets,
    diagnostics,
    ..RuntimeWorkspaceSnapshot::default()
  };
  collect_loaded_modules(runtime, &loaded_versions, &mut snapshot)?;
  Ok(snapshot)
}

fn changed_sources(snapshot: &RuntimeWorkspaceSnapshot) -> Vec<RuntimeWorkspaceChange> {
  snapshot
    .sources
    .values()
    .filter_map(|source| {
      let path = source.path.as_ref()?;
      if !path.exists() {
        return Some(RuntimeWorkspaceChange {
          canonical_uri: source.canonical_uri.clone(),
          path: Some(path.clone()),
          kind: RuntimeWorkspaceChangeKind::Removed,
          previous_hash: Some(source.content_hash),
          current_hash: None,
        });
      }

      let current_hash = std::fs::read(path).ok().map(hash_content)?;
      if current_hash == source.content_hash {
        return None;
      }
      Some(RuntimeWorkspaceChange {
        canonical_uri: source.canonical_uri.clone(),
        path: Some(path.clone()),
        kind: RuntimeWorkspaceChangeKind::Modified,
        previous_hash: Some(source.content_hash),
        current_hash: Some(current_hash),
      })
    })
    .collect()
}

fn affected_targets(
  snapshot: &RuntimeWorkspaceSnapshot,
  changes: &[RuntimeWorkspaceChange],
) -> Vec<String> {
  let changed_uris = changes
    .iter()
    .map(|change| change.canonical_uri.as_str())
    .collect::<BTreeSet<_>>();
  let changed_versions = snapshot
    .sources
    .values()
    .filter(|source| changed_uris.contains(source.canonical_uri.as_str()))
    .filter_map(|source| source.module_version)
    .collect::<BTreeSet<_>>();

  snapshot
    .targets
    .values()
    .filter_map(|target| {
      if changed_uris.contains(target.canonical_uri.as_str())
        || import_closure_contains(snapshot, target.module_version, &changed_versions)
      {
        Some(target.name.clone())
      } else {
        None
      }
    })
    .collect()
}

fn import_closure_contains(
  snapshot: &RuntimeWorkspaceSnapshot,
  root: ModuleVersionId,
  versions: &BTreeSet<ModuleVersionId>,
) -> bool {
  let mut pending = VecDeque::from([root]);
  let mut visited = BTreeSet::new();
  while let Some(module_version) = pending.pop_front() {
    if !visited.insert(module_version) {
      continue;
    }
    if versions.contains(&module_version) {
      return true;
    }
    pending.extend(
      snapshot
        .import_edges
        .iter()
        .filter(|edge| edge.importer == module_version)
        .map(|edge| edge.dependency),
    );
  }
  false
}

fn workspace_target_specifier(root: &Path, specifier: &str) -> MResult<String> {
  if specifier.contains("://") {
    return Ok(specifier.to_string());
  }

  let path = PathBuf::from(specifier);
  let joined = if path.is_absolute() {
    path
  } else {
    root.join(path)
  };

  let canonical = joined.canonicalize().map_err(|error| {
    MechError::new(
      RuntimeWorkspaceInvalidConfig {
        reason: format!(
          "workspace target `{}` could not be canonicalized against root `{}`: {}",
          specifier,
          root.display(),
          error,
        ),
      },
      None,
    )
  })?;

  if !canonical.starts_with(root) {
    return Err(MechError::new(
      RuntimeWorkspaceInvalidConfig {
        reason: format!(
          "workspace target `{}` resolves outside workspace root `{}`",
          specifier,
          root.display(),
        ),
      },
      None,
    ));
  }

  Ok(canonical.to_string_lossy().to_string())
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

  #[test]
  fn workspace_target_specifier_canonicalizes_absolute_local_path() {
    let root = std::env::temp_dir().join(format!(
      "mech-runtime-workspace-absolute-target-{}",
      SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("main.mec");
    std::fs::write(&target, "result := true\n").unwrap();
    let canonical_root = root.canonicalize().unwrap();
    let canonical_target = target.canonicalize().unwrap();

    assert_eq!(
      workspace_target_specifier(&canonical_root, target.to_string_lossy().as_ref()).unwrap(),
      canonical_target.to_string_lossy(),
    );
  }

  #[test]
  fn workspace_target_specifier_passes_through_uri() {
    assert_eq!(
      workspace_target_specifier(Path::new("unused"), "memory://main.mec").unwrap(),
      "memory://main.mec",
    );
  }
}

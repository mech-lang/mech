use super::*;

pub(super) fn load_target(
  root: &Path,
  runtime: &mut MechRuntime,
  target: &RuntimeWorkspaceTarget,
  options: ModuleBuildOptions,
  capability_subject: Option<&str>,
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

  if let Some(subject) = capability_subject {
    let local_path = Path::new(&resolved_specifier);
    if !resolved_specifier.contains("://") {
      for operation in [crate::FS_RESOLVE, crate::FS_READ] {
        if let Err(error) = crate::check_fs_capability(runtime.capability_kernel_mut(), subject, operation, local_path) {
          return Ok(Err(target_diagnostic(target, format!("{:?}", error))));
        }
      }
    }
  }

  let module_version = match runtime.resolve_and_store_module_source(
    resolved_specifier.as_str(),
    options,
  ) {
    Ok(Some(module_version)) => module_version,
    Ok(None) => return Ok(Err(target_diagnostic(
      target,
      format!(
        "workspace target `{}` could not resolve `{}`",
        target.name,
        target.specifier,
      ),
    ))),
    Err(error) => return Ok(Err(target_diagnostic(
      target,
      format!(
        "workspace target `{}` failed to load `{}`: {:?}",
        target.name,
        target.specifier,
        error,
      ),
    ))),
  };

  let Some((module, _)) = runtime.workspace_module_records(module_version)? else {
    return Ok(Err(target_diagnostic(
      target,
      format!(
        "loaded module version `{}` was not found in the runtime store",
        module_version,
      ),
    )));
  };

  Ok(Ok(RuntimeWorkspaceTargetSnapshot {
    name: target.name.clone(),
    specifier: target.specifier.clone(),
    canonical_uri: module.name,
    module_version,
  }))
}

pub(super) fn collect_snapshot(
  runtime: &MechRuntime,
  root: PathBuf,
  targets: BTreeMap<String, RuntimeWorkspaceTargetSnapshot>,
  extra_module_versions: Vec<ModuleVersionId>,
  diagnostics: Vec<RuntimeWorkspaceDiagnostic>,
) -> MResult<RuntimeWorkspaceSnapshot> {
  let mut loaded_versions = targets
    .values()
    .map(|target| target.module_version)
    .collect::<Vec<_>>();

  loaded_versions.extend(extra_module_versions);
  
  let mut snapshot = RuntimeWorkspaceSnapshot {
    root,
    targets,
    diagnostics,
    ..RuntimeWorkspaceSnapshot::default()
  };

  collect_loaded_modules(runtime, &loaded_versions, &mut snapshot)?;

  Ok(snapshot)
}

pub(super) fn workspace_target_specifier(
  root: &Path,
  specifier: &str,
) -> MResult<String> {
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

pub(super) fn file_uri_path(canonical_uri: &str) -> Option<PathBuf> {
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
pub(super) fn file_uri_path_unix(rest: &str) -> Option<PathBuf> {
  if rest.is_empty() {
    return None;
  }
  Some(PathBuf::from(rest))
}

#[cfg(windows)]
pub(super) fn file_uri_path_windows(rest: &str) -> Option<PathBuf> {
  if rest.is_empty() {
    return None;
  }

  if let Some(path) = rest.strip_prefix("//?/") {
    return Some(PathBuf::from(format!(
      r"\\?\{}",
      path.replace('/', r"\"),
    )));
  }

  if rest.len() >= 3
    && rest.as_bytes()[0] == b'/'
    && rest.as_bytes()[2] == b':'
  {
    return Some(PathBuf::from(rest[1..].replace('/', r"\")));
  }

  Some(PathBuf::from(rest.replace('/', r"\")))
}
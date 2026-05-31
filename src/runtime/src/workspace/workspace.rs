use super::*;

#[derive(Clone, Debug)]
pub struct RuntimeWorkspace {
  config: RuntimeWorkspaceConfig,
  snapshot: Option<RuntimeWorkspaceSnapshot>,
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

    for folder in &config.folders {
      if folder.specifier.trim().is_empty() {
        return invalid_config("workspace folder specifier must not be empty");
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

    let mut extra_module_versions = Vec::new();

    for file in discover_workspace_files(&self.config.root, &self.config.folders)? {
      match runtime.resolve_and_store_module_source(
        file.specifier.as_str(),
        options.clone(),
      ) {
        Ok(Some(module_version)) => {
          extra_module_versions.push(module_version);
        }
        Ok(None) => diagnostics.push(RuntimeWorkspaceDiagnostic {
          severity: RuntimeWorkspaceDiagnosticSeverity::Error,
          target: None,
          canonical_uri: Some(file.canonical_path.to_string_lossy().to_string()),
          message: format!(
            "workspace discovered file `{}` could not resolve",
            file.canonical_path.display(),
          ),
        }),
        Err(error) => diagnostics.push(RuntimeWorkspaceDiagnostic {
          severity: RuntimeWorkspaceDiagnosticSeverity::Error,
          target: None,
          canonical_uri: Some(file.canonical_path.to_string_lossy().to_string()),
          message: format!(
            "workspace discovered file `{}` failed to load: {:?}",
            file.canonical_path.display(),
            error,
          ),
        }),
      }
    }    

    let snapshot = collect_snapshot(
      runtime,
      self.config.root.clone(),
      targets,
      extra_module_versions,
      diagnostics,
    )?;
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

    let changes = previous.changed_sources();
    if changes.is_empty() {
      return Ok(RuntimeWorkspaceRefresh {
        snapshot: previous,
        changes,
        affected_targets: Vec::new(),
        refresh_diagnostics: Vec::new(),
      });
    }

    let affected_targets = previous.affected_targets(&changes);
    let affected = affected_targets
      .iter()
      .cloned()
      .collect::<BTreeSet<_>>();

    let retained_diagnostics = previous
      .diagnostics
      .iter()
      .filter(|diagnostic| match diagnostic.target.as_deref() {
        Some(target) => !affected.contains(target),
        None => true,
      })
      .cloned()
      .collect::<Vec<_>>();

    let mut targets = previous
      .targets
      .iter()
      .filter(|(name, _)| !affected.contains(*name))
      .map(|(name, target)| (name.clone(), target.clone()))
      .collect::<BTreeMap<_, _>>();

    let mut refresh_diagnostics = Vec::new();

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
        Err(diagnostic) => refresh_diagnostics.push(diagnostic),
      }
    }

    let mut extra_module_versions = Vec::new();

    for file in discover_workspace_files(&self.config.root, &self.config.folders)? {
      match runtime.resolve_and_store_module_source(
        file.specifier.as_str(),
        options.clone(),
      ) {
        Ok(Some(module_version)) => {
          extra_module_versions.push(module_version);
        }
        Ok(None) => refresh_diagnostics.push(RuntimeWorkspaceDiagnostic {
          severity: RuntimeWorkspaceDiagnosticSeverity::Error,
          target: None,
          canonical_uri: Some(file.canonical_path.to_string_lossy().to_string()),
          message: format!(
            "workspace discovered file `{}` could not resolve",
            file.canonical_path.display(),
          ),
        }),
        Err(error) => refresh_diagnostics.push(RuntimeWorkspaceDiagnostic {
          severity: RuntimeWorkspaceDiagnosticSeverity::Error,
          target: None,
          canonical_uri: Some(file.canonical_path.to_string_lossy().to_string()),
          message: format!(
            "workspace discovered file `{}` failed to load: {:?}",
            file.canonical_path.display(),
            error,
          ),
        }),
      }
    }    

    let mut snapshot_diagnostics = retained_diagnostics;
    snapshot_diagnostics.extend(refresh_diagnostics.clone());

    let snapshot = collect_snapshot(
      runtime,
      self.config.root.clone(),
      targets,
      extra_module_versions,
      snapshot_diagnostics,
    )?;

    self.snapshot = Some(snapshot.clone());

    Ok(RuntimeWorkspaceRefresh {
      snapshot,
      changes,
      affected_targets,
      refresh_diagnostics,
    })
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
        reason: format!(
          "workspace root `{}` could not be canonicalized: {}",
          root.display(),
          error,
        ),
      },
      None,
    )
  })
}
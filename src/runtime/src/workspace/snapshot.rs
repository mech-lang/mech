use super::*;

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

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceDiagnostic {
  pub severity: RuntimeWorkspaceDiagnosticSeverity,
  pub target: Option<String>,
  pub canonical_uri: Option<String>,
  pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeWorkspaceSnapshot {
  pub root: PathBuf,
  pub targets: BTreeMap<String, RuntimeWorkspaceTargetSnapshot>,
  pub sources: BTreeMap<String, RuntimeWorkspaceSourceSnapshot>,
  pub import_edges: Vec<RuntimeWorkspaceImportEdge>,
  pub diagnostics: Vec<RuntimeWorkspaceDiagnostic>,
}

impl RuntimeWorkspaceSnapshot {

  pub fn changed_sources(&self) -> Vec<RuntimeWorkspaceChange> {
    self
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

  pub fn affected_targets(&self, changes: &[RuntimeWorkspaceChange]) -> Vec<String> {
    let changed_uris = changes
      .iter()
      .map(|change| change.canonical_uri.as_str())
      .collect::<BTreeSet<_>>();

    let changed_versions = self
      .sources
      .values()
      .filter(|source| changed_uris.contains(source.canonical_uri.as_str()))
      .filter_map(|source| source.module_version)
      .collect::<BTreeSet<_>>();

    self
      .targets
      .values()
      .filter_map(|target| {
        if changed_uris.contains(target.canonical_uri.as_str())
          || self.import_closure_contains(target.module_version, &changed_versions)
        {
          Some(target.name.clone())
        } else {
          None
        }
      })
      .collect()
  }

  pub fn import_closure_contains(
    &self,
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
        self
          .import_edges
          .iter()
          .filter(|edge| edge.importer == module_version)
          .map(|edge| edge.dependency),
      );
    }
    false
  }

}

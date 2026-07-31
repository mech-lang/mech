use super::*;

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceRefresh {
  pub snapshot: RuntimeWorkspaceSnapshot,
  pub changes: Vec<RuntimeWorkspaceChange>,
  pub affected_targets: Vec<String>,
  pub refresh_diagnostics: Vec<RuntimeWorkspaceDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceChange {
  pub canonical_uri: String,
  pub path: Option<PathBuf>,
  pub kind: RuntimeWorkspaceChangeKind,
  pub previous_hash: Option<u64>,
  pub current_hash: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeWorkspaceChangeKind {
  Added,
  Modified,
  Removed,
}
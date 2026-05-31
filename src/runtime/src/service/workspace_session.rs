use std::path::PathBuf;

use mech_core::MResult;

use crate::{
  FileSourceResolver, MechRuntime, ModuleBuildOptions, RuntimeBuilder,
  RuntimeWorkspace, RuntimeWorkspaceConfig, RuntimeWorkspaceFolder,
  RuntimeWorkspaceSnapshot, RuntimeWorkspaceTarget, RuntimeWorkspaceWatcher,
  RuntimeWorkspaceWatchPoll,
};

pub struct ServerWorkspaceSession {
  runtime: MechRuntime,
  workspace: RuntimeWorkspace,
  watcher: RuntimeWorkspaceWatcher,
}

impl ServerWorkspaceSession {
  pub fn open(
    root: impl Into<PathBuf>,
    targets: Vec<RuntimeWorkspaceTarget>,
    folders: Vec<RuntimeWorkspaceFolder>,
    options: ModuleBuildOptions,
  ) -> MResult<Self> {
    let root = root.into();
    let mut runtime = RuntimeBuilder::new()
      .source_resolver(FileSourceResolver::new(&root))
      .build()?;
    let mut config = RuntimeWorkspaceConfig::new(&root);

    for target in targets {
      config = config.target(target.name, target.specifier);
    }

    for folder in folders {
      config = config.folder_recursive(folder.specifier, folder.recursive);
    }

    let mut workspace = RuntimeWorkspace::open(config)?;
    workspace.load(&mut runtime, options.clone())?;
    let watcher = RuntimeWorkspaceWatcher::open(&workspace)?;

    Ok(Self {
      runtime,
      workspace,
      watcher,
    })
  }

  pub fn runtime(&self) -> &MechRuntime {
    &self.runtime
  }

  pub fn runtime_mut(&mut self) -> &mut MechRuntime {
    &mut self.runtime
  }

  pub fn workspace(&self) -> &RuntimeWorkspace {
    &self.workspace
  }

  pub fn workspace_mut(&mut self) -> &mut RuntimeWorkspace {
    &mut self.workspace
  }

  pub fn watcher(&self) -> &RuntimeWorkspaceWatcher {
    &self.watcher
  }

  pub fn snapshot(&self) -> Option<&RuntimeWorkspaceSnapshot> {
    self.workspace.snapshot()
  }

  pub fn poll(
    &mut self,
    options: ModuleBuildOptions,
  ) -> MResult<RuntimeWorkspaceWatchPoll> {
    self.watcher.poll(
      &mut self.workspace,
      &mut self.runtime,
      options,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn setup_session_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-server-workspace-session-{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
  }

  fn main_target() -> RuntimeWorkspaceTarget {
    RuntimeWorkspaceTarget {
      name: "main".to_string(),
      specifier: "main.mec".to_string(),
    }
  }

  fn recursive_root_folder() -> RuntimeWorkspaceFolder {
    RuntimeWorkspaceFolder {
      specifier: ".".to_string(),
      recursive: true,
    }
  }

  fn module_options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new("test", "v0.3", "native", &[], &[])
  }

  #[test]
  fn server_workspace_session_open_loads_snapshot() {
    let root = setup_session_root();
    std::fs::write(root.join("main.mec"), "result := true\n").unwrap();

    let session = ServerWorkspaceSession::open(
      &root,
      vec![main_target()],
      vec![recursive_root_folder()],
      module_options(),
    ).unwrap();

    assert!(session.snapshot().is_some());
    let snapshot = session.snapshot().unwrap();
    let main_path = root.join("main.mec").canonicalize().unwrap();

    assert!(snapshot.diagnostics.is_empty());
    assert!(snapshot.targets.contains_key("main"));
    assert!(snapshot.sources.values().any(|source| {
      source.path.as_ref() == Some(&main_path)
    }));
  }

  #[test]
  fn server_workspace_session_open_preserves_initial_diagnostics() {
    let root = setup_session_root();

    let session = ServerWorkspaceSession::open(
      &root,
      vec![RuntimeWorkspaceTarget {
        name: "missing".to_string(),
        specifier: "missing.mec".to_string(),
      }],
      vec![recursive_root_folder()],
      module_options(),
    ).unwrap();

    let diagnostics = &session.snapshot().unwrap().diagnostics;

    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics[0].target.as_deref(), Some("missing"));
  }

  #[test]
  fn server_workspace_session_poll_without_events_is_empty() {
    let root = setup_session_root();
    std::fs::write(root.join("main.mec"), "result := true\n").unwrap();

    let mut session = ServerWorkspaceSession::open(
      &root,
      vec![main_target()],
      vec![recursive_root_folder()],
      module_options(),
    ).unwrap();

    let poll = session.poll(module_options()).unwrap();

    assert!(poll.events.is_empty());
    assert!(poll.refresh.is_none());
  }
}

use std::{
  collections::BTreeSet,
  path::{Path, PathBuf},
  sync::mpsc::{self, Receiver},
};

use notify::{
  Event,
  EventKind,
  RecommendedWatcher,
  RecursiveMode,
  Watcher,
};

use super::*;

pub struct RuntimeWorkspaceWatcher {
  watcher: RecommendedWatcher,
  receiver: Receiver<notify::Result<Event>>,
  watched_paths: BTreeSet<RuntimeWorkspaceWatchedPath>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeWorkspaceWatchedPath {
  path: PathBuf,
  recursive: bool,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceWatchPoll {
  pub events: Vec<RuntimeWorkspaceWatchEvent>,
  pub refresh: Option<RuntimeWorkspaceRefresh>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceWatchEvent {
  pub path: PathBuf,
  pub kind: RuntimeWorkspaceWatchEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeWorkspaceWatchEventKind {
  Created,
  Modified,
  Removed,
  Other,
}

impl RuntimeWorkspaceWatcher {
  pub fn open(workspace: &RuntimeWorkspace, runtime: &mut MechRuntime) -> MResult<Self> {
    let (sender, receiver) = mpsc::channel();

    let watcher = notify::recommended_watcher(move |event| {
      let _ = sender.send(event);
    }).map_err(|error| {
      watch_error(format!("could not create watcher: {}", error))
    })?;

    let mut watcher = Self {
      watcher,
      receiver,
      watched_paths: BTreeSet::new(),
    };

    watcher.sync(workspace, runtime)?;

    Ok(watcher)
  }

  pub fn poll(
    &mut self,
    workspace: &mut RuntimeWorkspace,
    runtime: &mut MechRuntime,
    options: ModuleBuildOptions,
  ) -> MResult<RuntimeWorkspaceWatchPoll> {
    let events = self.drain_events()?;

    if events.is_empty() {
      return Ok(RuntimeWorkspaceWatchPoll {
        events,
        refresh: None,
      });
    }

    let refresh = workspace.refresh(runtime, options)?;
    self.sync(workspace, runtime)?;

    Ok(RuntimeWorkspaceWatchPoll {
      events,
      refresh: Some(refresh),
    })
  }

  pub fn sync(&mut self, workspace: &RuntimeWorkspace, runtime: &mut MechRuntime) -> MResult<()> {
    let desired = desired_watch_paths(workspace);

    for watched in self.watched_paths.clone() {
      if !desired.contains(&watched) {
        self.watcher.unwatch(&watched.path).map_err(|error| {
          watch_error(format!(
            "could not unwatch `{}`: {}",
            watched.path.display(),
            error,
          ))
        })?;
        self.watched_paths.remove(&watched);
      }
    }

    for watched in desired {
      if self.watched_paths.contains(&watched) {
        continue;
      }

      if let Some(subject) = workspace.config().capability_subject.as_deref() {
        crate::check_fs_capability(runtime.capability_kernel_mut(), subject, crate::FS_WATCH, &watched.path)?;
      }

      let recursive_mode = if watched.recursive {
        RecursiveMode::Recursive
      } else {
        RecursiveMode::NonRecursive
      };

      self.watcher.watch(&watched.path, recursive_mode).map_err(|error| {
        watch_error(format!(
          "could not watch `{}`: {}",
          watched.path.display(),
          error,
        ))
      })?;

      self.watched_paths.insert(watched);
    }

    Ok(())
  }

  pub fn drain_events(&mut self) -> MResult<Vec<RuntimeWorkspaceWatchEvent>> {
    let mut events = Vec::new();

    while let Ok(event) = self.receiver.try_recv() {
      let event = event.map_err(|error| {
        watch_error(format!("watch event error: {}", error))
      })?;

      events.extend(watch_events_from_notify_event(event));
    }

    Ok(events)
  }

  pub fn watched_paths(&self) -> Vec<PathBuf> {
    self.watched_paths
      .iter()
      .map(|watched| watched.path.clone())
      .collect()
  }

  #[cfg(test)]
  fn from_parts(
    watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<Event>>,
  ) -> Self {
    Self {
      watcher,
      receiver,
      watched_paths: BTreeSet::new(),
    }
  }

}

fn desired_watch_paths(
  workspace: &RuntimeWorkspace,
) -> BTreeSet<RuntimeWorkspaceWatchedPath> {
  let mut paths = BTreeSet::new();

  for folder in &workspace.config().folders {
    if let Some(path) = local_workspace_path(
      &workspace.config().root,
      &folder.specifier,
    ) {
      paths.insert(RuntimeWorkspaceWatchedPath {
        path,
        recursive: folder.recursive,
      });
    }
  }

  for target in &workspace.config().targets {
    if let Some(path) = local_workspace_path(&workspace.config().root, &target.specifier) {
      paths.insert(RuntimeWorkspaceWatchedPath {
        path,
        recursive: false,
      });
    }
  }

  paths
}

fn local_workspace_path(
  root: &Path,
  specifier: &str,
) -> Option<PathBuf> {
  if specifier.contains("://") {
    return None;
  }

  let path = PathBuf::from(specifier);
  let joined = if path.is_absolute() {
    path
  } else {
    root.join(path)
  };

  joined.canonicalize().ok()
}

fn watch_events_from_notify_event(event: Event) -> Vec<RuntimeWorkspaceWatchEvent> {
  let kind = watch_event_kind(&event.kind);

  event
    .paths
    .into_iter()
    .filter(|path| path.is_file() || !path.exists())
    .map(|path| RuntimeWorkspaceWatchEvent {
      path,
      kind: kind.clone(),
    })
    .collect()
}

fn watch_event_kind(kind: &EventKind) -> RuntimeWorkspaceWatchEventKind {
  match kind {
    EventKind::Create(_) => RuntimeWorkspaceWatchEventKind::Created,
    EventKind::Modify(_) => RuntimeWorkspaceWatchEventKind::Modified,
    EventKind::Remove(_) => RuntimeWorkspaceWatchEventKind::Removed,
    _ => RuntimeWorkspaceWatchEventKind::Other,
  }
}

fn watch_error(reason: impl Into<String>) -> MechError {
  MechError::new(
    RuntimeWorkspaceWatchError {
      reason: reason.into(),
    },
    None,
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::RuntimeBuilder;

  #[test]
  fn watcher_rejects_path_without_watch_capability() {
    let root = std::env::temp_dir().join(format!("mech-watch-capability-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())); std::fs::create_dir_all(&root).unwrap();
    let workspace = RuntimeWorkspace::open(RuntimeWorkspaceConfig::new(&root).capability_subject(crate::SERVE_HOST_SUBJECT).folder(".")).unwrap(); let mut runtime = RuntimeBuilder::new().capability_kernel(crate::SharedCapabilityKernel::new()).build().unwrap();
    assert!(RuntimeWorkspaceWatcher::open(&workspace, &mut runtime).is_err()); std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn watch_events_include_non_mec_paths() {
    let event = Event {
      kind: EventKind::Modify(notify::event::ModifyKind::Any),
      paths: vec![
        PathBuf::from("src/main.mec"),
        PathBuf::from("src/readme.txt"),
      ],
      attrs: Default::default(),
    };

    let events = watch_events_from_notify_event(event);

    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|event| {
      event.path == PathBuf::from("src/main.mec")
        && event.kind == RuntimeWorkspaceWatchEventKind::Modified
    }));
    assert!(events.iter().any(|event| {
      event.path == PathBuf::from("src/readme.txt")
        && event.kind == RuntimeWorkspaceWatchEventKind::Modified
    }));
  }

  #[test]
  fn watch_events_from_notify_event_maps_multiple_mec_paths() {
    let event = Event {
      kind: EventKind::Create(notify::event::CreateKind::File),
      paths: vec![
        PathBuf::from("src/a.mec"),
        PathBuf::from("src/b.mec"),
      ],
      attrs: Default::default(),
    };

    let events = watch_events_from_notify_event(event);

    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|event| {
      event.path == PathBuf::from("src/a.mec")
        && event.kind == RuntimeWorkspaceWatchEventKind::Created
    }));
    assert!(events.iter().any(|event| {
      event.path == PathBuf::from("src/b.mec")
        && event.kind == RuntimeWorkspaceWatchEventKind::Created
    }));
  }

  #[test]
  fn watch_event_kind_maps_create_modify_remove() {
    assert_eq!(
      watch_event_kind(&EventKind::Create(notify::event::CreateKind::Any)),
      RuntimeWorkspaceWatchEventKind::Created,
    );
    assert_eq!(
      watch_event_kind(&EventKind::Modify(notify::event::ModifyKind::Any)),
      RuntimeWorkspaceWatchEventKind::Modified,
    );
    assert_eq!(
      watch_event_kind(&EventKind::Remove(notify::event::RemoveKind::Any)),
      RuntimeWorkspaceWatchEventKind::Removed,
    );
  }
}
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
  watch_path: PathBuf,
  authorized_path: PathBuf,
  filter_paths: Vec<PathBuf>,
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
        self.watcher.unwatch(&watched.watch_path).map_err(|error| {
          watch_error(format!(
            "could not unwatch `{}`: {}",
            watched.watch_path.display(),
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
        crate::check_fs_capability(runtime.capability_kernel_mut(), subject, crate::FS_WATCH, &watched.authorized_path)?;
      }

      let recursive_mode = if watched.recursive {
        RecursiveMode::Recursive
      } else {
        RecursiveMode::NonRecursive
      };

      self.watcher.watch(&watched.watch_path, recursive_mode).map_err(|error| {
        watch_error(format!(
          "could not watch `{}`: {}",
          watched.watch_path.display(),
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

      events.extend(
        watch_events_from_notify_event(event)
          .into_iter()
          .filter(|event| event_allowed_by_watches(&self.watched_paths, &event.path))
      );
    }

    Ok(events)
  }

  pub fn watched_paths(&self) -> Vec<PathBuf> {
    self.watched_paths
      .iter()
      .map(|watched| watched.watch_path.clone())
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
        watch_path: path.clone(),
        authorized_path: path,
        filter_paths: Vec::new(),
        recursive: folder.recursive,
      });
    }
  }

  for target in &workspace.config().targets {
    if let Some(watch) = local_workspace_target_watch(&workspace.config().root, &target.specifier) {
      paths.insert(watch);
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

fn local_workspace_target_watch(
  root: &Path,
  specifier: &str,
) -> Option<RuntimeWorkspaceWatchedPath> {
  if specifier.contains("://") {
    return None;
  }
  let path = PathBuf::from(specifier);
  let joined = if path.is_absolute() { path.clone() } else { root.join(&path) };
  if joined.exists() && joined.is_dir() {
    let path = joined.canonicalize().ok()?;
    return Some(RuntimeWorkspaceWatchedPath {
      watch_path: path.clone(),
      authorized_path: path,
      filter_paths: Vec::new(),
      recursive: false,
    });
  }
  let filter_path = if joined.is_absolute() {
    joined.clone()
  } else {
    root.join(&path)
  };
  let authorized_path = if joined.exists() {
    joined.canonicalize().ok()?
  } else {
    filter_path.clone()
  };
  let parent = filter_path.parent()?.canonicalize().ok()?;
  let mut filter_paths = vec![filter_path.clone()];
  if let Ok(canonical) = filter_path.canonicalize() {
    if canonical != filter_path {
      filter_paths.push(canonical);
    }
  }
  Some(RuntimeWorkspaceWatchedPath {
    watch_path: parent,
    authorized_path,
    filter_paths,
    recursive: false,
  })
}

fn normalize_watch_event_path(path: &Path) -> PathBuf {
  if path.exists() {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
  } else {
    path.to_path_buf()
  }
}

fn event_allowed_by_watches(
  watches: &BTreeSet<RuntimeWorkspaceWatchedPath>,
  path: &Path,
) -> bool {
  let normalized = normalize_watch_event_path(path);
  watches.iter().any(|watch| {
    if !watch.filter_paths.is_empty() {
      return watch.filter_paths.iter().any(|target| {
        path == target || normalized == normalize_watch_event_path(target)
      });
    }
    let watch_path = normalize_watch_event_path(&watch.watch_path);
    if watch.recursive {
      normalized.starts_with(&watch_path)
    } else {
      normalized.parent() == Some(watch_path.as_path())
    }
  })
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
  fn target_watch_falls_back_to_existing_parent_when_target_is_deleted() {
    let root = std::env::temp_dir().join(format!("mech-watch-target-parent-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("main.mec");
    std::fs::write(&target, "x := 1").unwrap();
    let watch = local_workspace_target_watch(&root, "main.mec").unwrap();
    assert_eq!(watch.watch_path, root.canonicalize().unwrap());
    assert_eq!(watch.authorized_path, target.canonicalize().unwrap());
    assert_eq!(watch.filter_paths, vec![target.clone()]);
    std::fs::remove_file(&target).unwrap();
    let watch = local_workspace_target_watch(&root, "main.mec").unwrap();
    assert_eq!(watch.watch_path, root.canonicalize().unwrap());
    assert_eq!(watch.authorized_path, target);
    assert_eq!(watch.filter_paths, vec![watch.authorized_path.clone()]);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn target_watch_ignores_non_local_specifiers() {
    let root = std::env::temp_dir();
    assert!(local_workspace_target_watch(&root, "https://example.com/main.mec").is_none());
  }
  #[test]
  fn file_scoped_watch_grant_authorizes_parent_runtime_watch() {
    let root = std::env::temp_dir().join(format!("mech-watch-file-grant-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("main.mec");
    std::fs::write(&target, "x := 1").unwrap();

    let mut ids = crate::DefaultIdGenerator::new();
    let mut authority = crate::HostFilesystemAuthority::new(crate::MECH_TOOL_SUBJECT, crate::SharedCapabilityKernel::new());
    authority.grant_path(&mut ids, &target, false, [crate::FS_WATCH]).unwrap();
    authority.delegate_path_to(&mut ids, crate::SERVE_HOST_SUBJECT, &target, false, [crate::FS_WATCH]).unwrap();
    let mut runtime = RuntimeBuilder::new().capability_kernel(authority.kernel().clone()).build().unwrap();
    let workspace = RuntimeWorkspace::open(RuntimeWorkspaceConfig::new(&root).capability_subject(crate::SERVE_HOST_SUBJECT).target("main", "main.mec")).unwrap();
    let watcher = RuntimeWorkspaceWatcher::open(&workspace, &mut runtime).unwrap();
    assert_eq!(watcher.watched_paths(), vec![root.canonicalize().unwrap()]);
    std::fs::remove_dir_all(root).unwrap();
  }


  #[test]
  fn file_target_watch_filters_sibling_events() {
    let root = std::env::temp_dir().join(format!("mech-watch-filter-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("main.mec");
    let sibling = root.join("sibling.mec");
    std::fs::write(&target, "x := 1").unwrap();
    std::fs::write(&sibling, "y := 2").unwrap();
    let watch = local_workspace_target_watch(&root, "main.mec").unwrap();
    let mut watches = BTreeSet::new();
    watches.insert(watch);
    assert!(event_allowed_by_watches(&watches, &target));
    assert!(!event_allowed_by_watches(&watches, &sibling));
    std::fs::remove_file(&target).unwrap();
    assert!(event_allowed_by_watches(&watches, &target));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn target_watch_filter_accepts_symlink_link_and_canonical_paths_but_rejects_siblings() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("mech-watch-symlink-filter-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let real = root.join("real.mec");
    let link = root.join("link.mec");
    let sibling = root.join("sibling.mec");
    std::fs::write(&real, "x := 1").unwrap();
    std::fs::write(&sibling, "y := 2").unwrap();
    symlink(&real, &link).unwrap();

    let watch = local_workspace_target_watch(&root, "link.mec").unwrap();
    assert_eq!(watch.watch_path, root.canonicalize().unwrap());
    assert_eq!(watch.authorized_path, real.canonicalize().unwrap());
    assert!(watch.filter_paths.contains(&link));
    assert!(watch.filter_paths.contains(&real.canonicalize().unwrap()));

    let mut watches = BTreeSet::new();
    watches.insert(watch);
    assert!(event_allowed_by_watches(&watches, &link));
    assert!(event_allowed_by_watches(&watches, &real));
    assert!(!event_allowed_by_watches(&watches, &sibling));
    std::fs::remove_file(&link).unwrap();
    assert!(event_allowed_by_watches(&watches, &link));
    std::fs::remove_dir_all(root).unwrap();
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

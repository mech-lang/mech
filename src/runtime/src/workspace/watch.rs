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
  watched_keys: BTreeSet<RuntimeWorkspaceWatchKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeWorkspaceWatchedPath {
  watch_path: PathBuf,
  authorized_path: PathBuf,
  filter_paths: Vec<PathBuf>,
  recursive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeWorkspaceWatchKey {
  path: PathBuf,
  recursive: bool,
}

fn watch_key(watch: &RuntimeWorkspaceWatchedPath) -> RuntimeWorkspaceWatchKey {
  RuntimeWorkspaceWatchKey {
    path: watch.watch_path.clone(),
    recursive: watch.recursive,
  }
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
      watched_keys: BTreeSet::new(),
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
    let desired = preserve_existing_watch_authorizations(
      &self.watched_paths,
      desired_watch_paths(workspace),
    );

    if let Some(subject) = workspace.config().capability_subject.as_deref() {
      for watch in &desired {
        crate::check_fs_capability(runtime.capability_kernel_mut(), subject, crate::FS_WATCH, &watch.authorized_path)?;
      }
    }

    let desired_keys: BTreeSet<_> = desired.iter().map(watch_key).collect();
    let current_keys = self.watched_keys.clone();

    for key in current_keys.difference(&desired_keys) {
      self.watcher.unwatch(&key.path).map_err(|error| {
        watch_error(format!(
          "could not unwatch `{}`: {}",
          key.path.display(),
          error,
        ))
      })?;
      self.watched_keys.remove(key);
    }

    for key in desired_keys.difference(&self.watched_keys).cloned().collect::<Vec<_>>() {
      let recursive_mode = if key.recursive {
        RecursiveMode::Recursive
      } else {
        RecursiveMode::NonRecursive
      };

      self.watcher.watch(&key.path, recursive_mode).map_err(|error| {
        watch_error(format!(
          "could not watch `{}`: {}",
          key.path.display(),
          error,
        ))
      })?;

      self.watched_keys.insert(key);
    }

    self.watched_paths = desired;
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
    self.watched_keys
      .iter()
      .map(|key| key.path.clone())
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
      watched_keys: BTreeSet::new(),
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
    for watch in local_workspace_target_watches(&workspace.config().root, &target.specifier) {
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
  local_workspace_target_watches(root, specifier).into_iter().next()
}

fn local_workspace_target_watches(
  root: &Path,
  specifier: &str,
) -> Vec<RuntimeWorkspaceWatchedPath> {
  if specifier.contains("://") {
    return Vec::new();
  }
  let path = PathBuf::from(specifier);
  let joined = if path.is_absolute() { path.clone() } else { root.join(&path) };
  if joined.exists() && joined.is_dir() {
    let Some(path) = joined.canonicalize().ok() else { return Vec::new(); };
    return vec![RuntimeWorkspaceWatchedPath {
      watch_path: path.clone(),
      authorized_path: path,
      filter_paths: Vec::new(),
      recursive: false,
    }];
  }
  let filter_path = if joined.is_absolute() {
    joined.clone()
  } else {
    root.join(&path)
  };
  let authorized_path = if joined.exists() {
    let Some(canonical) = joined.canonicalize().ok() else { return Vec::new(); };
    canonical
  } else {
    filter_path.clone()
  };
  let Some(parent) = filter_path.parent().and_then(|parent| parent.canonicalize().ok()) else {
    return Vec::new();
  };
  let mut filter_paths = vec![filter_path.clone()];
  let canonical_target = filter_path.canonicalize().ok();
  if let Some(canonical) = &canonical_target {
    if canonical != &filter_path {
      filter_paths.push(canonical.clone());
    }
  }
  let mut watches = vec![RuntimeWorkspaceWatchedPath {
    watch_path: parent,
    authorized_path: authorized_path.clone(),
    filter_paths,
    recursive: false,
  }];

  if let Some(canonical) = canonical_target {
    if let Some(canonical_parent) = canonical.parent().and_then(|parent| parent.canonicalize().ok()) {
      if canonical_parent != watches[0].watch_path {
        watches.push(RuntimeWorkspaceWatchedPath {
          watch_path: canonical_parent,
          authorized_path,
          filter_paths: vec![canonical],
          recursive: false,
        });
      }
    }
  }

  watches
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
      normalized == watch_path || normalized.parent() == Some(watch_path.as_path())
    }
  })
}

fn watch_filter_matches(existing: &RuntimeWorkspaceWatchedPath, desired: &RuntimeWorkspaceWatchedPath) -> bool {
  if existing.filter_paths.is_empty() || desired.filter_paths.is_empty() {
    return false;
  }
  existing.watch_path == desired.watch_path
    && desired.filter_paths.iter().any(|desired_path| {
      existing.filter_paths.iter().any(|existing_path| {
        desired_path == existing_path
          || normalize_watch_event_path(desired_path) == normalize_watch_event_path(existing_path)
      })
    })
}

fn can_preserve_watch_authorization(
  existing: &RuntimeWorkspaceWatchedPath,
  desired: &RuntimeWorkspaceWatchedPath,
) -> bool {
  if !watch_filter_matches(existing, desired) {
    return false;
  }

  if desired.authorized_path.exists() {
    let desired_authorized = normalize_watch_event_path(&desired.authorized_path);
    let existing_authorized = normalize_watch_event_path(&existing.authorized_path);
    return desired_authorized == existing_authorized;
  }

  true
}

fn preserve_existing_watch_authorizations(
  current: &BTreeSet<RuntimeWorkspaceWatchedPath>,
  desired: BTreeSet<RuntimeWorkspaceWatchedPath>,
) -> BTreeSet<RuntimeWorkspaceWatchedPath> {
  desired
    .into_iter()
    .map(|mut watch| {
      if let Some(existing) = current.iter().find(|existing| watch_filter_matches(existing, &watch)) {
        if can_preserve_watch_authorization(existing, &watch) {
          watch.authorized_path = existing.authorized_path.clone();
          for path in &existing.filter_paths {
            if !watch.filter_paths.contains(path) {
              watch.filter_paths.push(path.clone());
            }
          }
          watch.filter_paths.sort();
          watch.filter_paths.dedup();
        }
      }
      watch
    })
    .collect()
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

  #[cfg(unix)]
  #[test]
  fn symlink_file_watch_preserves_authorized_path_after_link_delete() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("mech-watch-symlink-preserve-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let real = root.join("real.mec");
    let link = root.join("link.mec");
    let sibling = root.join("sibling.mec");
    std::fs::write(&real, "x := 1").unwrap();
    std::fs::write(&sibling, "y := 2").unwrap();
    symlink(&real, &link).unwrap();

    let existing_watch = local_workspace_target_watch(&root, "link.mec").unwrap();
    let canonical_real = real.canonicalize().unwrap();
    assert_eq!(existing_watch.authorized_path, canonical_real);
    assert!(existing_watch.filter_paths.contains(&link));
    assert!(existing_watch.filter_paths.contains(&canonical_real));

    let mut current = BTreeSet::new();
    current.insert(existing_watch);
    std::fs::remove_file(&link).unwrap();
    let mut desired = BTreeSet::new();
    desired.insert(local_workspace_target_watch(&root, "link.mec").unwrap());
    let reconciled = preserve_existing_watch_authorizations(&current, desired);
    let watch = reconciled.iter().next().unwrap();

    assert_eq!(watch.authorized_path, canonical_real);
    assert!(watch.filter_paths.contains(&link));
    assert!(event_allowed_by_watches(&reconciled, &link));
    assert!(!event_allowed_by_watches(&reconciled, &sibling));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn retargeted_symlink_watch_uses_new_authorization_and_filters() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("mech-watch-symlink-retarget-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let a = root.join("a.mec");
    let b = root.join("b.mec");
    let link = root.join("link.mec");
    let sibling = root.join("sibling.mec");
    std::fs::write(&a, "x := 1").unwrap();
    std::fs::write(&b, "x := 2").unwrap();
    std::fs::write(&sibling, "y := 3").unwrap();
    symlink(&a, &link).unwrap();

    let current = local_workspace_target_watches(&root, "link.mec")
      .into_iter()
      .collect::<BTreeSet<_>>();
    let canonical_a = a.canonicalize().unwrap();
    let canonical_b = b.canonicalize().unwrap();
    assert!(current.iter().any(|watch| watch.authorized_path == canonical_a));

    std::fs::remove_file(&link).unwrap();
    symlink(&b, &link).unwrap();

    let desired = local_workspace_target_watches(&root, "link.mec")
      .into_iter()
      .collect::<BTreeSet<_>>();
    let reconciled = preserve_existing_watch_authorizations(&current, desired);
    let link_watch = reconciled
      .iter()
      .find(|watch| watch.filter_paths.contains(&link))
      .unwrap();

    assert_eq!(link_watch.authorized_path, canonical_b);
    assert_ne!(link_watch.authorized_path, canonical_a);
    assert!(link_watch.filter_paths.contains(&link));
    assert!(link_watch.filter_paths.contains(&canonical_b));
    assert!(!link_watch.filter_paths.contains(&canonical_a));
    assert!(event_allowed_by_watches(&reconciled, &link));
    assert!(event_allowed_by_watches(&reconciled, &b));
    assert!(!event_allowed_by_watches(&reconciled, &sibling));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn symlink_target_outside_link_dir_watches_link_and_target_parents() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("mech-watch-symlink-outside-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let link_dir = root.join("links");
    let shared_dir = root.join("shared");
    std::fs::create_dir_all(&link_dir).unwrap();
    std::fs::create_dir_all(&shared_dir).unwrap();
    let real = shared_dir.join("real.mec");
    let link = link_dir.join("link.mec");
    let link_sibling = link_dir.join("sibling.mec");
    let target_sibling = shared_dir.join("sibling.mec");
    std::fs::write(&real, "x := 1").unwrap();
    std::fs::write(&link_sibling, "y := 2").unwrap();
    std::fs::write(&target_sibling, "z := 3").unwrap();
    symlink(&real, &link).unwrap();

    let watches = local_workspace_target_watches(&root, "links/link.mec");
    let link_parent = link_dir.canonicalize().unwrap();
    let target_parent = shared_dir.canonicalize().unwrap();
    assert!(watches.iter().any(|watch| watch.watch_path == link_parent && watch.filter_paths.contains(&link)));
    assert!(watches.iter().any(|watch| watch.watch_path == target_parent && watch.filter_paths.contains(&real.canonicalize().unwrap())));
    let watch_set = watches.into_iter().collect::<BTreeSet<_>>();
    assert!(event_allowed_by_watches(&watch_set, &link));
    assert!(event_allowed_by_watches(&watch_set, &real));
    assert!(!event_allowed_by_watches(&watch_set, &link_sibling));
    assert!(!event_allowed_by_watches(&watch_set, &target_sibling));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[cfg(unix)]
  #[test]
  fn symlink_target_same_parent_uses_single_watch_with_both_filters() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!("mech-watch-symlink-same-parent-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let real = root.join("real.mec");
    let link = root.join("link.mec");
    std::fs::write(&real, "x := 1").unwrap();
    symlink(&real, &link).unwrap();

    let watches = local_workspace_target_watches(&root, "link.mec");
    assert_eq!(watches.len(), 1);
    assert!(watches[0].filter_paths.contains(&link));
    assert!(watches[0].filter_paths.contains(&real.canonicalize().unwrap()));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn shared_parent_watch_keys_collapse_multiple_file_filters() {
    let root = std::env::temp_dir().join(format!("mech-watch-shared-parent-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("a.mec"), "a := 1").unwrap();
    std::fs::write(root.join("b.mec"), "b := 2").unwrap();
    let mut watches = BTreeSet::new();
    watches.extend(local_workspace_target_watches(&root, "a.mec"));
    watches.extend(local_workspace_target_watches(&root, "b.mec"));
    let keys = watches.iter().map(watch_key).collect::<BTreeSet<_>>();
    assert_eq!(watches.len(), 2);
    assert_eq!(keys.len(), 1);
    assert_eq!(keys.iter().next().unwrap().path, root.canonicalize().unwrap());
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn nonrecursive_directory_watch_allows_directory_and_direct_children_only() {
    let root = std::env::temp_dir().join(format!("mech-watch-directory-event-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let docs = root.join("docs");
    let nested = docs.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let watch = RuntimeWorkspaceWatchedPath {
      watch_path: docs.clone(),
      authorized_path: docs.clone(),
      filter_paths: Vec::new(),
      recursive: false,
    };
    let watches = [watch].into_iter().collect::<BTreeSet<_>>();
    assert!(event_allowed_by_watches(&watches, &docs));
    assert!(event_allowed_by_watches(&watches, &docs.join("main.mec")));
    assert!(!event_allowed_by_watches(&watches, &nested.join("main.mec")));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn filtered_file_watch_still_rejects_parent_directory_event() {
    let root = std::env::temp_dir().join(format!("mech-watch-filter-parent-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let target = root.join("main.mec");
    std::fs::write(&target, "x := 1").unwrap();
    let watch = local_workspace_target_watch(&root, "main.mec").unwrap();
    let watches = [watch].into_iter().collect::<BTreeSet<_>>();
    assert!(!event_allowed_by_watches(&watches, &root));
    assert!(event_allowed_by_watches(&watches, &target));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn recursive_directory_watch_still_allows_descendants() {
    let root = std::env::temp_dir().join(format!("mech-watch-recursive-event-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let docs = root.join("docs");
    let nested_file = docs.join("nested/main.mec");
    std::fs::create_dir_all(nested_file.parent().unwrap()).unwrap();
    let watch = RuntimeWorkspaceWatchedPath {
      watch_path: docs.clone(),
      authorized_path: docs.clone(),
      filter_paths: Vec::new(),
      recursive: true,
    };
    let watches = [watch].into_iter().collect::<BTreeSet<_>>();
    assert!(event_allowed_by_watches(&watches, &docs));
    assert!(event_allowed_by_watches(&watches, &nested_file));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn regular_missing_file_watch_without_existing_authorization_uses_missing_path() {
    let root = std::env::temp_dir().join(format!("mech-watch-regular-missing-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let missing = root.join("missing.mec");
    let watch = local_workspace_target_watch(&root, "missing.mec").unwrap();
    assert_eq!(watch.authorized_path, missing);
    assert_eq!(watch.filter_paths, vec![watch.authorized_path.clone()]);
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

use super::discovery::{RuntimeWorkspaceDiscoveredFile, discover_workspace_files};
use super::*;

#[derive(Clone, Debug)]
pub struct RuntimeWorkspace {
    config: RuntimeWorkspaceConfig,
    snapshot: Option<RuntimeWorkspaceSnapshot>,
    // Rejected inputs must remain observable without becoming accepted sources.
    rejected_targets: BTreeMap<String, RejectedTargetSource>,
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
            rejected_targets: BTreeMap::new(),
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
        let mut rejected_targets = BTreeMap::new();

        for target in &self.config.targets {
            match load_target(
                &self.config.root,
                runtime,
                target,
                options.clone(),
                self.config.capability_subject.as_deref(),
            )? {
                Ok(snapshot) => {
                    targets.insert(target.name.clone(), snapshot);
                }
                Err(failure) => {
                    if let Some(source) = failure.rejected_source {
                        rejected_targets.insert(target.name.clone(), source);
                    }
                    diagnostics.push(failure.diagnostic);
                }
            }
        }

        let (discovered_files, discovery_diagnostics) = discover_workspace_files(
            &self.config.root,
            &self.config.folders,
            runtime,
            self.config.capability_subject.as_deref(),
        )?;
        diagnostics.extend(discovery_diagnostics);
        let (extra_module_versions, discovered_diagnostics) =
            load_discovered_workspace_files(runtime, &discovered_files, options.clone());
        diagnostics.extend(discovered_diagnostics);

        let snapshot = collect_snapshot(
            runtime,
            self.config.root.clone(),
            targets,
            extra_module_versions,
            diagnostics,
        )?;
        self.snapshot = Some(snapshot.clone());
        self.rejected_targets = rejected_targets;
        Ok(snapshot)
    }

    pub fn refresh(
        &mut self,
        runtime: &mut MechRuntime,
        options: ModuleBuildOptions,
    ) -> MResult<RuntimeWorkspaceRefresh> {
        self.refresh_inner(runtime, options, false)
    }

    pub(super) fn refresh_after_watch_events(
        &mut self,
        runtime: &mut MechRuntime,
        options: ModuleBuildOptions,
        events: &[RuntimeWorkspaceWatchEvent],
    ) -> MResult<RuntimeWorkspaceRefresh> {
        let retry_rejected = events.iter().any(|event| {
            matches!(
                crate::SourceKind::from_path(&event.path),
                crate::SourceKind::Mech
            )
        });
        self.refresh_inner(runtime, options, retry_rejected)
    }

    pub(super) fn has_rejected_targets(&self) -> bool {
        !self.rejected_targets.is_empty()
    }

    fn refresh_inner(
        &mut self,
        runtime: &mut MechRuntime,
        options: ModuleBuildOptions,
        retry_rejected: bool,
    ) -> MResult<RuntimeWorkspaceRefresh> {
        let Some(previous) = self.snapshot.clone() else {
            return Err(MechError::new(RuntimeWorkspaceNotLoaded, None));
        };

        let (discovered_files, discovery_diagnostics) = discover_workspace_files(
            &self.config.root,
            &self.config.folders,
            runtime,
            self.config.capability_subject.as_deref(),
        )?;

        let mut changes = previous.changed_sources();
        changes.extend(added_folder_file_changes(&previous, &discovered_files));
        let mut affected = BTreeSet::new();
        for (name, source) in &self.rejected_targets {
            if let Some(subject) = self.config.capability_subject.as_deref() {
                let authorized = [crate::FS_RESOLVE, crate::FS_READ]
                    .into_iter()
                    .all(|operation| {
                        crate::check_fs_capability(
                            runtime.capability_kernel_mut(),
                            subject,
                            operation,
                            &source.path,
                        )
                        .is_ok()
                    });
                if !authorized {
                    continue;
                }
            }
            let current_hash = std::fs::read(&source.path).ok().map(hash_content);
            if current_hash == source.content_hash {
                continue;
            }
            affected.insert(name.clone());
            if !changes
                .iter()
                .any(|change| change.path.as_ref() == Some(&source.path))
            {
                changes.push(RuntimeWorkspaceChange {
                    canonical_uri: source.canonical_uri.clone(),
                    path: Some(source.path.clone()),
                    kind: match (source.content_hash, current_hash) {
                        (None, Some(_)) => RuntimeWorkspaceChangeKind::Added,
                        (_, None) => RuntimeWorkspaceChangeKind::Removed,
                        _ => RuntimeWorkspaceChangeKind::Modified,
                    },
                    previous_hash: source.content_hash,
                    current_hash,
                });
            }
        }

        if changes.is_empty() && !(retry_rejected && self.has_rejected_targets()) {
            return Ok(RuntimeWorkspaceRefresh {
                snapshot: previous,
                changes,
                affected_targets: Vec::new(),
                refresh_diagnostics: Vec::new(),
            });
        }

        affected.extend(previous.affected_targets(&changes));
        // Failed admission leaves no accepted import graph. A workspace edit may
        // repair a dependency even when the rejected target itself is unchanged.
        affected.extend(self.rejected_targets.keys().cloned());
        let affected_targets = affected.iter().cloned().collect::<Vec<_>>();

        let retained_diagnostics = previous
            .diagnostics
            .iter()
            .filter(|diagnostic| match diagnostic.target.as_deref() {
                Some(target) => !affected.contains(target),
                // Discovery and discovered-file diagnostics are recomputed below.
                None => false,
            })
            .cloned()
            .collect::<Vec<_>>();

        let mut targets = previous
            .targets
            .iter()
            .filter(|(name, _)| !affected.contains(*name))
            .map(|(name, target)| (name.clone(), target.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut refresh_diagnostics = discovery_diagnostics;
        let mut rejected_targets = self.rejected_targets.clone();

        for target in self
            .config
            .targets
            .iter()
            .filter(|target| affected.contains(&target.name))
        {
            match load_target(
                &self.config.root,
                runtime,
                target,
                options.clone(),
                self.config.capability_subject.as_deref(),
            )? {
                Ok(snapshot) => {
                    rejected_targets.remove(&target.name);
                    targets.insert(target.name.clone(), snapshot);
                }
                Err(failure) => {
                    if let Some(source) = failure.rejected_source {
                        rejected_targets.insert(target.name.clone(), source);
                    }
                    refresh_diagnostics.push(failure.diagnostic);
                }
            }
        }

        let (extra_module_versions, discovered_diagnostics) =
            load_discovered_workspace_files(runtime, &discovered_files, options.clone());
        refresh_diagnostics.extend(discovered_diagnostics);

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
        self.rejected_targets = rejected_targets;

        Ok(RuntimeWorkspaceRefresh {
            snapshot,
            changes,
            affected_targets,
            refresh_diagnostics,
        })
    }

    pub fn target(&self, name: &str) -> Option<&RuntimeWorkspaceTargetSnapshot> {
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

fn added_folder_file_changes(
    previous: &RuntimeWorkspaceSnapshot,
    discovered_files: &[RuntimeWorkspaceDiscoveredFile],
) -> Vec<RuntimeWorkspaceChange> {
    let previous_paths = previous
        .sources
        .values()
        .filter_map(|source| source.path.clone())
        .collect::<BTreeSet<_>>();

    discovered_files
        .iter()
        .filter(|file| !previous_paths.contains(&file.canonical_path))
        .map(|file| RuntimeWorkspaceChange {
            canonical_uri: file.canonical_path.to_string_lossy().to_string(),
            path: Some(file.canonical_path.clone()),
            kind: RuntimeWorkspaceChangeKind::Added,
            previous_hash: None,
            current_hash: std::fs::read(&file.canonical_path).ok().map(hash_content),
        })
        .collect()
}

fn load_discovered_workspace_files(
    runtime: &mut MechRuntime,
    discovered_files: &[RuntimeWorkspaceDiscoveredFile],
    options: ModuleBuildOptions,
) -> (Vec<ModuleVersionId>, Vec<RuntimeWorkspaceDiagnostic>) {
    let mut module_versions = Vec::new();
    let mut diagnostics = Vec::new();

    for file in discovered_files {
        match runtime.resolve_and_store_module_source(file.specifier.as_str(), options.clone()) {
            Ok(Some(module_version)) => {
                module_versions.push(module_version);
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

    (module_versions, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::{ResolvedSource, SourceRequest, SourceResolver};
    use crate::{FileSourceResolver, RuntimeBuilder};

    fn setup_workspace(source: &str) -> (PathBuf, RuntimeWorkspace, MechRuntime) {
        let root = std::env::temp_dir().join(format!(
            "mech-workspace-rejected-target-{}",
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        std::fs::write(root.join("main.mec"), source).unwrap();
        let workspace =
            RuntimeWorkspace::open(RuntimeWorkspaceConfig::new(&root).target("main", "main.mec"))
                .unwrap();
        let runtime = RuntimeBuilder::new()
            .source_resolver(FileSourceResolver::new(&root))
            .build()
            .unwrap();
        (root, workspace, runtime)
    }

    fn options() -> ModuleBuildOptions<'static> {
        ModuleBuildOptions::new("test", "v0.4", "native", &[], &[])
    }

    fn assert_idle(workspace: &mut RuntimeWorkspace, runtime: &mut MechRuntime) {
        let refresh = workspace.refresh(runtime, options()).unwrap();
        assert!(refresh.changes.is_empty());
        assert!(refresh.affected_targets.is_empty());
        assert!(refresh.refresh_diagnostics.is_empty());
    }

    #[test]
    fn workspace_refresh_retries_initially_rejected_target_only_after_edit() {
        let (root, mut workspace, mut runtime) = setup_workspace("value := [\n");
        let snapshot = workspace.load(&mut runtime, options()).unwrap();
        assert!(snapshot.targets.is_empty());
        assert!(snapshot.sources.is_empty());
        assert_eq!(snapshot.diagnostics.len(), 1);
        assert_eq!(
            snapshot.diagnostics[0].canonical_uri.as_deref(),
            Some(root.join("main.mec").to_str().unwrap()),
        );
        assert_idle(&mut workspace, &mut runtime);

        std::fs::write(root.join("main.mec"), "value := true\n").unwrap();
        let refresh = workspace.refresh(&mut runtime, options()).unwrap();
        assert_eq!(refresh.affected_targets, ["main"]);
        assert_eq!(refresh.changes.len(), 1);
        assert!(refresh.snapshot.targets.contains_key("main"));
        assert!(!refresh.snapshot.sources.is_empty());
        assert!(refresh.snapshot.diagnostics.is_empty());
        assert_idle(&mut workspace, &mut runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_refresh_recovers_target_after_rejected_edit() {
        let (root, mut workspace, mut runtime) = setup_workspace("value := true\n");
        let snapshot = workspace.load(&mut runtime, options()).unwrap();
        assert!(snapshot.targets.contains_key("main"));
        assert!(snapshot.diagnostics.is_empty());

        std::fs::write(root.join("main.mec"), "value := [\n").unwrap();
        let rejected = workspace.refresh(&mut runtime, options()).unwrap();
        assert_eq!(rejected.affected_targets, ["main"]);
        assert!(rejected.snapshot.targets.is_empty());
        assert!(rejected.snapshot.sources.is_empty());
        assert_eq!(rejected.refresh_diagnostics.len(), 1);
        assert_idle(&mut workspace, &mut runtime);

        std::fs::write(root.join("main.mec"), "value := false\n").unwrap();
        let repaired = workspace.refresh(&mut runtime, options()).unwrap();
        assert_eq!(repaired.affected_targets, ["main"]);
        assert!(repaired.snapshot.targets.contains_key("main"));
        assert!(repaired.snapshot.diagnostics.is_empty());
        assert_idle(&mut workspace, &mut runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_refresh_detects_repair_during_failed_load() {
        #[derive(Debug)]
        struct RepairDuringLoad {
            resolver: FileSourceResolver,
            path: PathBuf,
        }

        impl SourceResolver for RepairDuringLoad {
            fn resolve(&self, request: &SourceRequest) -> MResult<Option<ResolvedSource>> {
                let result = self.resolver.resolve(request);
                if result.is_err() {
                    std::fs::write(&self.path, "value := true\n").unwrap();
                }
                result
            }
        }

        let (root, mut workspace, _) = setup_workspace("value := [\n");
        let mut runtime = RuntimeBuilder::new()
            .source_resolver(RepairDuringLoad {
                resolver: FileSourceResolver::new(&root),
                path: root.join("main.mec"),
            })
            .build()
            .unwrap();
        let rejected = workspace.load(&mut runtime, options()).unwrap();
        assert_eq!(rejected.diagnostics.len(), 1);
        assert!(rejected.sources.is_empty());

        let repaired = workspace.refresh(&mut runtime, options()).unwrap();
        assert_eq!(repaired.affected_targets, ["main"]);
        assert!(repaired.snapshot.targets.contains_key("main"));
        assert!(repaired.snapshot.diagnostics.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_does_not_track_target_rejected_by_filesystem_authority() {
        let (root, _, mut runtime) = setup_workspace("value := [\n");
        let mut workspace = RuntimeWorkspace::open(
            RuntimeWorkspaceConfig::new(&root)
                .target("main", "main.mec")
                .capability_subject(crate::SERVE_HOST_SUBJECT),
        )
        .unwrap();
        let snapshot = workspace.load(&mut runtime, options()).unwrap();
        assert_eq!(snapshot.diagnostics.len(), 1);
        assert!(snapshot.diagnostics[0].canonical_uri.is_none());
        assert!(workspace.rejected_targets.is_empty());
        std::fs::write(root.join("main.mec"), "value := true\n").unwrap();
        assert_idle(&mut workspace, &mut runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_refresh_clears_repaired_discovered_file_diagnostic() {
        let (root, _, mut runtime) = setup_workspace("value := [\n");
        let mut workspace =
            RuntimeWorkspace::open(RuntimeWorkspaceConfig::new(&root).folder(".")).unwrap();
        let snapshot = workspace.load(&mut runtime, options()).unwrap();
        assert!(snapshot.sources.is_empty());
        assert_eq!(snapshot.diagnostics.len(), 1);
        assert!(snapshot.diagnostics[0].target.is_none());

        std::fs::write(root.join("main.mec"), "value := true\n").unwrap();
        let repaired = workspace.refresh(&mut runtime, options()).unwrap();
        assert_eq!(repaired.snapshot.sources.len(), 1);
        assert!(repaired.snapshot.diagnostics.is_empty());
        assert!(repaired.refresh_diagnostics.is_empty());
        assert_idle(&mut workspace, &mut runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_refresh_recovers_rejected_target_after_dependency_repair() {
        let main_source = "+> ./support\nanswer := support/value\nanswer\n";
        let (root, _, mut runtime) = setup_workspace(main_source);
        std::fs::write(root.join("support.mec"), "value := [\n<+ value\n").unwrap();
        let mut workspace = RuntimeWorkspace::open(
            RuntimeWorkspaceConfig::new(&root)
                .target("main", "main.mec")
                .folder("."),
        )
        .unwrap();
        let rejected = workspace.load(&mut runtime, options()).unwrap();
        assert!(rejected.targets.is_empty());
        assert!(
            rejected
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.target.as_deref() == Some("main") })
        );

        std::fs::write(root.join("support.mec"), "value := 42\n<+ value\n").unwrap();
        let repaired = workspace.refresh(&mut runtime, options()).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("main.mec")).unwrap(),
            main_source
        );
        assert_eq!(repaired.affected_targets, ["main"]);
        assert!(repaired.snapshot.targets.contains_key("main"));
        assert!(repaired.snapshot.diagnostics.is_empty());
        assert!(repaired.refresh_diagnostics.is_empty());
        assert_idle(&mut workspace, &mut runtime);
        std::fs::remove_dir_all(root).unwrap();
    }
}

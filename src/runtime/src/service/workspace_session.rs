use std::path::PathBuf;
#[cfg(feature = "resident-routing")]
use std::sync::Arc;

#[cfg(feature = "resident-routing")]
use mech_core::FunctionCatalog;
use mech_core::MResult;

use crate::{
    DefaultIdGenerator, EventSink, FileSourceResolver, IdGenerator, MechRuntime,
    ModuleBuildOptions, RuntimeBuilder, RuntimeConfig, RuntimeEvent, RuntimeEventKind,
    RuntimeWorkspace, RuntimeWorkspaceConfig, RuntimeWorkspaceDiagnostic, RuntimeWorkspaceFolder,
    RuntimeWorkspaceRefresh, RuntimeWorkspaceSnapshot, RuntimeWorkspaceTarget,
    RuntimeWorkspaceWatchPoll, RuntimeWorkspaceWatcher, SharedCapabilityKernel,
};

pub struct ServerWorkspaceSession {
    runtime: MechRuntime,
    workspace: RuntimeWorkspace,
    watcher: RuntimeWorkspaceWatcher,
    event_ids: DefaultIdGenerator,
    event_sequence: u64,
}

impl ServerWorkspaceSession {
    pub fn open(
        root: impl Into<PathBuf>,
        targets: Vec<RuntimeWorkspaceTarget>,
        folders: Vec<RuntimeWorkspaceFolder>,
        options: ModuleBuildOptions,
    ) -> MResult<Self> {
        Self::open_with_builder(
            root.into(),
            targets,
            folders,
            options,
            RuntimeBuilder::new(),
        )
    }

    #[cfg(feature = "resident-routing")]
    pub fn open_with_function_catalog(
        root: impl Into<PathBuf>,
        targets: Vec<RuntimeWorkspaceTarget>,
        folders: Vec<RuntimeWorkspaceFolder>,
        options: ModuleBuildOptions,
        function_catalog: Arc<FunctionCatalog>,
    ) -> MResult<Self> {
        Self::open_with_builder(
            root.into(),
            targets,
            folders,
            options,
            RuntimeBuilder::new().function_catalog(function_catalog),
        )
    }

    fn open_with_builder(
        root: PathBuf,
        targets: Vec<RuntimeWorkspaceTarget>,
        folders: Vec<RuntimeWorkspaceFolder>,
        options: ModuleBuildOptions,
        builder: RuntimeBuilder,
    ) -> MResult<Self> {
        let mut runtime = builder
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
        let watcher = RuntimeWorkspaceWatcher::open(&workspace, &mut runtime)?;

        Ok(Self {
            runtime,
            workspace,
            watcher,
            event_ids: DefaultIdGenerator::new(),
            event_sequence: 0,
        })
    }

    pub fn open_with_capabilities(
        root: impl Into<PathBuf>,
        targets: Vec<RuntimeWorkspaceTarget>,
        folders: Vec<RuntimeWorkspaceFolder>,
        options: ModuleBuildOptions,
        capability_kernel: SharedCapabilityKernel,
        capability_subject: impl Into<String>,
    ) -> MResult<Self> {
        Self::open_with_capabilities_and_config(
            root,
            targets,
            folders,
            options,
            capability_kernel,
            capability_subject,
            RuntimeConfig::default(),
        )
    }

    pub fn open_with_capabilities_and_config(
        root: impl Into<PathBuf>,
        targets: Vec<RuntimeWorkspaceTarget>,
        folders: Vec<RuntimeWorkspaceFolder>,
        options: ModuleBuildOptions,
        capability_kernel: SharedCapabilityKernel,
        capability_subject: impl Into<String>,
        runtime_config: RuntimeConfig,
    ) -> MResult<Self> {
        Self::open_with_capabilities_builder(
            root.into(),
            targets,
            folders,
            options,
            capability_kernel,
            capability_subject.into(),
            RuntimeBuilder::new().config(runtime_config),
        )
    }

    #[cfg(feature = "resident-routing")]
    pub fn open_with_capabilities_config_and_function_catalog(
        root: impl Into<PathBuf>,
        targets: Vec<RuntimeWorkspaceTarget>,
        folders: Vec<RuntimeWorkspaceFolder>,
        options: ModuleBuildOptions,
        capability_kernel: SharedCapabilityKernel,
        capability_subject: impl Into<String>,
        runtime_config: RuntimeConfig,
        function_catalog: Arc<FunctionCatalog>,
    ) -> MResult<Self> {
        Self::open_with_capabilities_builder(
            root.into(),
            targets,
            folders,
            options,
            capability_kernel,
            capability_subject.into(),
            RuntimeBuilder::new()
                .config(runtime_config)
                .function_catalog(function_catalog),
        )
    }

    fn open_with_capabilities_builder(
        root: PathBuf,
        targets: Vec<RuntimeWorkspaceTarget>,
        folders: Vec<RuntimeWorkspaceFolder>,
        options: ModuleBuildOptions,
        capability_kernel: SharedCapabilityKernel,
        capability_subject: String,
        builder: RuntimeBuilder,
    ) -> MResult<Self> {
        let mut runtime = builder
            .capability_kernel(capability_kernel.clone())
            .source_resolver(
                FileSourceResolver::new(&root)
                    .with_capabilities(capability_kernel, capability_subject.clone()),
            )
            .build()?;
        let mut config = RuntimeWorkspaceConfig::new(&root).capability_subject(capability_subject);
        for target in targets {
            config = config.target(target.name, target.specifier);
        }
        for folder in folders {
            config = config.folder_recursive(folder.specifier, folder.recursive);
        }
        let mut workspace = RuntimeWorkspace::open(config)?;
        workspace.load(&mut runtime, options.clone())?;
        let watcher = RuntimeWorkspaceWatcher::open(&workspace, &mut runtime)?;
        Ok(Self {
            runtime,
            workspace,
            watcher,
            event_ids: DefaultIdGenerator::new(),
            event_sequence: 0,
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

    pub fn set_watch_event_notifier(&mut self, notifier: impl Fn() + Send + Sync + 'static) {
        self.watcher.set_event_notifier(notifier);
    }

    pub fn snapshot(&self) -> Option<&RuntimeWorkspaceSnapshot> {
        self.workspace.snapshot()
    }

    pub fn refresh(&mut self, options: ModuleBuildOptions) -> MResult<RuntimeWorkspaceRefresh> {
        self.workspace.refresh(&mut self.runtime, options)
    }

    pub fn emit_initial_events(&mut self, events: &mut dyn EventSink) -> MResult<()> {
        let Some(snapshot) = self.workspace.snapshot().cloned() else {
            return Ok(());
        };

        self.emit_snapshot_loaded_events(&snapshot, events)?;
        self.emit_workspace_diagnostics(&snapshot.diagnostics, events)
    }

    pub fn poll(&mut self, options: ModuleBuildOptions) -> MResult<RuntimeWorkspaceWatchPoll> {
        self.watcher
            .poll(&mut self.workspace, &mut self.runtime, options)
    }

    pub fn poll_and_emit(
        &mut self,
        options: ModuleBuildOptions,
        events: &mut dyn EventSink,
    ) -> MResult<RuntimeWorkspaceWatchPoll> {
        let poll = self.poll(options)?;

        self.emit_watch_poll_events(&poll, events)?;

        Ok(poll)
    }

    fn emit_event(&mut self, kind: RuntimeEventKind, events: &mut dyn EventSink) -> MResult<()> {
        self.event_sequence += 1;
        let event = RuntimeEvent::new(self.event_ids.event_id(), self.event_sequence, kind);
        events.emit(event)?;
        Ok(())
    }

    fn emit_snapshot_loaded_events(
        &mut self,
        snapshot: &RuntimeWorkspaceSnapshot,
        events: &mut dyn EventSink,
    ) -> MResult<()> {
        for source in snapshot.sources.values() {
            self.emit_event(
                RuntimeEventKind::SourceResolved {
                    canonical_uri: source.canonical_uri.clone(),
                },
                events,
            )?;
        }

        for edge in &snapshot.import_edges {
            self.emit_event(
                RuntimeEventKind::ModuleImportLinked {
                    importer: edge.importer,
                    dependency: edge.dependency,
                    specifier: edge.specifier.clone(),
                },
                events,
            )?;
        }

        Ok(())
    }

    fn emit_watch_poll_events(
        &mut self,
        poll: &RuntimeWorkspaceWatchPoll,
        events: &mut dyn EventSink,
    ) -> MResult<()> {
        for event in &poll.events {
            self.emit_event(
                RuntimeEventKind::SourceChanged {
                    canonical_uri: event.path.to_string_lossy().to_string(),
                },
                events,
            )?;
        }

        let Some(refresh) = &poll.refresh else {
            return Ok(());
        };

        for change in &refresh.changes {
            self.emit_event(
                RuntimeEventKind::SourceChanged {
                    canonical_uri: change.canonical_uri.clone(),
                },
                events,
            )?;
        }

        for target_name in &refresh.affected_targets {
            if let Some(target) = refresh.snapshot.targets.get(target_name) {
                self.emit_event(
                    RuntimeEventKind::SourceReloaded {
                        canonical_uri: target.canonical_uri.clone(),
                    },
                    events,
                )?;
            }
        }

        self.emit_workspace_diagnostics(&refresh.refresh_diagnostics, events)
    }

    fn emit_workspace_diagnostics(
        &mut self,
        diagnostics: &[RuntimeWorkspaceDiagnostic],
        events: &mut dyn EventSink,
    ) -> MResult<()> {
        for diagnostic in diagnostics {
            if let Some(canonical_uri) = &diagnostic.canonical_uri {
                self.emit_event(
                    RuntimeEventKind::ModuleCompileFailed {
                        canonical_uri: canonical_uri.clone(),
                        message: diagnostic.message.clone(),
                    },
                    events,
                )?;
            } else {
                self.emit_event(
                    RuntimeEventKind::SourceResolveFailed {
                        specifier: diagnostic
                            .target
                            .clone()
                            .unwrap_or_else(|| "workspace".to_string()),
                        message: diagnostic.message.clone(),
                    },
                    events,
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventId;

    #[derive(Debug, Default)]
    struct RecordingEventSink {
        events: Vec<RuntimeEvent>,
    }

    impl EventSink for RecordingEventSink {
        fn emit(&mut self, event: RuntimeEvent) -> MResult<EventId> {
            let id = event.id;
            self.events.push(event);
            Ok(id)
        }
    }

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
        )
        .unwrap();

        assert!(session.snapshot().is_some());
        let snapshot = session.snapshot().unwrap();
        let main_path = root.join("main.mec").canonicalize().unwrap();

        assert!(snapshot.diagnostics.is_empty());
        assert!(snapshot.targets.contains_key("main"));
        assert!(snapshot.sources.values().any(|source| {
            source
                .path
                .as_ref()
                .and_then(|path| path.canonicalize().ok())
                .as_ref()
                == Some(&main_path)
        }));
    }

    #[cfg(feature = "resident-routing-source")]
    #[test]
    fn server_workspace_session_accepts_an_explicit_function_catalog() {
        let root = setup_session_root();
        std::fs::write(root.join("main.mec"), "true\n").unwrap();
        let catalog = mech_stdlib::source_catalog();

        let mut session = ServerWorkspaceSession::open_with_function_catalog(
            &root,
            vec![main_target()],
            vec![recursive_root_folder()],
            module_options(),
            catalog,
        )
        .unwrap();

        let result = session
            .runtime_mut()
            .load_source_program(
                include_str!(
                    "../../../../tests/architecture/resident-activation/n-body-source-v1.mec"
                ),
                crate::ResidentDurabilityPolicy::Volatile,
            )
            .unwrap();
        assert_eq!(result.route, crate::RuntimeProgramRoute::ResidentPure,);
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
        )
        .unwrap();

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
        )
        .unwrap();

        let poll = session.poll(module_options()).unwrap();

        assert!(poll.events.is_empty());
        assert!(poll.refresh.is_none());
    }

    #[test]
    fn server_workspace_session_notifies_when_a_watch_event_is_queued() {
        let root = setup_session_root();
        let main = root.join("main.mec");
        std::fs::write(&main, "result := false\n").unwrap();
        let main = main.canonicalize().unwrap();

        let mut session = ServerWorkspaceSession::open(
            &root,
            vec![main_target()],
            vec![recursive_root_folder()],
            module_options(),
        )
        .unwrap();
        session.poll(module_options()).unwrap();

        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        session.set_watch_event_notifier(move || {
            if sender.try_send(()).is_err() {
                // The single-slot test signal is already queued or disconnected.
            }
        });
        std::fs::write(&main, "result := true\n").unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let saw_main_event = loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() || receiver.recv_timeout(remaining).is_err() {
                break false;
            }
            let poll = session.poll(module_options()).unwrap();
            if poll
                .events
                .iter()
                .any(|event| event.path.canonicalize().ok().as_ref() == Some(&main))
            {
                break true;
            }
        };
        assert!(
            saw_main_event,
            "filesystem event did not wake the workspace session"
        );

        drop(session);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_workspace_session_emit_initial_events_reports_sources() {
        let root = setup_session_root();
        std::fs::write(root.join("main.mec"), "result := true\n").unwrap();

        let mut session = ServerWorkspaceSession::open(
            &root,
            vec![main_target()],
            vec![recursive_root_folder()],
            module_options(),
        )
        .unwrap();

        let mut sink = RecordingEventSink::default();

        session.emit_initial_events(&mut sink).unwrap();

        assert!(sink.events.iter().any(|event| {
            matches!(
              &event.kind,
              RuntimeEventKind::SourceResolved { canonical_uri }
                if canonical_uri.ends_with("main.mec")
            )
        }));
    }

    #[test]
    fn server_workspace_session_emit_initial_events_reports_diagnostics() {
        let root = setup_session_root();

        let mut session = ServerWorkspaceSession::open(
            &root,
            vec![RuntimeWorkspaceTarget {
                name: "missing".to_string(),
                specifier: "missing.mec".to_string(),
            }],
            vec![recursive_root_folder()],
            module_options(),
        )
        .unwrap();

        let mut sink = RecordingEventSink::default();

        session.emit_initial_events(&mut sink).unwrap();

        assert!(sink.events.iter().any(|event| {
            matches!(
              &event.kind,
              RuntimeEventKind::SourceResolveFailed { specifier, .. }
                if specifier == "missing"
            )
        }));
    }

    #[test]
    fn server_workspace_session_poll_and_emit_without_events_emits_nothing() {
        let root = setup_session_root();
        std::fs::write(root.join("main.mec"), "result := true\n").unwrap();

        let mut session = ServerWorkspaceSession::open(
            &root,
            vec![main_target()],
            vec![recursive_root_folder()],
            module_options(),
        )
        .unwrap();

        let mut sink = RecordingEventSink::default();

        let poll = session.poll_and_emit(module_options(), &mut sink).unwrap();

        assert!(poll.events.is_empty());
        assert!(poll.refresh.is_none());
        assert!(sink.events.is_empty());
    }

    #[test]
    fn server_workspace_session_emits_refresh_events_for_changed_file() {
        let root = setup_session_root();
        std::fs::write(root.join("main.mec"), "result := false\n").unwrap();

        let mut session = ServerWorkspaceSession::open(
            &root,
            vec![main_target()],
            vec![recursive_root_folder()],
            module_options(),
        )
        .unwrap();

        std::fs::write(root.join("main.mec"), "result := true\n").unwrap();

        let refresh = session.refresh(module_options()).unwrap();

        let poll = RuntimeWorkspaceWatchPoll {
            events: Vec::new(),
            refresh: Some(refresh),
        };

        let mut sink = RecordingEventSink::default();

        session.emit_watch_poll_events(&poll, &mut sink).unwrap();

        assert!(sink.events.iter().any(|event| {
            matches!(
              &event.kind,
              RuntimeEventKind::SourceChanged { canonical_uri }
                if canonical_uri.ends_with("main.mec")
            )
        }));

        assert!(sink.events.iter().any(|event| {
            matches!(
              &event.kind,
              RuntimeEventKind::SourceReloaded { canonical_uri }
                if canonical_uri.ends_with("main.mec")
            )
        }));
    }
}

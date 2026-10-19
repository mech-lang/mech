//! Real filesystem watch polls through the production registry publication path.
use super::*;
use std::{
    cell::Cell,
    fs, thread,
    time::{SystemTime, UNIX_EPOCH},
};

thread_local! {
    pub(super) static WORKSPACE_VIEW_REBUILDS: Cell<u64> = const { Cell::new(0) };
}

struct Fixture {
    root: PathBuf,
    session: Arc<Mutex<ServerWorkspaceSession>>,
    registry: Arc<RwLock<ServerSourceRegistry>>,
    events: Arc<RwLock<Vec<RuntimeEvent>>>,
}
impl Fixture {
    fn new() -> Self {
        WORKSPACE_VIEW_REBUILDS.with(|count| count.set(0));
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "mech-serve-static-refresh-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        fs::write(root.join("main.mec"), "answer := 42\nanswer\n").unwrap();
        let session = ServerWorkspaceSession::open(
            &root,
            vec![RuntimeWorkspaceTarget {
                name: "main".into(),
                specifier: "main.mec".into(),
            }],
            vec![RuntimeWorkspaceFolder {
                specifier: ".".into(),
                recursive: true,
            }],
            module_options(),
        )
        .unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.set_document_controller(
            Some(include_str!("../../include/document.js").to_string()),
            Some("include/index.html".to_string()),
        );
        registry
            .sync_workspace_snapshot(
                &root,
                session.snapshot().unwrap(),
                "",
                include_str!("../../include/index.html"),
                &[],
            )
            .unwrap();
        assert_eq!(rebuilds(), 1);
        Self {
            root,
            session: Arc::new(Mutex::new(session)),
            registry: Arc::new(RwLock::new(registry)),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn pump_until(
        &self,
        description: &str,
        allow_error: bool,
        ready: impl Fn(&Self) -> bool,
    ) -> Vec<String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut errors = Vec::new();
        loop {
            let result = poll_workspace_once(
                &self.session,
                &self.registry,
                &self.events,
                &self.root,
                &HtmlStyleSheets::default(),
                include_str!("../../include/index.html"),
                &[],
                None,
                None,
            );
            if let Err(error) = result {
                assert!(allow_error, "{description}: {error:?}");
                errors.push(format!("{error:?}"));
            }
            if ready(self) {
                return errors;
            }
            assert!(
                Instant::now() < deadline,
                "filesystem watch did not observe {description}"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn snapshot(&self) -> RuntimeWorkspaceSnapshot {
        self.session.lock().unwrap().snapshot().unwrap().clone()
    }

    fn route(&self, path: &str) -> Option<Vec<u8>> {
        self.registry
            .read()
            .unwrap()
            .get_route(path)
            .map(|asset| asset.bytes)
    }

    fn code_source(&self) -> Option<String> {
        self.route("/code/main.mec").map(|bytes| {
            mech_runtime::BrowserDocumentPayload::decode(std::str::from_utf8(&bytes).unwrap())
                .unwrap()
                .source()
                .to_string()
        })
    }

    fn diagnostics(&self) -> bool {
        self.events.read().unwrap().iter().any(|event| {
            matches!(
                &event.kind,
                mech_runtime::RuntimeEventKind::ModuleCompileFailed { .. }
                    | mech_runtime::RuntimeEventKind::SourceResolveFailed { .. }
            )
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Only this test-created, exact directory is removed.
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn rebuilds() -> u64 {
    WORKSPACE_VIEW_REBUILDS.with(Cell::get)
}

#[test]
fn static_index_watch_reloads_without_reparsing_or_recompiling_unchanged_mech() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let before_source = before.sources.values().next().unwrap();
    let before_document = before_source.source_document.as_ref().unwrap();
    let code = fixture.route("/code/main.mec").unwrap();
    let html = fixture.route("/main.mec").unwrap();
    let manifest = fixture.route("/_mech/project-sources.json").unwrap();
    let provenance = fixture.registry.read().unwrap().source_provenance.clone();
    let backing = fixture.registry.read().unwrap().code_sources["main.mec"]
        .backing_paths
        .clone();

    // Reproduce both writes around the smoke script's preflight: creating the
    // user index and then injecting the harness into that same static asset.
    for text in [
        "<!doctype html><p>first index</p>",
        "<!doctype html><p>injected harness</p>",
    ] {
        fs::write(fixture.root.join("index.html"), text).unwrap();
        fixture.pump_until("static index edit", false, |fixture| {
            fixture.route("/") == Some(text.as_bytes().to_vec())
        });
        assert_eq!(
            rebuilds(),
            1,
            "static updates must never enter the actual rebuild routine"
        );
        let after = fixture.snapshot();
        let source = after.sources.get(&before_source.canonical_uri).unwrap();
        assert_eq!(source.module_version, before_source.module_version);
        assert!(std::ptr::eq(
            source.source_document.as_ref().unwrap().snapshot(),
            before_document.snapshot()
        ));
        assert_eq!(fixture.route("/code/main.mec").unwrap(), code);
        assert_eq!(fixture.route("/main.mec").unwrap(), html);
        assert_eq!(
            fixture.route("/_mech/project-sources.json").unwrap(),
            manifest
        );
        let registry = fixture.registry.read().unwrap();
        assert_eq!(registry.source_provenance, provenance);
        assert_eq!(registry.code_sources["main.mec"].backing_paths, backing);
    }
    fs::remove_file(fixture.root.join("index.html")).unwrap();
    fixture.pump_until("static index deletion", false, |fixture| {
        !fixture
            .registry
            .read()
            .unwrap()
            .user_assets
            .contains("index.html")
            && fixture.route("/") == Some(html.clone())
    });
    assert_eq!(rebuilds(), 1);
    assert_eq!(
        fixture.code_source().as_deref(),
        Some("answer := 42\nanswer\n")
    );
    assert!(!fixture.diagnostics());
}

#[test]
fn actual_language_edit_rebuilds_and_invalid_source_reports_diagnostics_then_recovers() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let before_source = before.sources.values().next().unwrap();
    let changed = "answer := 43\nanswer\n";
    fs::write(fixture.root.join("main.mec"), changed).unwrap();
    fixture.pump_until("language edit", false, |fixture| {
        fixture.code_source().as_deref() == Some(changed)
    });
    assert_eq!(rebuilds(), 2);
    let after = fixture.snapshot();
    let source = after.sources.get(&before_source.canonical_uri).unwrap();
    assert_ne!(source.module_version, before_source.module_version);
    assert!(!std::ptr::eq(
        source.source_document.as_ref().unwrap().snapshot(),
        before_source.source_document.as_ref().unwrap().snapshot()
    ));
    let previous_code = fixture.route("/code/main.mec");
    fs::write(fixture.root.join("main.mec"), "answer := {{{\n").unwrap();
    let errors = fixture.pump_until("invalid language edit", true, |fixture| {
        fixture.diagnostics() && rebuilds() > 2
    });
    if errors.is_empty() {
        assert!(
            fixture.route("/code/main.mec").is_none(),
            "an invalid published snapshot must not advertise an old executable code response"
        );
    } else {
        assert_eq!(
            fixture.route("/code/main.mec"),
            previous_code,
            "a failed rebuild must retain the atomically published registry"
        );
    }
    let recovered = "answer := 44\nanswer\n";
    fs::write(fixture.root.join("main.mec"), recovered).unwrap();
    fixture.pump_until("language recovery", false, |fixture| {
        fixture.code_source().as_deref() == Some(recovered)
    });
    assert!(rebuilds() >= 4);
}

#[test]
fn actual_language_deletion_rebuilds_and_removes_source_and_code_routes() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root.join("main.mec")).unwrap();
    fixture.pump_until("language deletion", false, |fixture| {
        rebuilds() > 1
            && fixture.route("/source/main.mec").is_none()
            && fixture.route("/code/main.mec").is_none()
    });
    assert!(fixture.route("/main.mec").is_none());
    let manifest = fixture.route("/_mech/project-sources.json").unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert!(manifest["sources"].as_array().unwrap().is_empty());
    assert!(
        fixture.diagnostics(),
        "the removed run target remains a reported failure"
    );
}

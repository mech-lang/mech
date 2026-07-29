use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use mech_core::{MResult, MechSourceCode, Value};
use mech_runtime::{
    FS_IMPORT, FS_READ, FS_RESOLVE, FileSourceResolver, HostFilesystemAuthority,
    InMemorySourceResolver, MechRuntime, ResolvedSource, RuntimeBuilder, RuntimeHealth,
    RuntimeValueSnapshot, SequentialIdGenerator, SharedCapabilityKernel, SourceKind, SourceRequest,
    SourceResolver, module_id,
};

use super::{MechRepl, ReplCommand, runtime_repl_load_request};

const TEST_FS_SUBJECT: &str = "test://runtime-repl-load";

#[derive(Debug)]
struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "mech-runtime-repl-load-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self {
            path: path.canonicalize().unwrap(),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug)]
struct RecordingResolver {
    inner: Box<dyn SourceResolver>,
    requests: Arc<Mutex<Vec<SourceRequest>>>,
}

impl RecordingResolver {
    fn new(inner: impl SourceResolver + 'static) -> (Self, Arc<Mutex<Vec<SourceRequest>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                inner: Box::new(inner),
                requests: requests.clone(),
            },
            requests,
        )
    }
}

impl SourceResolver for RecordingResolver {
    fn resolve(&self, request: &SourceRequest) -> MResult<Option<ResolvedSource>> {
        self.requests.lock().unwrap().push(request.clone());
        self.inner.resolve(request)
    }
}

fn runtime_with_resolver(resolver: impl SourceResolver + 'static) -> MechRuntime {
    RuntimeBuilder::new()
        .source_resolver(resolver)
        .build()
        .unwrap()
}

fn file_resolver_with_grant(
    root: &Path,
    granted_path: &Path,
    recursive: bool,
    operations: impl IntoIterator<Item = &'static str>,
) -> FileSourceResolver {
    let mut ids = SequentialIdGenerator::new();
    let mut authority =
        HostFilesystemAuthority::new(TEST_FS_SUBJECT, SharedCapabilityKernel::new());
    authority
        .grant_path(&mut ids, granted_path, recursive, operations)
        .unwrap();
    FileSourceResolver::new(root).with_capabilities(authority.kernel().clone(), TEST_FS_SUBJECT)
}

fn file_resolver_without_grants(root: &Path) -> FileSourceResolver {
    FileSourceResolver::new(root).with_capabilities(SharedCapabilityKernel::new(), TEST_FS_SUBJECT)
}

fn file_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        let path = if text.starts_with('/') {
            text
        } else {
            format!("/{text}")
        };
        return format!("file://{path}");
    }
    #[cfg(not(windows))]
    {
        format!("file://{text}")
    }
}

fn runtime(repl: &MechRepl) -> &MechRuntime {
    repl.runtime
        .as_ref()
        .expect("runtime-backed REPL must retain its runtime")
}

fn assert_f64(value: RuntimeValueSnapshot, expected: f64, label: &str) {
    match value.into_value() {
        Value::F64(value) => {
            assert_eq!(*value.borrow(), expected, "{label}");
        }
        Value::MutableReference(value) => match &*value.borrow() {
            Value::F64(value) => {
                assert_eq!(*value.borrow(), expected, "{label}");
            }
            other => {
                panic!("expected f64 mutable reference from {label}, got {other:?}",);
            }
        },
        other => panic!("expected f64 from {label}, got {other:?}"),
    }
}

#[test]
fn runtime_repl_load_uses_configured_source_resolver() {
    let mut memory = InMemorySourceResolver::new();
    memory
        .insert_source(
            "memory:resolver-owned",
            ResolvedSource::new(
                "resolver-owned.mec",
                "memory:resolver-owned",
                MechSourceCode::String(
                    "resolver-owned-value := 42\nresolver-owned-value\n".to_string(),
                ),
            )
            .with_kind(SourceKind::Mech),
        )
        .unwrap();
    let (resolver, requests) = RecordingResolver::new(memory);
    let mut repl = MechRepl::from_runtime(runtime_with_resolver(resolver));

    let output = repl
        .execute_repl_command(ReplCommand::Load(vec!["memory:resolver-owned".to_string()]))
        .unwrap();

    assert!(output.contains("42"));
    assert_eq!(
        requests.lock().unwrap().as_slice(),
        [SourceRequest::new("memory:resolver-owned")],
    );
    assert_f64(
        runtime(&repl)
            .root_symbol_value("resolver-owned-value")
            .unwrap(),
        42.0,
        "resolver-owned retained symbol",
    );
}

#[test]
fn runtime_repl_load_resolves_relative_module_imports() {
    let root = TestRoot::new("relative-import");
    let main = root.path().join("main.mec");
    let dependency = root.path().join("dep.mec");
    std::fs::write(&main, "+> ./dep.mec\nanswer := dep/value + 1\nanswer\n").unwrap();
    std::fs::write(&dependency, "value := 41\n<+ value\n").unwrap();
    let resolver = file_resolver_with_grant(
        root.path(),
        root.path(),
        true,
        [FS_RESOLVE, FS_READ, FS_IMPORT],
    );
    let (resolver, requests) = RecordingResolver::new(resolver);
    let mut repl = MechRepl::from_runtime(runtime_with_resolver(resolver));

    let output = repl
        .execute_repl_command(ReplCommand::Load(vec![main.to_string_lossy().into_owned()]))
        .unwrap();

    assert!(output.contains("42"));
    assert_f64(
        runtime(&repl).root_symbol_value("answer").unwrap(),
        42.0,
        "relative-import answer",
    );
    let requests = requests.lock().unwrap();
    let dependency_request = requests
        .iter()
        .find(|request| request.specifier == "./dep.mec")
        .expect("dependency must be resolved through the configured resolver");
    assert_eq!(
        dependency_request.referrer.as_deref(),
        Some(file_uri(&main).as_str()),
    );
    assert_eq!(runtime(&repl).runtime_health(), RuntimeHealth::Healthy);
}

#[test]
fn runtime_repl_load_denies_ungranted_file() {
    let root = TestRoot::new("denied-root");
    let denied = root.path().join("denied.mec");
    std::fs::write(&denied, "denied-symbol := 42\ndenied-symbol\n").unwrap();
    let mut repl = MechRepl::from_runtime(runtime_with_resolver(file_resolver_without_grants(
        root.path(),
    )));

    let error = repl
        .execute_repl_command(ReplCommand::Load(vec![
            denied.to_string_lossy().into_owned(),
        ]))
        .unwrap_err();

    assert_eq!(error.kind_name(), "CapabilityDenied");
    let message = error.full_chain_message();
    assert!(
        message.contains(FS_RESOLVE) || message.contains(&denied.to_string_lossy().to_string()),
        "denial did not identify the operation or path: {message}",
    );
    assert!(runtime(&repl).root_symbol_value("denied-symbol").is_err());
    assert!(
        runtime(&repl)
            .get_module(module_id(&file_uri(&denied)))
            .unwrap()
            .is_none()
    );
    assert_eq!(runtime(&repl).runtime_health(), RuntimeHealth::Healthy);
}

#[test]
fn runtime_repl_load_denies_ungranted_dependency_without_partial_state() {
    let root = TestRoot::new("denied-dependency");
    let main = root.path().join("main.mec");
    let dependency = root.path().join("dep.mec");
    std::fs::write(
        &main,
        "+> ./dep.mec\nroot-started := 1\n\
     answer := dep/value + 1\nanswer\n",
    )
    .unwrap();
    std::fs::write(&dependency, "value := 41\n<+ value\n").unwrap();
    let resolver = file_resolver_with_grant(root.path(), &main, false, [FS_RESOLVE, FS_READ]);
    let (resolver, requests) = RecordingResolver::new(resolver);
    let mut repl = MechRepl::from_runtime(runtime_with_resolver(resolver));

    let error = repl
        .execute_repl_command(ReplCommand::Load(vec![main.to_string_lossy().into_owned()]))
        .unwrap_err();

    assert_eq!(error.kind_name(), "CapabilityDenied");
    let requests = requests.lock().unwrap();
    assert!(
        requests
            .iter()
            .any(|request| request.specifier == main.to_string_lossy())
    );
    assert!(
        requests
            .iter()
            .any(|request| request.specifier == "./dep.mec"
                && request.referrer.as_deref() == Some(file_uri(&main).as_str()))
    );
    drop(requests);

    let retained = runtime(&repl);
    for symbol in ["root-started", "answer", "dep/value", "value"] {
        assert!(
            retained.root_symbol_value(symbol).is_err(),
            "denied dependency retained symbol {symbol}",
        );
    }
    for path in [&main, &dependency] {
        assert!(
            retained
                .get_module(module_id(&file_uri(path)))
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(retained.runtime_health(), RuntimeHealth::Healthy);

    let recovery = repl
        .execute_repl_command(ReplCommand::Code(vec![(
            "recovery".to_string(),
            MechSourceCode::String("inline-recovery := 7\ninline-recovery\n".to_string()),
        )]))
        .unwrap();
    assert!(recovery.contains('7'));
    assert_f64(
        runtime(&repl).root_symbol_value("inline-recovery").unwrap(),
        7.0,
        "inline recovery after denied dependency",
    );
}

#[test]
fn runtime_repl_load_preserves_multiple_path_order() {
    let root = TestRoot::new("multiple-order");
    let first = root.path().join("first.mec");
    let second = root.path().join("second.mec");
    std::fs::write(&first, "first := 41\n").unwrap();
    std::fs::write(&second, "answer := first + 1\nanswer\n").unwrap();
    let resolver = file_resolver_with_grant(
        root.path(),
        root.path(),
        true,
        [FS_RESOLVE, FS_READ, FS_IMPORT],
    );
    let mut repl = MechRepl::from_runtime(runtime_with_resolver(resolver));

    let output = repl
        .execute_repl_command(ReplCommand::Load(vec![
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ]))
        .unwrap();

    assert!(output.contains("42"));
    assert_f64(
        runtime(&repl).root_symbol_value("answer").unwrap(),
        42.0,
        "ordered multiple-load answer",
    );
}

#[test]
fn runtime_repl_load_request_preserves_explicit_source_schemes() {
    for specifier in [
        "file:///tmp/main.mec",
        "fs://project/main.mec",
        "pkg:plot@1.2.0",
        "mech:std/math",
        "workspace:current-buffer",
        "memory:main",
    ] {
        assert_eq!(
            runtime_repl_load_request(specifier).unwrap().specifier,
            specifier,
        );
    }

    let relative = Path::new("relative").join("main.mec");
    assert_eq!(
        runtime_repl_load_request(&relative.to_string_lossy(),)
            .unwrap()
            .specifier,
        std::env::current_dir()
            .unwrap()
            .join(relative)
            .to_string_lossy(),
    );

    #[cfg(windows)]
    assert_eq!(
        runtime_repl_load_request(r"C:\project\main.mec")
            .unwrap()
            .specifier,
        r"C:\project\main.mec",
    );
}

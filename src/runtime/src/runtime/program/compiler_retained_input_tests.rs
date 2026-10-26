use super::{ProgramCompiler, RetainedCanonicalCompilation, RetainedSourceCollector};
use crate::{
    FileSourceResolver, InMemorySourceResolver, ModuleBuildOptions, ResolvedSource, RuntimeBuilder,
    SourceRequest, SourceResolver,
};
use mech_core::{FunctionCatalog, MResult};
use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug)]
struct SharedResolver {
    sources: Arc<Mutex<InMemorySourceResolver>>,
    reads: Arc<AtomicUsize>,
    late_dep_reads: AtomicUsize,
    detached: bool,
}
impl SourceResolver for SharedResolver {
    fn resolve(&self, request: &SourceRequest) -> MResult<Option<ResolvedSource>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        // Resolve the namespace only in the existing detached-import pass.
        if self.detached
            && request.specifier == "dep"
            && self.late_dep_reads.fetch_add(1, Ordering::SeqCst) == 0
        {
            return Ok(None);
        }
        self.sources.lock().unwrap().resolve(request)
    }
}
fn compiler(
    sources: &Arc<Mutex<InMemorySourceResolver>>,
    reads: &Arc<AtomicUsize>,
    catalog: &Arc<FunctionCatalog>,
    detached: bool,
) -> ProgramCompiler {
    RuntimeBuilder::new()
        .function_catalog(Arc::clone(catalog))
        .source_resolver(SharedResolver {
            sources: Arc::clone(sources),
            reads: Arc::clone(reads),
            late_dep_reads: AtomicUsize::new(0),
            detached,
        })
        .build_compiler()
        .unwrap()
}
fn options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new("test", "v0.4", "native", &[], &[])
}
fn assert_decoded_identity(input: &RetainedCanonicalCompilation) {
    let decoded =
        mech_engine::decode_program_artifact_bytecode_v1(input.product().bytecode()).unwrap();
    assert_eq!(decoded.revision(), input.product().artifact().revision());
}

#[test]
fn retained_compilation_keeps_exact_ordered_and_detached_owners_after_resolver_replacement() {
    for detached in [false, true] {
        let first = "+> dep\nanswer := dep/value\nanswer\n";
        let dep = "+> ./leaf.mec\nvalue := leaf/value\n<+ value\n";
        let leaf = "value := \"retained unicode é\"\n<+ value\n";
        let mut resolver = InMemorySourceResolver::new();
        for (name, text) in [
            ("main.mec", first),
            ("other.mec", "other := \"second\"\nother\n"),
            ("dep", dep),
            ("leaf.mec", leaf),
        ] {
            resolver.insert_canonical_string(name, text).unwrap();
        }
        let expected = ["main.mec", "other.mec", "dep", "leaf.mec"].map(|name| {
            resolver
                .resolve(&SourceRequest::new(name))
                .unwrap()
                .unwrap()
                .source_document()
                .unwrap()
                .clone()
        });
        let sources = Arc::new(Mutex::new(resolver));
        let catalog = mech_stdlib::source_catalog();
        let requests = [
            SourceRequest::new("main.mec"),
            SourceRequest::new("other.mec"),
        ];
        let ordinary_reads = Arc::new(AtomicUsize::new(0));
        let ordinary = compiler(&sources, &ordinary_reads, &catalog, detached)
            .compile_canonical_roots(&requests, options())
            .unwrap();
        let reads = Arc::new(AtomicUsize::new(0));
        let mut compiler = compiler(&sources, &reads, &catalog, detached);
        let input = compiler
            .compile_canonical_roots_retained(&requests, options())
            .unwrap();
        drop(compiler);
        let before_reads = reads.load(Ordering::SeqCst);
        assert_eq!(before_reads, ordinary_reads.load(Ordering::SeqCst));
        assert_eq!(input.product().bytecode(), ordinary.bytecode());
        assert_eq!(
            input.product().source_dependencies(),
            ordinary.source_dependencies()
        );
        sources
            .lock()
            .unwrap()
            .insert_canonical_string("dep", "value := \"changed\"\n<+ value\n")
            .unwrap();
        *sources.lock().unwrap() = InMemorySourceResolver::new();
        assert!(Arc::ptr_eq(input.sources().function_catalog(), &catalog));
        assert!(!Arc::ptr_eq(
            input.sources().function_catalog(),
            &Arc::new(FunctionCatalog::empty())
        ));
        let roots = input.sources().roots();
        let dependencies = input.sources().dependencies();
        assert_eq!(
            roots.iter().map(|s| s.canonical_uri()).collect::<Vec<_>>(),
            ["memory:main.mec", "memory:other.mec"]
        );
        assert_eq!(
            dependencies
                .iter()
                .map(|s| s.canonical_uri())
                .collect::<Vec<_>>(),
            ["memory:dep", "memory:leaf.mec"]
        );
        for (source, expected) in roots.iter().chain(dependencies).zip(&expected) {
            assert!(std::ptr::eq(
                source.document().snapshot(),
                expected.snapshot()
            ));
            assert_eq!(
                source.document().source().revision(),
                expected.source().revision()
            );
            assert_eq!(
                source.document().nominal_origin(),
                expected.nominal_origin()
            );
            assert_eq!(
                source.document().nominal_package_id(),
                expected.nominal_package_id()
            );
        }
        assert_eq!(
            dependencies[0].document().source().to_contiguous_string(),
            dep
        );
        assert_eq!(
            dependencies[1].document().source().to_contiguous_string(),
            leaf
        );
        assert_decoded_identity(&input);
        assert_eq!(reads.load(Ordering::SeqCst), before_reads);
    }
}

#[test]
fn collector_keeps_distinct_nominal_owners_of_the_same_snapshot() {
    let origin = |name: &str| mech_core::CanonicalNominalPath::new([name.to_owned()]).unwrap();
    let first = super::super::retained_compiler_document("1u64\n")
        .unwrap()
        .with_nominal_origin(origin("owner-a"))
        .with_nominal_package_id("package-a");
    let other_origin = first.clone().with_nominal_origin(origin("owner-b"));
    let other_package = first.clone().with_nominal_package_id("package-b");
    let compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let collector = RefCell::new(RetainedSourceCollector::default());
    let mut view = compiler.view();
    view.retained_sources = Some(&collector);
    for document in [&first, &first, &other_origin, &other_package] {
        assert!(std::ptr::eq(first.snapshot(), document.snapshot()));
        view.retain_compilation_source("memory:same.mec", document, false);
    }
    drop(view);
    let retained = collector.into_inner().dependencies;
    assert_eq!(retained.len(), 3);
    for (actual, expected) in retained.iter().zip([&first, &other_origin, &other_package]) {
        assert!(std::ptr::eq(
            actual.document().snapshot(),
            expected.snapshot()
        ));
        assert_eq!(
            actual.document().nominal_origin(),
            expected.nominal_origin()
        );
        assert_eq!(
            actual.document().nominal_package_id(),
            expected.nominal_package_id()
        );
    }
}

struct TestDirectory(PathBuf);
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn retained_compilation_survives_file_mutation_and_deletion_without_recompiling() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = TestDirectory(std::env::temp_dir().join(format!(
        "mech-retained-input-{}-{unique}",
        std::process::id()
    )));
    fs::create_dir(&dir.0).unwrap();
    let root = dir.0.join("main.mec");
    let dependency = dir.0.join("dep.mec");
    let root_text = "+> ./dep.mec\nanswer := dep/value\nanswer\n";
    let dep_text = "value := \"original\"\n<+ value\n";
    fs::write(&root, root_text).unwrap();
    fs::write(&dependency, dep_text).unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(FileSourceResolver::new(&dir.0))
        .build_compiler()
        .unwrap();
    let requests = [SourceRequest::from_filesystem_path(&root).unwrap()];
    let original = compiler
        .compile_canonical_roots(&requests, options())
        .unwrap();
    let input = compiler
        .compile_canonical_roots_retained(&requests, options())
        .unwrap();
    assert_eq!(original.bytecode(), input.product().bytecode());
    assert_eq!(
        original.source_dependencies(),
        input.product().source_dependencies()
    );
    assert_eq!(
        original.instruction_type_binding_requirements(),
        input.product().instruction_type_binding_requirements()
    );
    fs::write(&dependency, "invalid {{{").unwrap();
    fs::remove_file(&root).unwrap();
    fs::remove_file(&dependency).unwrap();
    assert_eq!(
        input.sources().roots()[0]
            .document()
            .source()
            .to_contiguous_string(),
        root_text
    );
    assert_eq!(
        input.sources().dependencies()[0]
            .document()
            .source()
            .to_contiguous_string(),
        dep_text
    );
    assert_decoded_identity(&input);
    assert!(
        compiler
            .compile_canonical_roots_retained(&requests, options())
            .is_err()
    );
}

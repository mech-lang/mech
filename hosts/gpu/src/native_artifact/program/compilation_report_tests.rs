use super::{NativeCompilationReport, NativeCompilerConfiguration};
use crate::native_artifact::emit_compact_closed_object_with_configuration_for_diagnostic as compile;
use mech_core::FunctionCatalog;
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn input() -> (ProgramArtifact, std::sync::Arc<FunctionCatalog>) {
    let source = "value := word<u64>\ndelta := delta<u64>\nsum := value + delta\nsum\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(173), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let catalog = mech_stdlib::source_catalog();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    (artifact, catalog)
}

#[test]
fn report_and_compile_workers_preserve_object_and_descriptor() {
    let (artifact, catalog) = input();
    for observe in [false, true] {
        let base = NativeCompilerConfiguration::capture([]);
        let expected = compile(&artifact, &catalog, &base, observe, None).unwrap();
        for workers in ["0", "1", "2", "4"] {
            let config = NativeCompilerConfiguration::capture([(
                "MECH_COMPACT_COMPILE_THREADS".into(),
                workers.into(),
            )]);
            let mut report = NativeCompilationReport::default();
            let actual = compile(&artifact, &catalog, &config, observe, Some(&mut report)).unwrap();
            assert_eq!(actual, expected);
            let json = report.json(&actual.1);
            assert_eq!(json["phase_observer"], observe);
            assert_eq!(json["compile_workers"], workers.parse::<usize>().unwrap());
            assert_eq!(json["object_sizes"]["object_file_bytes"], actual.0.len());
            assert!(
                json["object_sizes"]["function_body_bytes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert_eq!(json["object_attempts"][0]["status"], "emitted");
            assert!(json["stages"].as_array().unwrap().len() >= 6);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[test]
fn distinct_alignments_are_isolated_in_alternating_and_concurrent_compiles() {
    let (artifact, catalog) = input();
    let configuration = |alignment: &str| {
        NativeCompilerConfiguration::capture([
            (
                "MECH_COMPACT_FUNCTION_ALIGNMENT_LOG2".into(),
                alignment.into(),
            ),
            ("MECH_COMPACT_COMPILE_THREADS".into(), "2".into()),
        ])
    };
    let a = configuration("4");
    let b = configuration("7");
    let reference_a = compile(&artifact, &catalog, &a, false, None).unwrap();
    let reference_b = compile(&artifact, &catalog, &b, false, None).unwrap();
    assert_ne!(reference_a.0, reference_b.0);
    assert_eq!(
        compile(&artifact, &catalog, &a, false, None).unwrap(),
        reference_a
    );
    assert_eq!(
        compile(&artifact, &catalog, &b, false, None).unwrap(),
        reference_b
    );
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let run = |config: &NativeCompilerConfiguration| {
            let (artifact, catalog) = input();
            barrier.wait();
            compile(&artifact, &catalog, config, false, None).unwrap()
        };
        let first = scope.spawn(move || run(&a));
        let second = scope.spawn(move || run(&b));
        assert_eq!(first.join().unwrap(), reference_a);
        assert_eq!(second.join().unwrap(), reference_b);
    });
    let malformed = configuration("invalid");
    assert!(
        compile(&artifact, &catalog, &malformed, false, None)
            .unwrap_err()
            .to_string()
            .contains("function alignment")
    );
    assert!(compile(&artifact, &catalog, &malformed, true, None).is_ok());
}

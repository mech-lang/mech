use crate::{
    MechRuntime, ModuleBuildOptions, ModuleBuilder, ResolvedSource, RuntimeConfig, SourceDocument,
    SourceKind,
};
use mech_core::MechSourceCode;
use mech_syntax::document::{ParseConfig, Revision};

#[test]
fn direct_source_build_retains_one_canonical_revision_through_the_store() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let source = "  value := 41\r\n";
    let version = runtime
        .put_source_module(
            "main.mec",
            "memory:main.mec",
            source,
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .unwrap();
    let (_, record) = runtime
        .workspace_module_records(version)
        .unwrap()
        .expect("stored module revision");
    let document = record
        .source_document
        .expect("module store retains the canonical source revision");
    assert_eq!(document.source().to_contiguous_string(), source);
    assert_eq!(
        document.source().document().0,
        mech_core::hash_str("memory:main.mec")
    );
    assert_eq!(document.source().revision().0, 0);
    assert!(document.is_strictly_clean());
}

#[test]
fn successful_and_rejected_runtime_replacements_preserve_revision_history() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let options = ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]);
    let first = runtime
        .put_canonical_source_module("main.mec", "memory:main.mec", "value := 1\n", options)
        .unwrap();
    let second = runtime
        .put_canonical_source_module("main.mec", "memory:main.mec", "value := 2\n", options)
        .unwrap();
    let first_document = runtime
        .workspace_module_records(first)
        .unwrap()
        .unwrap()
        .1
        .source_document
        .unwrap();
    let second_document = runtime
        .workspace_module_records(second)
        .unwrap()
        .unwrap()
        .1
        .source_document
        .unwrap();
    assert_eq!(
        first_document.source().document(),
        second_document.source().document()
    );
    assert_eq!(first_document.source().revision(), Revision(0));
    assert_eq!(second_document.source().revision(), Revision(1));
    assert_eq!(
        first_document.source().to_contiguous_string(),
        "value := 1\n"
    );
    assert_eq!(
        second_document.source().to_contiguous_string(),
        "value := 2\n"
    );

    assert!(
        runtime
            .put_canonical_source_module("main.mec", "memory:main.mec", "value := [\n", options)
            .is_err()
    );
    let third = runtime
        .put_canonical_source_module("main.mec", "memory:main.mec", "value := 3\n", options)
        .unwrap();
    let third_document = runtime
        .workspace_module_records(third)
        .unwrap()
        .unwrap()
        .1
        .source_document
        .unwrap();
    assert_eq!(third_document.source().revision(), Revision(2));
}

#[test]
fn invalid_canonical_authority_never_falls_back_to_an_available_legacy_tree() {
    let source = include_str!("../../../../../../examples/gpu-particles/particles.mec");
    let document = SourceDocument::parse_resolved(
        "memory:main.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(!document.is_strictly_clean());
    let resolved = ResolvedSource::new(
        "main.mec",
        "memory:main.mec",
        MechSourceCode::String(source.to_owned()),
    )
    .with_kind(SourceKind::Mech)
    .with_source_document(document)
    .unwrap()
    .with_syntax_tree(mech_syntax::parser::parse(source.trim()).unwrap());
    assert!(resolved.syntax_tree.is_some());
    assert!(resolved.canonical_document_index().is_err());

    let record = ModuleBuilder::new()
        .build_resolved_source(resolved, "test", "v0.4", "native", &[], &[], &[])
        .unwrap();
    assert!(record.syntax_tree.is_some());
    assert!(record.canonical_document_index().is_err());
}

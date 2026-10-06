#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::{
    CanonicalNominalPath, FunctionCatalog, FunctionExposure, MechSourceCode, ReactiveInstanceId,
};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, SourceSemanticError};
use mech_runtime::resolver::InMemorySourceResolver;
use mech_runtime::{
    ModuleBuildOptions, ResolvedSource, RuntimeBuilder, RuntimeValueSnapshot, SourceDocument,
    SourceKind, SourceRequest,
};
use mech_syntax::document::{ParseConfig, Revision};
use std::collections::BTreeSet;
use std::sync::Arc;

fn document(source: &str) -> SourceDocument {
    let document = SourceDocument::parse_resolved(
        "visibility.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "{source}: {:?}",
        document.snapshot().diagnostics
    );
    document
}

#[test]
fn product_compiler_uses_the_retained_defining_origin_for_enums() {
    let source = "<event> := :idle | :busy\nvalue<event> := :idle\nvalue\n";
    let origin = CanonicalNominalPath::new(vec!["sample-package".to_owned()]).unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    assert!(compiler.compile_document(&document(source)).is_err());
    let compiled = compiler
        .compile_document(&document(source).with_nominal_origin(origin))
        .expect("product compilation receives the defining package");
    assert!(!compiled.bytecode().is_empty());
}

#[test]
fn recursive_graph_imports_reject_distinct_sources_of_one_enum_path() {
    let source = "<event> := :idle | :busy\nvalue<event> := :idle\n<+ value\n";
    let origin = CanonicalNominalPath::new(vec!["shared".to_owned()]).unwrap();
    let dependency = |name: &str, owner: &str| {
        ResolvedSource::new(
            name,
            format!("memory:app/{name}"),
            MechSourceCode::String(source.to_owned()),
        )
        .with_kind(SourceKind::Mech)
        .retain_source_document(Revision(0), ParseConfig::default())
        .unwrap()
        .with_nominal_origin(origin.clone())
        .with_nominal_package_id(owner)
    };
    for second_owner in ["package-a", "package-b"] {
        let resolver = InMemorySourceResolver::new()
            .with_string(
                "app/main.mec",
                "+> ./a.mec\n+> ./b.mec\nvalue := 1\nvalue\n",
            )
            .with_source("app/a.mec", dependency("a.mec", "package-a"))
            .with_source("app/b.mec", dependency("b.mec", second_owner));
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .source_resolver(resolver)
            .build_compiler()
            .unwrap();
        let error = compiler
            .compile_canonical_root(SourceRequest::new("app/main.mec"))
            .expect_err("distinct source files cannot own one nominal enum path");
        assert!(format!("{error:?}").contains("ambiguous-nominal-declaration-v1"));
    }
}

#[test]
fn ordered_root_and_detached_dependency_share_nominal_collision_registry() {
    let enum_source = "<event> := :idle | :busy\nvalue<event> := :idle\n<+ value\nvalue\n";
    let origin = CanonicalNominalPath::new(vec!["shared".to_owned()]).unwrap();
    let retained = |name: &str, owner: &str| {
        ResolvedSource::new(
            name,
            format!("memory:app/{name}"),
            MechSourceCode::String(enum_source.to_owned()),
        )
        .with_kind(SourceKind::Mech)
        .retain_source_document(Revision(0), ParseConfig::default())
        .unwrap()
        .with_nominal_origin(origin.clone())
        .with_nominal_package_id(owner)
    };
    let resolver = InMemorySourceResolver::new()
        .with_source("app/first.mec", retained("first.mec", "package-a"))
        .with_string(
            "app/second.mec",
            "+> ./dep.mec\nresult := dep/value\nresult\n",
        )
        .with_source("app/dep.mec", retained("dep.mec", "package-b"));
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let error = compiler
        .compile_canonical_roots(
            &[
                SourceRequest::new("app/first.mec"),
                SourceRequest::new("app/second.mec"),
            ],
            ModuleBuildOptions::new("test", "v0.4", "native", &[], &[]),
        )
        .expect_err("root and detached dependency cannot own one nominal path");
    assert!(format!("{error:?}").contains("ambiguous-nominal-declaration-v1"));
}

#[test]
fn non_nominal_dependency_keeps_text_only_freshness_with_package_metadata() {
    let dependency_source = "value := 1\n<+ value\nvalue\n";
    let dependency = ResolvedSource::new(
        "dep.mec",
        "memory:app/dep.mec",
        MechSourceCode::String(dependency_source.to_owned()),
    )
    .with_kind(SourceKind::Mech)
    .retain_source_document(Revision(0), ParseConfig::default())
    .unwrap()
    .with_nominal_origin(CanonicalNominalPath::new(vec!["package-a".to_owned()]).unwrap())
    .with_nominal_package_id("package-a");
    let resolver = InMemorySourceResolver::new()
        .with_string(
            "app/main.mec",
            "+> ./dep.mec\nresult := dep/value\nresult\n",
        )
        .with_source("app/dep.mec", dependency);
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_root(SourceRequest::new("app/main.mec"))
        .unwrap();
    assert_eq!(
        product.source_dependencies().get("memory:app/dep.mec"),
        Some(&mech_core::hash_str(dependency_source)),
    );
}

#[test]
fn imported_enum_uses_its_contextual_schema_for_qualified_payload_patterns() {
    let dependency_source =
        "<event> := :data<f64> | :idle\nvalue<event> := :data(3.0)\n<+ value\nvalue\n";
    document(dependency_source);
    let dependency = ResolvedSource::new(
        "dep.mec",
        "memory:app/dep.mec",
        MechSourceCode::String(dependency_source.to_owned()),
    )
    .with_kind(SourceKind::Mech)
    .retain_source_document(Revision(0), ParseConfig::default())
    .unwrap()
    .with_nominal_origin(CanonicalNominalPath::new(vec!["sample-package".to_owned()]).unwrap())
    .with_nominal_package_id("sample-package");
    let root_source =
        "+> ./dep.mec\nresult := dep/value?\n  | :event/data(x) => x\n  | * => 0.\nresult\n";
    document(root_source);
    let resolver = InMemorySourceResolver::new()
        .with_string("app/main.mec", root_source)
        .with_source("app/dep.mec", dependency);
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let compiled = compiler
        .compile_canonical_root(SourceRequest::new("app/main.mec"))
        .expect("an imported enum's exact schema admits its qualified payload pattern");
    assert!(!compiled.bytecode().is_empty());
}

#[test]
fn imported_enum_rejects_a_misspelled_qualified_payload_pattern() {
    let dependency_source =
        "<event> := :data<f64> | :idle\nvalue<event> := :data(3.0)\n<+ value\nvalue\n";
    let dependency = ResolvedSource::new(
        "dep.mec",
        "memory:app/dep.mec",
        MechSourceCode::String(dependency_source.to_owned()),
    )
    .with_kind(SourceKind::Mech)
    .retain_source_document(Revision(0), ParseConfig::default())
    .unwrap()
    .with_nominal_origin(CanonicalNominalPath::new(vec!["sample-package".to_owned()]).unwrap())
    .with_nominal_package_id("sample-package");
    let root_source =
        "+> ./dep.mec\nresult := dep/value?\n  | :evnet/data(x) => x\n  | * => 0.\nresult\n";
    document(root_source);
    let resolver = InMemorySourceResolver::new()
        .with_string("app/main.mec", root_source)
        .with_source("app/dep.mec", dependency);
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let error = compiler
        .compile_canonical_root(SourceRequest::new("app/main.mec"))
        .expect_err("a typo cannot borrow the imported enum schema");
    assert!(format!("{error:?}").contains("unknown enum qualifier"));
}

fn rejected(source: &str, catalog: Arc<FunctionCatalog>) -> SourceSemanticError {
    let document = document(source);
    CanonicalSourceFrontend
        .compile_document_with_catalog(&document.document(), catalog)
        .err()
        .unwrap_or_else(|| panic!("unexpectedly admitted: {source}"))
}

#[test]
fn configured_catalog_census_rejects_unbound_names_before_arguments() {
    let catalog = mech_stdlib::source_catalog();
    let names = catalog
        .all_exports()
        .map(|export| export.canonical_name.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names.len(),
        120,
        "review any change to the frozen catalog census"
    );
    let mut checked = 0;
    for name in names {
        if catalog.all_exports().any(|export| {
            export.canonical_name == name && export.exposure == FunctionExposure::Prelude
        }) {
            continue;
        }
        // Underscores are not source identifier characters; these names cannot
        // reach semantic lookup. Every lexically callable name must fail there.
        if name.contains('_') {
            continue;
        }
        let source = format!("{name}(unbound-argument)\n");
        let error = rejected(&source, Arc::clone(&catalog));
        assert_eq!(
            error.code, "source-semantics/unknown-function",
            "{name}: {error}"
        );
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            name
        );
        checked += 1;
    }
    assert!(
        checked >= 50,
        "the census must exercise module-only and internal names"
    );
}

#[test]
fn internal_names_cannot_be_imported_or_aliased() {
    let catalog = mech_stdlib::source_catalog();
    let names = catalog
        .all_exports()
        .filter(|export| export.exposure == FunctionExposure::Internal)
        .map(|export| export.canonical_name.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names.len(),
        18,
        "review changes to the internal operation census"
    );
    for name in names {
        if name.contains('_') {
            continue;
        }
        // An operation can have an explicitly callable module export as well
        // (string/concat). Only the module export may install that binding.
        let (module, item) = name.rsplit_once('/').unwrap();
        if catalog.module_export(module, item).is_some() {
            continue;
        }
        for source in [
            format!("+> hidden := {name}\nhidden(1)\n"),
            format!("+> {name}\n1\n"),
        ] {
            assert_eq!(
                rejected(&source, Arc::clone(&catalog)).code,
                "source-semantics/unknown-function-import",
                "{source}"
            );
        }
    }
}

#[test]
fn configured_empty_catalog_does_not_recover_global_named_declarations() {
    for source in ["math/add(unknown, 1)\n", "compare/eq(1,1)\n"] {
        assert_eq!(
            rejected(source, Arc::new(FunctionCatalog::empty())).code,
            "source-semantics/unknown-function"
        );
    }
}

#[test]
fn imports_bind_only_the_requested_visible_names() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "+> math\ncos(0f32)\n",
        "+> math/cos\nmath/cos(0f32)\n",
        "+> wave := math/cos\ncos(0f32)\n",
        "+> math/{sin}\ncos(0f32)\n",
        "+> math/*\nmath/cos(0f32)\n",
    ] {
        assert_eq!(
            rejected(source, Arc::clone(&catalog)).code,
            "source-semantics/unknown-function",
            "{source}"
        );
    }
}

#[test]
fn visible_calls_and_operators_preserve_source_and_bytecode_results() {
    let catalog = mech_stdlib::source_catalog();
    for (source, expected) in [
        ("+> math\nmath/cos(0f32)\n", "1"),
        ("+> math/*\ncos(0f32)\n", "1"),
        ("+> math/cos\ncos(0f32)\n", "1"),
        ("+> math/{sin, cos}\ncos(0f32)\n", "1"),
        ("+> wave := math/cos\nwave(0f32)\n", "1"),
        ("+> string\nstring/concat(\"a\",\"b\") == \"ab\"\n", "true"),
        ("compare/eq(1,1)\n", "true"),
        (
            "math/add(x<f64>, y<f64>) = out<f64> := out := 99.\nmath/add(1,2) + (1+2)\n",
            "102",
        ),
        ("1 > 0\n", "true"),
        ("xs := [1 2]\nxs[2]\n", "2"),
        ("r := 1..=3\nr[3]\n", "3"),
    ] {
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(Arc::clone(&catalog))
            .build_compiler()
            .unwrap();
        let product = compiler
            .compile_document(&document(source))
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
        for artifact in [product.artifact(), &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x801, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            for _ in 0..2 {
                instance.turn(&[]).unwrap();
                let actual = RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap())
                    .unwrap()
                    .format_canonical_inline();
                assert_eq!(actual, expected, "{source}");
            }
        }
    }
}

#[test]
fn ordered_roots_do_not_share_callable_imports() {
    use mech_engine::CanonicalOrderedDocument;
    use std::collections::BTreeMap;
    let root = |identity, source: &str| CanonicalOrderedDocument {
        identity,
        publish_result: true,
        document: document(source).document(),
        nominal_origin: None,
        nominal_package_id: None,
        input_schemas: BTreeMap::new(),
        resource_writes: BTreeMap::new(),
        imports: BTreeMap::new(),
        resolved_modules: BTreeSet::new(),
    };
    let catalog = mech_stdlib::source_catalog();
    let error = CanonicalSourceFrontend
        .compile_ordered_documents_with_catalog(
            &[
                root(0, "+> wave := math/cos\nwave(0f32)\n"),
                root(1, "wave(0f32)\n"),
            ],
            Arc::clone(&catalog),
        )
        .err()
        .expect("imports belong to their retained root");
    assert_eq!(error.code, "source-semantics/unknown-function");
    CanonicalSourceFrontend
        .compile_ordered_documents_with_catalog(
            &[
                root(0, "+> wave := math/cos\nwave(0f32)\n"),
                root(1, "+> wave := math/sin\nwave(0f32)\n"),
            ],
            catalog,
        )
        .expect("independent roots may reuse an import alias");
}

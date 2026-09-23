#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use std::collections::BTreeMap;

use mech_core::{
    CanonicalNominalPath, FunctionCatalogBuilder, MechSourceCode, ReactiveInstanceId,
    ResidentValueRef,
};
use mech_engine::__resident::CapturedSignalInput;
use mech_engine::resident::{ActivationFacts, activate};
use mech_runtime::resolver::{
    CanonicalDocumentCompilation, CanonicalDocumentHandoffError, CanonicalResolvedImport,
    InMemorySourceResolver, SourceResolver, source_request_for_import,
};
use mech_runtime::{RuntimeValueSnapshot, SourceDocument};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn document(id: u64, source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(id), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn catalog() -> mech_core::FunctionCatalog {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    catalog.build().unwrap()
}

#[test]
fn resolver_owned_handoff_retains_nominal_origin() {
    let source = "<event> := :idle | :busy\nvalue<event> := :idle\nvalue\n";
    let retained = SourceDocument::parse_resolved(
        "memory:events.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap()
    .with_nominal_origin(CanonicalNominalPath::new(vec!["sample".to_owned()]).unwrap())
    .with_nominal_package_id("sample@1");
    assert!(CanonicalDocumentCompilation::from_document(&retained.document()).is_err());
    CanonicalDocumentCompilation::from_source_document(&retained)
        .expect("resolver-owned handoff compiles the enum with its origin");
}

#[test]
fn resolved_canonical_export_is_a_usable_imported_binding() {
    let resolver =
        InMemorySourceResolver::new().with_string("app/dep.mec", "value := 42\n<+ value\n");
    let root = CanonicalDocumentCompilation::from_document(&document(
        1,
        "+> ./dep.mec\nanswer := dep/value\nanswer\n",
    ))
    .unwrap();
    let declaration = root.index.program_imports()[0].clone();
    let resolved = resolver
        .resolve(&source_request_for_import(
            &declaration,
            Some("memory:app/main.mec"),
        ))
        .unwrap()
        .unwrap();
    let MechSourceCode::String(dependency_source) = &resolved.source else {
        panic!("test dependency must be textual Mech source")
    };
    let dependency =
        CanonicalDocumentCompilation::from_document(&document(2, dependency_source)).unwrap();

    let dependency_artifact = dependency.program.compile_artifact().unwrap();
    let catalog = catalog();
    let mut dependency_instance = activate(
        ReactiveInstanceId::new(2, 0),
        &dependency_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    dependency_instance.turn(&[]).unwrap();
    let values = (0..dependency.program.program().outputs.len())
        .map(|output| {
            RuntimeValueSnapshot::from_value(dependency_instance.copied_output(output).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>();
    let exports = dependency.exports_from_values(&values).unwrap();
    let bindings = root
        .bind_resolved_imports(&[CanonicalResolvedImport {
            declaration,
            canonical_uri: resolved.canonical_uri,
            exports,
        }])
        .unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].name, "dep/value");

    let artifact = root.program.compile_artifact().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(1, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let values = bindings
        .iter()
        .map(|binding| Some(binding.value.to_value()))
        .collect::<Vec<_>>();
    let inputs = bindings
        .iter()
        .zip(&values)
        .map(|(binding, value)| {
            let artifact_slot = artifact.inputs()[binding.input as usize].slot;
            let slot = instance
                .plan
                .inputs
                .iter()
                .find(|input| input.artifact_slot == artifact_slot)
                .unwrap()
                .slot;
            CapturedSignalInput {
                slot,
                value: ResidentValueRef::Snapshot(std::slice::from_ref(value)),
            }
        })
        .collect::<Vec<_>>();
    instance.turn(&inputs).unwrap();
    let value = RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap()).unwrap();
    assert_eq!(value.format_canonical_inline(), "42");
}

#[test]
fn resolved_canonical_exports_can_be_sealed_into_artifact_constants() {
    let resolver =
        InMemorySourceResolver::new().with_string("app/dep.mec", "value := 42\n<+ value\n");
    let root = CanonicalDocumentCompilation::from_document(&document(
        31,
        "+> ./dep.mec\nanswer := dep/value\nanswer\n",
    ))
    .unwrap();
    let declaration = root.declared_imports()[0].clone();
    let resolved = resolver
        .resolve(&source_request_for_import(
            &declaration,
            Some("memory:app/main.mec"),
        ))
        .unwrap()
        .unwrap();
    let MechSourceCode::String(dependency_source) = &resolved.source else {
        panic!("test dependency must be textual Mech source")
    };
    let dependency =
        CanonicalDocumentCompilation::from_document(&document(32, dependency_source)).unwrap();
    let catalog = catalog();
    let dependency_artifact = dependency.program.compile_artifact().unwrap();
    let mut dependency_instance = activate(
        ReactiveInstanceId::new(32, 0),
        &dependency_artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    dependency_instance.turn(&[]).unwrap();
    let dependency_values = (0..dependency.program.program().outputs.len())
        .map(|output| {
            RuntimeValueSnapshot::from_value(dependency_instance.copied_output(output).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>();
    let exports = dependency.exports_from_values(&dependency_values).unwrap();
    let bindings = root
        .bind_resolved_imports(&[CanonicalResolvedImport {
            declaration,
            canonical_uri: resolved.canonical_uri,
            exports,
        }])
        .unwrap();
    let program = root
        .program
        .bind_input_constants(
            &bindings
                .iter()
                .map(|binding| (binding.input, binding.value.to_value()))
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let artifact = program.compile_artifact().unwrap();
    assert!(artifact.inputs().is_empty());
    let mut instance = activate(
        ReactiveInstanceId::new(31, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    instance.turn(&[]).unwrap();
    let value = RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap()).unwrap();
    assert_eq!(value.format_canonical_inline(), "42");
}

#[test]
fn resolved_bindings_remain_local_to_root_named_and_mika_owners() {
    fn exported_value(source: &str) -> RuntimeValueSnapshot {
        let dependency =
            CanonicalDocumentCompilation::from_document(&document(20, source)).unwrap();
        let artifact = dependency.program.compile_artifact().unwrap();
        let catalog = catalog();
        let mut instance = activate(
            ReactiveInstanceId::new(20, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        let export = &dependency.program.document_exports()[0];
        RuntimeValueSnapshot::from_value(instance.copied_output(export.output as usize).unwrap())
            .unwrap()
    }

    fn execute_import(
        compilation: &CanonicalDocumentCompilation,
        value: &RuntimeValueSnapshot,
        id: u32,
    ) -> RuntimeValueSnapshot {
        let declaration = compilation.declared_imports()[0].clone();
        let bindings = compilation
            .bind_resolved_imports(&[CanonicalResolvedImport {
                declaration,
                canonical_uri: "memory:app/dep.mec".to_owned(),
                exports: BTreeMap::from([("value".to_owned(), value.clone())]),
            }])
            .unwrap();
        let artifact = compilation.program.compile_artifact().unwrap();
        let catalog = catalog();
        let mut instance = activate(
            ReactiveInstanceId::new(id, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let values = bindings
            .iter()
            .map(|binding| Some(binding.value.to_value()))
            .collect::<Vec<_>>();
        let inputs = bindings
            .iter()
            .zip(&values)
            .map(|(binding, value)| {
                let artifact_slot = artifact.inputs()[binding.input as usize].slot;
                let slot = instance
                    .plan
                    .inputs
                    .iter()
                    .find(|input| input.artifact_slot == artifact_slot)
                    .unwrap()
                    .slot;
                CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::Snapshot(std::slice::from_ref(value)),
                }
            })
            .collect::<Vec<_>>();
        instance.turn(&inputs).unwrap();
        let output = compilation
            .program
            .document_outputs()
            .iter()
            .find(|output| output.kind == mech_engine::SourceDocumentOutputKind::Program)
            .unwrap()
            .output as usize;
        RuntimeValueSnapshot::from_value(instance.copied_output(output).unwrap()).unwrap()
    }

    let source = "+> ./dep.mec\nroot := dep/value\n<+ root\nroot\n\n```mech:worker\n+> ./dep.mec\nworker := dep/value\n<+ worker\nworker\n```\n\n~∘~⸢+> ./dep.mec\nchild := dep/value\n<+ child\nchild\n\n```mech:worker\n+> ./dep.mec\nnested := dep/value\n<+ nested\nnested\n```\n⸥\n";
    let document = document(21, source);
    let child = &document.mika_scopes()[0].section;
    let compilations = [
        CanonicalDocumentCompilation::from_document(&document).unwrap(),
        CanonicalDocumentCompilation::from_named_document_scope(&document, "worker").unwrap(),
        CanonicalDocumentCompilation::from_mika_section(child).unwrap(),
        CanonicalDocumentCompilation::from_named_mika_scope(child, "worker").unwrap(),
    ];
    let value = exported_value("value := 42\n<+ value\n");
    for (id, (compilation, expected)) in compilations
        .iter()
        .zip(["42", "42", "42", "42"])
        .enumerate()
    {
        assert_eq!(compilation.declared_imports().len(), 1);
        assert_eq!(compilation.program.document_exports().len(), 1);
        assert_eq!(
            execute_import(compilation, &value, 21 + id as u32).format_canonical_inline(),
            expected
        );
    }
}

#[test]
fn every_declared_import_requires_a_positioned_resolution() {
    let compilation = CanonicalDocumentCompilation::from_document(&document(
        23,
        "+> ./missing.mec\nanswer := 42\nanswer\n",
    ))
    .unwrap();
    let error = compilation
        .bind_resolved_imports(&[])
        .err()
        .expect("an unresolved declared import cannot publish bindings");
    let CanonicalDocumentHandoffError::UnresolvedImport { occurrence, .. } = error else {
        panic!("expected an unresolved import error")
    };
    let occurrence = occurrence.expect("declaration retains its source position");
    assert_eq!((occurrence.start.row, occurrence.start.col), (1, 4));
    assert_eq!((occurrence.end.row, occurrence.end.col), (1, 17));
}

#[test]
fn source_wildcard_requires_a_positioned_resolution() {
    let compilation = CanonicalDocumentCompilation::from_document(&document(
        27,
        "+> ./missing.mec/*\nanswer := 42\nanswer\n",
    ))
    .unwrap();
    let error = compilation
        .bind_resolved_imports(&[])
        .err()
        .expect("a source wildcard cannot omit its dependency");
    let CanonicalDocumentHandoffError::UnresolvedImport { occurrence, .. } = error else {
        panic!("expected an unresolved import error")
    };
    let occurrence = occurrence.expect("declaration retains its source position");
    assert_eq!((occurrence.start.row, occurrence.start.col), (1, 4));
    assert_eq!((occurrence.end.row, occurrence.end.col), (1, 19));
}

#[test]
fn context_alias_imports_do_not_require_source_dependency_edges() {
    let compilation = CanonicalDocumentCompilation::from_document(&document(
        24,
        "+> @env := cli/env\nanswer := 42\nanswer\n",
    ))
    .unwrap();
    assert_eq!(compilation.declared_imports().len(), 1);
    assert!(compilation.bind_resolved_imports(&[]).unwrap().is_empty());
}

#[test]
fn compiler_module_imports_remain_optional_source_dependencies() {
    for import in ["math", "math/sin", "math/*"] {
        let compilation = CanonicalDocumentCompilation::from_document(&document(
            26,
            &format!("+> {import}\nanswer := 42\nanswer\n"),
        ))
        .unwrap();
        assert_eq!(compilation.declared_imports().len(), 1);
        assert!(compilation.bind_resolved_imports(&[]).unwrap().is_empty());
    }
}

#[test]
fn incomplete_export_results_fail_at_the_retained_export_anchor() {
    let compilation =
        CanonicalDocumentCompilation::from_document(&document(25, "value := 42\n<+ value\n"))
            .unwrap();
    let error = compilation.exports_from_values(&[]).unwrap_err();
    let CanonicalDocumentHandoffError::MissingCompletedExport { range, .. } = error else {
        panic!("expected a missing completed export error")
    };
    let range = range.expect("export output retains a source anchor");
    assert_eq!((range.start.0, range.end.0), (15, 20));
}

#[test]
fn missing_dependency_exports_report_the_import_occurrence() {
    let compilation =
        CanonicalDocumentCompilation::from_document(&document(22, "+> dep/value\nvalue\n"))
            .unwrap();
    let declaration = compilation.declared_imports()[0].clone();
    let error = compilation
        .bind_resolved_imports(&[CanonicalResolvedImport {
            declaration,
            canonical_uri: "memory:dep.mec".to_owned(),
            exports: BTreeMap::new(),
        }])
        .err()
        .expect("missing dependency export must reject the handoff");
    let CanonicalDocumentHandoffError::MissingExport { occurrence, .. } = error else {
        panic!("expected a missing export error")
    };
    let occurrence = occurrence.expect("declared imports have retained positions");
    assert_eq!((occurrence.start.row, occurrence.start.col), (1, 4));
    assert_eq!((occurrence.end.row, occurrence.end.col), (1, 13));
}

#[test]
fn missing_namespace_exports_report_the_import_occurrence() {
    let compilation = CanonicalDocumentCompilation::from_document(&document(
        28,
        "+> ./dep.mec\nanswer := dep/missing\nanswer\n",
    ))
    .unwrap();
    let declaration = compilation.declared_imports()[0].clone();
    let error = compilation
        .bind_resolved_imports(&[CanonicalResolvedImport {
            declaration,
            canonical_uri: "memory:dep.mec".to_owned(),
            exports: BTreeMap::new(),
        }])
        .err()
        .expect("a namespace dependency cannot omit a referenced export");
    let CanonicalDocumentHandoffError::MissingExport {
        export, occurrence, ..
    } = error
    else {
        panic!("expected a missing namespace export error")
    };
    assert_eq!(export, "missing");
    let occurrence = occurrence.expect("declared imports have retained positions");
    assert_eq!((occurrence.start.row, occurrence.start.col), (1, 4));
    assert_eq!((occurrence.end.row, occurrence.end.col), (1, 13));
}

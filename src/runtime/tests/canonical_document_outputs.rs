#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram, SourceDocumentOutputKind};
use mech_runtime::RuntimeValueSnapshot;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document, reconstruct_source,
};

fn compile(source: &str) -> CanonicalSourceProgram {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x573), Revision(7), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed.diagnostics
    );
    assert_eq!(
        reconstruct_source(&parsed.root, &parsed.source).unwrap(),
        source
    );
    CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
}

fn rendered_turns(source: &str, expected: &[&[(SourceDocumentOutputKind, &str)]]) {
    let compiled = compile(source);
    assert!(
        compiled.program().inputs.is_empty(),
        "document-local presentation must not manufacture external inputs"
    );
    let artifact = compiled
        .compile_artifact()
        .expect("complete canonical artifact");
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x573, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .expect("maintained resident activation");
    let names = compiled
        .program()
        .outputs
        .iter()
        .map(|output| output.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        names
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        names.len(),
        "distinct source positions own distinct output names"
    );
    for turn in expected {
        assert_eq!(compiled.document_outputs().len(), turn.len());
        instance.turn(&[]).unwrap();
        for (binding, (kind, expected)) in compiled.document_outputs().iter().zip(*turn) {
            assert_eq!(binding.kind, *kind);
            let index = binding.output as usize;
            let anchor = compiled.source_map().outputs[index];
            assert_eq!(anchor.document, DocumentId(0x573));
            assert_eq!(anchor.revision, Revision(7));
            let source_slice = &source[anchor.range.start.0 as usize..anchor.range.end.0 as usize];
            match kind {
                SourceDocumentOutputKind::Program => assert_eq!(names[index], "result"),
                SourceDocumentOutputKind::Inline => {
                    assert!(source_slice.starts_with('{') && source_slice.ends_with('}'));
                    assert_eq!(
                        names[index],
                        format!("document:inline:{}", anchor.range.start.0)
                    );
                }
                SourceDocumentOutputKind::Fence => {
                    assert!(source_slice.starts_with("~~~") || source_slice.starts_with("```"));
                    assert_eq!(
                        names[index],
                        format!("document:fence:{}", anchor.range.start.0)
                    );
                }
            }
            let value =
                RuntimeValueSnapshot::from_value(instance.copied_output(index).unwrap()).unwrap();
            assert_eq!(
                value.format_canonical_inline(),
                *expected,
                "{kind:?}: {source_slice:?}"
            );
            assert_eq!(
                value.format_html(),
                format!("<span class='mech-value'>{expected}</span>")
            );
        }
    }
}

#[test]
fn shared_document_fixture_renders_inline_and_fence_outputs_from_the_root_program() {
    use SourceDocumentOutputKind::{Fence, Inline, Program};
    let source = include_str!("../../../tests/fixtures/syntax-source-boundary/document.mec");
    rendered_turns(
        source,
        &[
            &[(Program, "42"), (Inline, "42"), (Fence, "42")],
            &[(Program, "43"), (Inline, "43"), (Fence, "43")],
        ],
    );
}

#[test]
fn fence_results_keep_statement_versions_while_inline_reads_the_completed_program() {
    use SourceDocumentOutputKind::{Fence, Inline, Program};
    let source = "The final answer is {answer}.\n\n~answer := 0\n~~~mech\nanswer += 1\nanswer\n~~~\nanswer += 1\nanswer\n";
    rendered_turns(
        source,
        &[
            &[(Program, "2"), (Inline, "2"), (Fence, "1")],
            &[(Program, "4"), (Inline, "4"), (Fence, "3")],
        ],
    );
}

#[test]
fn fenced_definitions_share_root_scope_without_flattening_child_or_disabled_scopes() {
    use SourceDocumentOutputKind::{Fence, Program};
    let source = "~~~mech\n~answer := 40\nanswer\n~~~\n~~~mech:child\n~answer := 100\nanswer\n~~~\n~~~mech:disabled\nanswer += 100\n~~~\n```rust\nnot Mech code!\n```\n\nShown {{answer += 100}}.\n\nanswer += 2\nanswer\n";
    rendered_turns(
        source,
        &[
            &[(Program, "42"), (Fence, "40")],
            &[(Program, "44"), (Fence, "42")],
        ],
    );
}

#[test]
fn repeated_fence_results_and_inline_only_documents_have_stable_distinct_bindings() {
    use SourceDocumentOutputKind::{Fence, Inline, Program};
    rendered_turns(
        "~~~mech\n1\n~~~\n~~~mech\n1\n~~~\n",
        &[&[(Program, "1"), (Fence, "1"), (Fence, "1")]],
    );
    rendered_turns("Evaluated {1 + 2}.\n", &[&[(Program, "3"), (Inline, "3")]]);
}

#[test]
fn deferred_presentation_does_not_change_executable_statement_visibility() {
    let compiled = compile("before := answer\n~answer := 41\nbefore\n");
    assert_eq!(compiled.program().inputs.len(), 1);
    assert_eq!(compiled.program().inputs[0].name, "answer");
    assert_eq!(
        compiled.program().outputs[0].source,
        mech_engine::SourceValue::Input(0)
    );
}

#[test]
fn undocumented_mech_prefix_suffixes_cannot_promote_code_into_root_scope() {
    for info in [
        "mechanics",
        "mechdisabled",
        "mechchild",
        "mecchild",
        "🤖child",
    ] {
        let source = format!("~answer := 0\n~~~{info}\nanswer += 100\n~~~\nanswer\n");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x574), Revision(7), source.clone()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.diagnostics.is_empty(), "{info}");
        let error = CanonicalSourceFrontend
            .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
            .err()
            .expect("undocumented scope must be rejected before executing code");
        assert_eq!(error.code, "source-semantics/unsupported-fence-info");
        assert_eq!(error.anchor.document, DocumentId(0x574));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            info
        );
    }
}

#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram};
use mech_runtime::{
    CanonicalDocumentRenderer, CanonicalRenderScope, CanonicalScopeResults, RuntimeValueSnapshot,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn document(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57d), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn execute(
    owner: mech_syntax::document::DocumentScopeId,
    scope: CanonicalRenderScope,
    program: &CanonicalSourceProgram,
    instance_id: u32,
) -> CanonicalScopeResults {
    let artifact = program.compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(instance_id, 0),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    instance.turn(&[]).unwrap();
    let values = (0..program.program().outputs.len())
        .map(|output| {
            RuntimeValueSnapshot::from_value(instance.copied_output(output).unwrap()).unwrap()
        })
        .collect::<Vec<_>>();
    CanonicalScopeResults::from_values(owner, scope, program, &values).unwrap()
}

#[test]
fn retained_document_renders_root_named_and_mika_results_in_place() {
    let source = "Prose A & B displays {{1 + 1}} and evaluates {1 + 2}.\n\n```mech{color: \"red'&\"}\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:hidden\ncounter += 10\n```\n\n```mech:disabled\ncounter += 100\n```\n\n```text\n<literal>\n```\n\n```mech:worker\n~worker := 20\nworker += 2\nworker\n```\n\n~∘~⸢Child A & B evaluates {1 + 4}.\n\n```mech\n~child := 30\nchild += 3\nchild\n```\n\n```mech:worker\n~nested := 40\nnested += 4\nnested\n```\n⸥\n\n(i)> Block evaluates {2 + 4}.\n\ncounter\n";
    let document = document(source);
    let root_owner = document.scope_id();
    let child = &document.mika_scopes()[0].section;
    let child_owner = child.scope_id();
    let frontend = CanonicalSourceFrontend;
    let root = frontend.compile_document(&document).unwrap();
    let named = frontend
        .compile_named_document_scope(&document, "worker")
        .unwrap();
    let child_root = frontend.compile_mika_section(child).unwrap();
    let child_named = frontend.compile_named_mika_scope(child, "worker").unwrap();
    let results = vec![
        execute(root_owner, CanonicalRenderScope::Root, &root, 1),
        execute(
            root_owner,
            CanonicalRenderScope::Named("worker".to_owned()),
            &named,
            2,
        ),
        execute(child_owner, CanonicalRenderScope::Root, &child_root, 3),
        execute(
            child_owner,
            CanonicalRenderScope::Named("worker".to_owned()),
            &child_named,
            4,
        ),
    ];

    let renderer = CanonicalDocumentRenderer;
    let html = renderer.render_html(&document, &results).unwrap();
    assert!(
        html.starts_with("<article class='mech-document'>"),
        "{html}"
    );
    assert!(html.ends_with("</article>"), "{html}");
    assert!(html.contains("Prose A &amp; B displays"), "{html}");
    assert!(
        html.contains("<code class='mech-inline'>1 + 1</code>"),
        "{html}"
    );
    assert!(!html.contains("{1 + 2}"), "{html}");
    assert!(!html.contains("counter += 10\n"), "{html}");
    assert!(html.contains("counter += 100"), "{html}");
    assert!(html.contains("&lt;literal&gt;"), "{html}");
    assert!(
        html.contains("data-mech-styles='color:red&#39;&amp;'"),
        "{html}"
    );
    for value in ["1", "3", "5", "6", "11", "22", "33", "44"] {
        assert!(
            html.contains(&format!(">{value}</span>")),
            "missing {value}: {html}"
        );
    }
    let mika_start = html.find("<section class='mech-mika'>").unwrap();
    let mika_html = &html[mika_start..];
    assert!(mika_html.contains(">5</span>"), "{mika_html}");
    assert!(mika_html.contains(">33</span>"), "{mika_html}");
    assert!(mika_html.contains(">44</span>"), "{mika_html}");
    assert!(!mika_html.contains(">22</span>"), "{mika_html}");

    let text = renderer.render_text(&document, &results).unwrap();
    assert!(text.contains("Prose A & B displays 1 + 1 and evaluates 3."));
    assert!(!text.contains("counter += 10\n"));
    assert!(text.contains("counter += 100"));
    assert!(text.contains("<literal>"));
    assert!(text.contains("Child A & B evaluates 5."));
    assert!(text.contains("Block evaluates 6."));
    for value in ["11", "22", "33", "44"] {
        assert!(
            text.contains(&format!("=> {value}")),
            "missing {value}: {text}"
        );
    }
}

#[test]
fn renderer_rejects_results_from_another_document_owner() {
    let first = document("answer := 1\nanswer\n");
    let second = {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x57e), Revision(1), "answer := 2\nanswer\n").unwrap(),
            ParseConfig::default(),
        );
        DocumentSyntax::cast(parsed.syntax()).unwrap()
    };
    let program = CanonicalSourceFrontend.compile_document(&second).unwrap();
    let foreign = execute(second.scope_id(), CanonicalRenderScope::Root, &program, 5);
    let error = CanonicalDocumentRenderer
        .render_html(&first, &[foreign])
        .unwrap_err();
    assert!(error.message.contains("different canonical document"));
}

#[test]
fn renderer_rejects_duplicate_results_for_one_presentation_slot() {
    let document = document("answer := 42\nanswer\n");
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let results = execute(document.scope_id(), CanonicalRenderScope::Root, &program, 7);
    let error = CanonicalDocumentRenderer
        .render_html(&document, &[results.clone(), results])
        .unwrap_err();
    assert!(error.message.contains("duplicate results"));
}

#[test]
fn renderer_preserves_title_subtitle_and_plain_document_structure() {
    let document = document(
        "Grammar Conformance\n===================\nBody A & B.\n\n1. Overview\n--------\nMore prose.\n",
    );
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(html.contains("<header class='mech-document-title'><pre>"));
    assert!(html.contains("Grammar Conformance"));
    assert!(
        html.contains("<h2 class='mech-subtitle'>1. Overview\n--------</h2>"),
        "{html}"
    );
    assert!(html.contains("<p>Body A &amp; B.</p>"));
    assert!(html.contains("<p>More prose.</p>"));
}

#[test]
fn visible_executable_fences_require_their_owner_result() {
    let document = document("```mech\nanswer := 42\nanswer\n```\n");
    let error = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap_err();
    assert_eq!(
        error.message,
        "visible executable fence has no completed scope result"
    );
    assert!(error.range.is_some());
}

#[test]
fn shared_mixed_document_fixture_is_rendered_without_dropping_nodes() {
    let source = include_str!("../../syntax/tests/fixtures/grammar/accepted/mixed-document.mec");
    let document = document(source);
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let results = [execute(
        document.scope_id(),
        CanonicalRenderScope::Root,
        &program,
        6,
    )];
    let renderer = CanonicalDocumentRenderer;
    let html = renderer.render_html(&document, &results).unwrap();
    for expected in [
        "Mixed Document",
        "<strong class='mech-strong'>strong</strong>",
        "<a class='mech-hyperlink' href='https://example.com'>a link</a>",
        "<code class='mech-inline'>x + 1</code>",
        "x := 1",
        "y := x + 1",
        "Parsed as an information block.",
    ] {
        assert!(html.contains(expected), "missing {expected:?}: {html}");
    }
    let text = renderer.render_text(&document, &results).unwrap();
    for expected in [
        "Mixed Document",
        "**strong**",
        "[a link](https://example.com)",
        "x + 1",
        "x := 1",
        "y := x + 1",
        "Parsed as an information block.",
    ] {
        assert!(text.contains(expected), "missing {expected:?}: {text}");
    }
}

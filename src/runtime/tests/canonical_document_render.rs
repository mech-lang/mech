#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::{CanonicalNominalPath, FunctionCatalogBuilder, ReactiveInstanceId};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram, SourceDocumentOutputKind};
use mech_runtime::{
    CanonicalDocumentRenderer, CanonicalRenderScope, CanonicalScopeResults, RuntimeValueSnapshot,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentStream, DocumentSyntax, ParseConfig, Revision, StreamProgress,
    TextSnapshot, parse_canonical_document,
};

fn document(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57d), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}

fn streamed_document(source: &str) -> DocumentSyntax {
    let mut stream = DocumentStream::new(DocumentId(0x57d), ParseConfig::default());
    for character in source.chars() {
        let mut update = stream.append(&character.to_string(), 19).unwrap();
        while update.progress == StreamProgress::NeedsProcessing {
            update = stream.advance(19);
        }
        assert_eq!(update.progress, StreamProgress::NeedInput);
    }
    let mut update = stream.finish(19);
    while update.progress == StreamProgress::NeedsProcessing {
        update = stream.advance(19);
    }
    assert_eq!(update.progress, StreamProgress::Finished);
    let snapshot = stream.materialize().unwrap();
    assert!(snapshot.is_strictly_clean());
    DocumentSyntax::cast(snapshot.syntax()).unwrap()
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
fn finalized_streams_feed_execution_and_complete_document_rendering() {
    let source = "Streamed Report\n===============\nanswer := 40 + 2\nThe answer is {answer}.\n";
    let document = streamed_document(source);
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let results = [execute(
        document.scope_id(),
        CanonicalRenderScope::Root,
        &program,
        826,
    )];
    let renderer = CanonicalDocumentRenderer;
    let html = renderer.render_html(&document, &results).unwrap();
    assert!(
        html.contains("<h1 class='mech-document-title'>Streamed Report</h1>"),
        "{html}"
    );
    assert!(html.contains("The answer is <span"), "{html}");
    assert!(html.contains(">42</span>"), "{html}");
    assert!(!html.contains("{answer}"), "{html}");
    let text = renderer.render_text(&document, &results).unwrap();
    assert!(text.contains("Streamed Report"), "{text}");
    assert!(text.contains("The answer is 42."), "{text}");
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
fn isolated_document_scopes_reject_nominal_declarations_without_scope_provenance() {
    let named =
        document("```mech:worker\n<event> := :idle | :busy\nvalue<event> := :idle\nvalue\n```\n");
    let frontend = CanonicalSourceFrontend.with_nominal_origin(
        CanonicalNominalPath::new(vec!["sample-package".to_owned(), "document".to_owned()])
            .unwrap(),
    );
    let error = frontend
        .compile_named_document_scope(&named, "worker")
        .err()
        .expect("a named scope needs its own durable nominal namespace");
    assert_eq!(
        error.code,
        "source-semantics/isolated-nominal-origin-required"
    );

    let mika = document("~∘~⸢<event> := :idle | :busy\nvalue<event> := :idle\nvalue\n⸥\n");
    let error = frontend
        .compile_mika_section(&mika.mika_scopes()[0].section)
        .err()
        .expect("a Mika scope needs its own durable nominal namespace");
    assert_eq!(
        error.code,
        "source-semantics/isolated-nominal-origin-required"
    );
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
fn renderer_rejects_results_from_an_older_document_revision() {
    let parsed = |revision, source| {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x580), Revision(revision), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        DocumentSyntax::cast(parsed.syntax()).unwrap()
    };
    let old = parsed(1, "answer := 1\nanswer\n");
    let current = parsed(2, "answer := 2\nanswer\n");
    let program = CanonicalSourceFrontend.compile_document(&old).unwrap();
    let stale = execute(old.scope_id(), CanonicalRenderScope::Root, &program, 8);
    let error = CanonicalDocumentRenderer
        .render_html(&current, &[stale])
        .unwrap_err();
    assert!(
        error
            .message
            .contains("different canonical document revision")
    );
}

#[test]
fn renderer_rejects_same_document_results_relabelled_to_another_owner() {
    let document = document("root := 1\nroot\n\n~∘~⸢child := 2\nchild\n⸥\n");
    let child = &document.mika_scopes()[0].section;
    let program = CanonicalSourceFrontend.compile_mika_section(child).unwrap();
    let error = CanonicalScopeResults::from_values(
        document.scope_id(),
        CanonicalRenderScope::Root,
        &program,
        &[],
    )
    .err()
    .expect("owner mismatch must be rejected before accepting values");
    assert!(error.message.contains("retained presentation owner"));
    assert!(error.range.is_some());
}

#[test]
fn renderer_rejects_results_relabelled_to_another_execution_scope() {
    let document = document("answer := 42\nanswer\n");
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let relabelled = execute(
        document.scope_id(),
        CanonicalRenderScope::Named("ghost".to_owned()),
        &program,
        11,
    );
    let error = CanonicalDocumentRenderer
        .render_html(&document, &[relabelled])
        .unwrap_err();
    assert!(error.message.contains("retained document execution scope"));
    assert!(error.range.is_some());
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
    assert!(html.contains(
        "<header class='mech-document-header'><h1 class='mech-document-title'>Grammar Conformance</h1>"
    ));
    assert!(html.contains("Grammar Conformance"));
    assert!(!html.contains("==================="), "{html}");
    assert!(
        html.contains("<h2 class='mech-subtitle' id='1'>Overview</h2>"),
        "{html}"
    );
    assert!(html.contains("<p>Body A &amp; B.</p>"));
    assert!(html.contains("<p>More prose.</p>"));
}

#[test]
fn text_renderer_preserves_retained_blank_lines() {
    let document = document("first line.\n\nsecond line.\n");
    assert_eq!(
        CanonicalDocumentRenderer
            .render_text(&document, &[])
            .unwrap(),
        "first line.\n\nsecond line.\n"
    );
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
fn declaration_only_fences_do_not_require_completed_text_results() {
    let document = document("```mech\n<count> := <u64>\n```\nanswer := 42u64\nanswer\n");
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let results = [execute(
        document.scope_id(),
        CanonicalRenderScope::Root,
        &program,
        0x923,
    )];

    let text = CanonicalDocumentRenderer
        .render_text(&document, &results)
        .unwrap();
    assert!(text.contains("<count> := <u64>"), "{text}");
    assert!(text.contains("=> 42"), "{text}");
}

#[test]
fn visible_root_programs_require_their_owner_result() {
    let document = document("answer := 42\nanswer\n");
    for error in [
        CanonicalDocumentRenderer
            .render_html(&document, &[])
            .unwrap_err(),
        CanonicalDocumentRenderer
            .render_text(&document, &[])
            .unwrap_err(),
    ] {
        assert_eq!(
            error.message,
            "visible root program has no completed scope result"
        );
        assert!(error.range.is_some());
    }
}

#[test]
fn visible_mika_root_programs_require_their_owner_result() {
    let document = document("~∘~⸢answer := 42\nanswer\n⸥\n");
    let error = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap_err();
    assert_eq!(
        error.message,
        "visible root program has no completed scope result"
    );
    assert!(error.range.is_some());
}

#[test]
fn metadata_only_documents_do_not_require_program_results() {
    for source in ["+> ./dep.mec\n", "@ui := fs://workspace\n"] {
        let document = document(source);
        CanonicalDocumentRenderer
            .render_html(&document, &[])
            .unwrap();
        CanonicalDocumentRenderer
            .render_text(&document, &[])
            .unwrap();
    }
}

#[test]
fn hidden_and_output_suppressed_fences_do_not_leak_the_program_result() {
    for (source, source_is_visible) in [
        ("```mech:hidden\n42\n```\n", false),
        ("```mech{output: false}\n42\n```\n", true),
    ] {
        let document = document(source);
        let source_html = CanonicalDocumentRenderer.format_html(&document).unwrap();
        assert!(
            source_html.contains("<code><span class='mech-number'>42</span>\n</code>"),
            "{source_html}"
        );
        assert_eq!(
            source_html.contains("class='mech-code-block hidden'"),
            !source_is_visible
        );
        assert!(
            !source_html.contains("class='mech-output'"),
            "{source_html}"
        );
        assert_eq!(
            CanonicalDocumentRenderer.format_text(&document).unwrap(),
            source
        );
        let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
        let results = [execute(
            document.scope_id(),
            CanonicalRenderScope::Root,
            &program,
            9,
        )];
        let html = CanonicalDocumentRenderer
            .render_html(&document, &results)
            .unwrap();
        assert_eq!(
            html.contains("<figure class='mech-code-block'"),
            source_is_visible
        );
        assert!(!html.contains("class='mech-value'"), "{html}");
        assert!(!html.contains("class='mech-program-output'"), "{html}");
    }
}

#[test]
fn renderer_rejects_active_script_hyperlinks() {
    let document = document("[open](javascript:alert)\n");
    let error = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap_err();
    assert!(error.message.contains("unsafe hyperlink scheme"));
    assert!(error.range.is_some());
}

#[test]
fn retained_images_render_with_safe_escaped_attributes_and_captions() {
    let document =
        document("![A ' & B](image'file.png?x=1&y=2)\n![x'onerror='alert(1)](missing.png)\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(
        html.contains(
            "<img class='mech-image' src='image&#39;file.png?x=1&amp;y=2' alt='A &#39; &amp; B' />"
        ),
        "{html}"
    );
    assert!(
        html.contains("<figcaption class='mech-figure-caption'>A ' &amp; B</figcaption>"),
        "{html}"
    );
    assert!(html.contains("alt='x&#39;onerror=&#39;alert(1)'"), "{html}");
    assert!(!html.contains("alt='x'onerror='"), "{html}");
}

#[test]
fn image_url_extraction_starts_after_parentheses_in_the_caption() {
    let document = document("![Results (draft)](plot.png)\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(html.contains("src='plot.png'"), "{html}");
    assert!(html.contains("alt='Results (draft)'"), "{html}");
}

#[test]
fn image_options_are_safely_whitelisted_into_presentation_attributes() {
    let document = document(
        "![small](image.png){width: \"200px\", height: \"50%\", alignment: center}\n![unsafe](other.png){width: \"1px; color: red\", onclick: alert}\n",
    );
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(
        html.contains(
            "class='mech-image mech-image-align-center' src='image.png' alt='small' style='width: 200px; height: 50%'"
        ),
        "{html}"
    );
    assert!(!html.contains("color: red"), "{html}");
    assert!(!html.contains("onclick"), "{html}");
}

#[test]
fn inert_fences_preserve_their_language_for_highlighting() {
    let document = document("```rust\nfn main() {}\n```\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(html.contains("<pre><code data-language='rust'>"), "{html}");
}

#[test]
fn title_front_matter_is_semantic_and_uses_completed_inline_results() {
    let document = document(
        "Result Report\n===================\nauthor: Ada\nvalue: {answer}\n===================\nanswer := 42\n",
    );
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let results = [execute(
        document.scope_id(),
        CanonicalRenderScope::Root,
        &program,
        18,
    )];
    let html = CanonicalDocumentRenderer
        .render_html(&document, &results)
        .unwrap();
    assert!(
        html.contains("<h1 class='mech-document-title'>Result Report</h1>"),
        "{html}"
    );
    assert!(
        html.contains("<div class='mech-title-field'><dt>author</dt><dd>Ada</dd></div>"),
        "{html}"
    );
    assert!(html.contains("<dt>value</dt><dd><span"), "{html}");
    assert!(html.contains(">42</span>"), "{html}");
    assert!(!html.contains("{answer}"), "{html}");
    assert!(!html.contains("==================="), "{html}");
    let text = CanonicalDocumentRenderer
        .render_text(&document, &results)
        .unwrap();
    assert!(text.contains("value: 42"), "{text}");
    assert!(!text.contains("{answer}"), "{text}");
}

#[test]
fn shim_reference_numbers_start_at_the_final_front_matter_value() {
    let document = document(
        "Report\n======\nauthor: Hidden[^hidden] [hidden]\nauthor: Visible[^visible] [visible]\n======\n\n[^hidden]: Hidden note.\n\n[^visible]: Visible note.\n\n[hidden]: Hidden citation.\n\n[visible]: Visible citation.\n",
    );
    for slots in [
        CanonicalDocumentRenderer
            .format_browser_html_slots(&document)
            .unwrap(),
        CanonicalDocumentRenderer
            .format_static_html_slots(&document)
            .unwrap(),
    ] {
        let author = &slots["AUTHOR"];
        assert!(!author.contains("Hidden"), "{author}");
        assert!(
            author.contains("href='#footnote-visible'>1</a>"),
            "{author}"
        );
        assert!(
            author.contains("href='#reference-visible'>1</a>"),
            "{author}"
        );
        assert!(
            slots["FOOTNOTES"]
                .contains("id='footnote-visible'><span class='mech-footnote-id'>1:</span>"),
            "{:?}",
            slots["FOOTNOTES"]
        );
    }
}

#[test]
fn raw_hyperlinks_and_inline_code_use_semantic_html() {
    let document = document("Visit http://example.com/path or `x < y & z`.\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(
        html.contains(
            "<a class='mech-hyperlink' href='http://example.com/path'>http://example.com/path</a>"
        ),
        "{html}"
    );
    assert!(
        html.contains("<code class='mech-inline-code'>x &lt; y &amp; z</code>"),
        "{html}"
    );
    assert!(!html.contains("`x"), "{html}");
}

#[test]
fn retained_inline_markup_uses_semantic_elements_without_delimiters() {
    let document = document(
        "1. Overview\n--------\n(1.2) Details\n*emphasis* _underline_ ~strike~ $$x+1$$ [ref] [^note] §1.2\n",
    );
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    for expected in [
        "<em class='mech-emphasis'>emphasis</em>",
        "<u class='mech-underline'>underline</u>",
        "<del class='mech-strikethrough'>strike</del>",
        "<span class='mech-inline-equation'>x+1</span>",
        "<span class='mech-reference'>[<a class='mech-reference-link' href='#reference-ref'>1</a>]</span>",
        "<a class='mech-footnote-reference' href='#footnote-note'>1</a>",
        "<a class='mech-section-reference-link' href='#1.2'>§1.2</a>",
        "<h3 class='mech-subtitle' id='1.2'>Details</h3>",
    ] {
        assert!(html.contains(expected), "missing {expected:?}: {html}");
    }
    for leaked in ["*emphasis*", "_underline_", "~strike~", "$$x+1$$"] {
        assert!(!html.contains(leaked), "leaked {leaked:?}: {html}");
    }
}

#[test]
fn retained_rich_document_nodes_use_semantic_html_containers() {
    for (source, expected) in [
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-abstract.mec"),
            "<aside class='mech-abstract'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-quote.mec"),
            "<blockquote class='mech-quote-block'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-info.mec"),
            "<aside class='mech-info-block'><p>Isolated info</p></aside>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-success.mec"),
            "<aside class='mech-success-block'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-idea.mec"),
            "<aside class='mech-idea-block'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-warning.mec"),
            "<aside class='mech-warning-block'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-error.mec"),
            "<aside class='mech-error-block'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-question.mec"),
            "<aside class='mech-question-block'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-prompt.mec"),
            "<div class='mech-prompt'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-table.mec"),
            "<table class='mech-table'><thead><tr><th class='mech-table-cell mech-align-left'>Name ",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-thematic.mec"),
            "<hr class='mech-thematic-break' />",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-equation.mec"),
            "<div class='mech-equation'>x + 1</div>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/citation.mec"),
            "<section class='mech-works-cited'><h3 class='mech-backmatter-heading'>Works Cited</h3><div class='mech-citation' id='reference-ref1'><span class='mech-citation-id'>[1]:</span>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/figures.mec"),
            "<figure class='mech-figure-table'><div class='mech-figure-grid'>",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/float.mec"),
            "<div class='mech-float mech-float-left'>",
        ),
    ] {
        let document = document(source);
        let html = CanonicalDocumentRenderer
            .render_html(&document, &[])
            .unwrap();
        assert!(html.contains(expected), "missing {expected:?}: {html}");
    }
}

#[test]
fn citations_are_numbered_and_deferred_to_link_safe_backmatter() {
    let document = document(
        "See [ref] before the definition.\n[ref]: Source [site](https://example.com)\nText after the definition.\n",
    );
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(
        html.contains(
            "<span class='mech-reference'>[<a class='mech-reference-link' href='#reference-ref'>1</a>]</span>"
        ),
        "{html}"
    );
    assert!(
        html.contains(
            "class='mech-hyperlink mech-citation-external-link' href='https://example.com' target='_blank' rel='noopener noreferrer'"
        ),
        "{html}"
    );
    let after = html.find("Text after the definition.").unwrap();
    let works_cited = html.find("class='mech-works-cited'").unwrap();
    assert!(works_cited > after, "{html}");
}

#[test]
fn citation_numbers_and_backmatter_follow_first_reference_order() {
    let document = document("Read [alpha] then [beta].\n[beta]: B\n[alpha]: A\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(html.contains("href='#reference-alpha'>1</a>"), "{html}");
    assert!(html.contains("href='#reference-beta'>2</a>"), "{html}");
    let works = html.split("class='mech-works-cited'").nth(1).unwrap();
    assert!(
        works.find("id='reference-alpha'").unwrap() < works.find("id='reference-beta'").unwrap(),
        "{html}"
    );
}

#[test]
fn floats_preserve_left_and_right_direction() {
    let document = document("<<: ![left](left.png)\n:>> ![right](right.png)\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(
        html.contains("<div class='mech-float mech-float-left'>"),
        "{html}"
    );
    assert!(
        html.contains("<div class='mech-float mech-float-right'>"),
        "{html}"
    );
}

#[test]
fn table_alignment_is_projected_to_header_and_body_cells() {
    let document = document(include_str!(
        "../../syntax/tests/fixtures/grammar/accepted/mechdown-table.mec"
    ));
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    for expected in [
        "<th class='mech-table-cell mech-align-left'>Name ",
        "<th class='mech-table-cell mech-align-right'>Value ",
        "<td class='mech-table-cell mech-align-left'>one  ",
        "<td class='mech-table-cell mech-align-right'>1     ",
    ] {
        assert!(html.contains(expected), "missing {expected:?}: {html}");
    }
}

#[test]
fn figure_grids_preserve_panel_labels_and_combined_caption() {
    let document = document(include_str!(
        "../../syntax/tests/fixtures/grammar/accepted/figures.mec"
    ));
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    for expected in [
        "<figure class='mech-figure-table'>",
        "<figure class='mech-subfigure' data-panel='a'>",
        "<figure class='mech-subfigure' data-panel='b'>",
        "<figure class='mech-subfigure' data-panel='c'>",
        "<figcaption class='mech-figure-table-caption'>",
        "<span class='mech-subfigure-label'>(a)</span> one",
        "<span class='mech-subfigure-label'>(c)</span> wide",
    ] {
        assert!(html.contains(expected), "missing {expected:?}: {html}");
    }
}

#[test]
fn retained_footnotes_preserve_every_paragraph() {
    let document = document("[^note]: First paragraph.\nSecond paragraph.\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(
        html.contains(
            "<aside class='mech-footnote' id='footnote-note'><span class='mech-footnote-id'>1:</span><p>First paragraph.</p><p>Second paragraph.</p></aside>"
        ),
        "{html}"
    );
}

#[test]
fn retained_lists_use_semantic_html_without_source_markers() {
    for (source, expected, marker) in [
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-ordered.mec"),
            "<ol class='mech-ordered-list'><li value='1'>first",
            "1.first",
        ),
        (
            include_str!("../../syntax/tests/fixtures/grammar/accepted/mechdown-unordered.mec"),
            "<ul class='mech-unordered-list'><li>first",
            "- first",
        ),
        (
            "-[x]done\ncontinued\n",
            "<ul class='mech-check-list'><li><input class='mech-check-item' type='checkbox' disabled checked /><span class='mech-list-item-label'>done</span><p class='mech-list-item-continuation'>continued</p>",
            "-[x]done",
        ),
        (
            "-[]todo\n",
            "<ul class='mech-check-list'><li><input class='mech-check-item' type='checkbox' disabled /><span class='mech-list-item-label'>todo</span>",
            "-[]todo",
        ),
    ] {
        let document = document(source);
        let html = CanonicalDocumentRenderer
            .render_html(&document, &[])
            .unwrap();
        assert!(html.contains(expected), "missing {expected:?}: {html}");
        assert!(!html.contains(marker), "leaked {marker:?}: {html}");
    }
}

#[test]
fn ordered_lists_preserve_their_authored_start_and_item_values() {
    let document = document("3.third\n5.fifth\n");
    let html = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap();
    assert!(
        html.contains(
            "<ol class='mech-ordered-list' start='3'><li value='3'>third\n</li><li value='5'>fifth"
        ),
        "{html}"
    );
}

#[test]
fn renderer_rejects_active_script_image_sources() {
    let document = document("![unsafe](javascript:payload)\n");
    let error = CanonicalDocumentRenderer
        .render_html(&document, &[])
        .unwrap_err();
    assert!(error.message.contains("unsafe image source scheme"));
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
        "<span class='mech-var-name' data-mech-var-name='x'>x</span> := <span class='mech-number'>1</span>",
        "<span class='mech-var-name' data-mech-var-name='y'>y</span> := <span class='mech-var-name' data-mech-var-name='x'>x</span> + <span class='mech-number'>1</span>",
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

#[test]
fn source_body_preserves_front_matter_without_duplicate_framing() {
    let document = document(
        "Result Report\n===================\nauthor: Ada\n===================\nanswer := 42\n",
    );
    let html = CanonicalDocumentRenderer
        .format_html_body(&document)
        .unwrap();
    assert!(html.contains("<dt>author</dt><dd>Ada</dd>"), "{html}");
    assert!(!html.contains("<h1"), "{html}");
    assert!(!html.contains("<article"), "{html}");
    assert!(html.contains("answer"), "{html}");
}

#[test]
fn live_body_keeps_inline_and_fence_output_addresses() {
    let document = document("Result {1 + 2}.\n\n```mech\n40 + 2\n```\n");
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let addresses = program
        .document_outputs()
        .iter()
        .filter(|output| {
            output.visible && output.kind != mech_engine::SourceDocumentOutputKind::Program
        })
        .map(|output| {
            (
                program.source_map().outputs[output.output as usize].range,
                u64::from(output.output),
            )
        })
        .collect::<Vec<_>>();
    let html = CanonicalDocumentRenderer
        .format_html_body_live(&document, &addresses)
        .unwrap();
    assert!(html.contains("class='mech-inline-mech-code'"), "{html}");
    assert!(html.contains("class='mech-block-output'"), "{html}");
    for (_, output_id) in addresses {
        assert!(html.contains(&format!("id='{output_id}:0'")), "{html}");
    }
}

#[test]
fn browser_source_mounts_match_compiled_canonical_output_anchors() {
    let document =
        document("~answer := 41\n\nThe answer is {answer + 1}.\n\n~~~mech\nanswer + 2\n~~~\n");
    let html = CanonicalDocumentRenderer
        .format_browser_html(&document)
        .unwrap();
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let expected_presentation_ids = program
        .document_outputs()
        .iter()
        .filter(|binding| binding.visible && binding.kind != SourceDocumentOutputKind::Program)
        .map(|binding| {
            let range = program.source_map().outputs[binding.output as usize].range;
            mech_runtime::canonical_document_output_id(binding.kind, range)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        mech_runtime::canonical_document_presentation_output_ids(&document).unwrap(),
        expected_presentation_ids,
    );
    for binding in program
        .document_outputs()
        .iter()
        .filter(|binding| binding.visible)
    {
        let range = program.source_map().outputs[binding.output as usize].range;
        let id = mech_runtime::canonical_document_output_id(binding.kind, range);
        assert!(
            html.contains(&format!("data-mech-output-address='{id}:0'")),
            "{binding:?}: {html}"
        );
    }
    assert!(html.contains("class='mech-inline-mech-code'"), "{html}");
    assert!(html.contains("class='mech-block-output'"), "{html}");
    assert!(
        CanonicalDocumentRenderer
            .render_html(&document, &[])
            .is_err()
    );
}

#[test]
fn browser_source_mounts_tuple_destructure_root_results() {
    let document = document("(x, y) := (1, 2)\n");
    let html = CanonicalDocumentRenderer
        .format_browser_html(&document)
        .unwrap();
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let output = program
        .document_outputs()
        .iter()
        .find(|output| output.kind == SourceDocumentOutputKind::Program && output.visible)
        .expect("tuple destructuring publishes the root result");
    let range = program.source_map().outputs[output.output as usize].range;
    let id = mech_runtime::canonical_document_output_id(output.kind, range);
    assert!(
        html.contains(&format!("data-mech-output-address='{id}:0'")),
        "{html}"
    );
}

#[test]
fn browser_source_omits_mounts_for_named_and_mika_scopes() {
    let document =
        document("Root {1}.\n\n~~~mech:worker\n2\n~~~\n\n~∘~⸢Child {3}.\n\n~~~mech\n4\n~~~\n⸥\n");
    let html = CanonicalDocumentRenderer
        .format_browser_html(&document)
        .unwrap();
    assert_eq!(
        html.matches("class='mech-inline-mech-code'").count(),
        1,
        "{html}"
    );
    assert!(!html.contains("class='mech-block-output'"), "{html}");
}

#[test]
fn browser_source_omits_mounts_for_declaration_only_root_fences() {
    let document = document(
        "~~~mech\n#Deferred() => <u64>\n  | :Start\n  | :Done.\n~~~\n\n~~~mech\n40 + 2\n~~~\n",
    );
    let html = CanonicalDocumentRenderer
        .format_browser_html(&document)
        .unwrap();
    assert_eq!(
        html.matches("class='mech-block-output'").count(),
        1,
        "{html}"
    );
    assert_eq!(
        mech_runtime::canonical_document_presentation_output_ids(&document)
            .unwrap()
            .len(),
        1,
    );
}

#[test]
fn browser_source_omits_program_mounts_for_effect_only_roots() {
    for source in [
        "@view := scene://view/root{:write(replace)}\n@view/replace <- 42\n",
        "~> @trigger/EVENT { value := @env/HOME }\n",
        "@out/line <- 1\nf() => <f64>\n  | * => 2.\n",
    ] {
        let html = CanonicalDocumentRenderer
            .format_browser_html(&document(source))
            .unwrap();
        assert!(!html.contains("class='mech-program-output'"), "{html}");
    }
}

#[test]
fn browser_source_preserves_program_mount_before_trailing_effects() {
    for source in [
        "answer := 42\n@out/line <- answer\n",
        "answer := 42\n~> answer {}\n",
    ] {
        let document = document(source);
        let html = CanonicalDocumentRenderer
            .format_browser_html(&document)
            .unwrap();
        assert!(
            html.contains("class='mech-block-output mech-program-output'"),
            "{html}"
        );
    }

    let activation = document("answer := 42\n~> answer {}\n");
    let program = CanonicalSourceFrontend
        .compile_document(&activation)
        .unwrap();
    assert!(
        program
            .document_outputs()
            .iter()
            .any(|output| output.kind == SourceDocumentOutputKind::Program && output.visible)
    );
}

#[test]
fn canonical_pretty_text_spaces_formula_operators() {
    let source = "answer:=40+2\n~count:=0\ncount +=1\nresult:=make(1 ,2)\n";
    let formatted = CanonicalDocumentRenderer
        .format_pretty_text(&document(source))
        .unwrap();
    assert_eq!(
        formatted,
        "answer := 40 + 2\n~count := 0\ncount += 1\nresult := make(1, 2)\n"
    );
}

#[test]
fn canonical_pretty_text_spaces_context_send_operators() {
    for (source, expected) in [
        ("@out/line<-answer\n", "@out/line <- answer\n"),
        ("@out/line  <-\tanswer\r\n", "@out/line <- answer\r\n"),
        (
            "@out/line<-\"literal<-arrow\"\n",
            "@out/line <- \"literal<-arrow\"\n",
        ),
        (
            "```mech:worker\n@out/line<-40+2\n```\n",
            "```mech:worker\n@out/line <- 40 + 2\n```\n",
        ),
        ("@out/line<f64><-answer\n", "@out/line<f64> <- answer\n"),
        (
            "Send {{@out/line<-answer}}.\n",
            "Send {{@out/line <- answer}}.\n",
        ),
    ] {
        let formatted = CanonicalDocumentRenderer
            .format_pretty_text(&document(source))
            .unwrap();
        assert_eq!(formatted, expected, "{source:?}");
        assert_eq!(
            CanonicalDocumentRenderer
                .format_pretty_text(&streamed_document(source))
                .unwrap(),
            formatted,
            "streamed {source:?}"
        );
        assert_eq!(
            CanonicalDocumentRenderer
                .format_pretty_text(&document(&formatted))
                .unwrap(),
            formatted,
            "{source:?}"
        );
    }
}

#[test]
fn canonical_pretty_text_keeps_range_operators_attached_to_bounds() {
    for (source, expected) in [
        ("values:=1..3\n", "values := 1..3\n"),
        ("values:=1..=3\r\n", "values := 1..=3\r\n"),
        ("selected:=values[1..=3]\n", "selected := values[1..=3]\n"),
        ("values:=(1+2)..(3+4)\n", "values := (1 + 2)..(3 + 4)\n"),
        (
            "```mech:worker\nvalues:=1..3\nother:=1..=3\n```\n",
            "```mech:worker\nvalues := 1..3\nother := 1..=3\n```\n",
        ),
        (
            "Ranges {1..3} and {{1..=3}}.\n",
            "Ranges {1..3} and {{1..=3}}.\n",
        ),
    ] {
        let formatted = CanonicalDocumentRenderer
            .format_pretty_text(&document(source))
            .unwrap();
        assert_eq!(formatted, expected, "{source:?}");
        assert_eq!(
            CanonicalDocumentRenderer
                .format_pretty_text(&streamed_document(source))
                .unwrap(),
            formatted,
            "streamed {source:?}"
        );
        assert_eq!(
            CanonicalDocumentRenderer
                .format_pretty_text(&document(&formatted))
                .unwrap(),
            formatted,
            "{source:?}"
        );
    }
}

#[test]
fn canonical_pretty_text_spaces_record_and_map_separators() {
    let formatted = CanonicalDocumentRenderer
        .format_pretty_text(&document(
            "record:={ a:1,b:2 }\nmap:={ 1:2,3:4 }\nvector:=[ 1 2 ]\n",
        ))
        .unwrap();
    assert_eq!(
        formatted,
        "record := {a: 1, b: 2}\nmap := {1: 2, 3: 4}\nvector := [1 2]\n"
    );
}

#[test]
fn canonical_pretty_text_preserves_prefix_not_and_normalizes_tuple_destructuring() {
    let source = "answer:=!true\nother:=¬false\nnested:=!!true\n(x,y):=pair\n";
    let formatted = CanonicalDocumentRenderer
        .format_pretty_text(&document(source))
        .unwrap();
    assert_eq!(
        formatted,
        "answer := !true\nother := ¬false\nnested := !!true\n(x, y) := pair\n"
    );
    let reparsed = document(&formatted);
    assert_eq!(
        CanonicalDocumentRenderer
            .format_pretty_text(&reparsed)
            .unwrap(),
        formatted
    );
}

#[test]
fn canonical_pretty_text_formats_inline_code_without_changing_prose() {
    let source = "Result  {40+2}, literal {{x:=1}} and `40+2`.\n";
    let formatted = CanonicalDocumentRenderer
        .format_pretty_text(&document(source))
        .unwrap();
    assert_eq!(
        formatted,
        "Result  {40 + 2}, literal {{x := 1}} and `40+2`.\n"
    );
    let reparsed = document(&formatted);
    assert_eq!(
        CanonicalDocumentRenderer
            .format_pretty_text(&reparsed)
            .unwrap(),
        formatted
    );
}

#[test]
fn inert_diagram_fences_render_as_mermaid_containers() {
    let html = CanonicalDocumentRenderer
        .format_browser_html(&document("```diagram\ngraph LR\n  A --> B\n```\n"))
        .unwrap();
    assert!(
        html.contains("class='mech-diagram mermaid' data-mech-diagram"),
        "{html}"
    );
    assert!(html.contains("graph LR"), "{html}");
    assert!(!html.contains("data-language='diagram'"), "{html}");
}

#[test]
fn browser_shim_regions_preserve_metadata_navigation_and_section_boundaries() {
    let document = document(include_str!("../../../tests/fixtures/shims/all-slots.mec"));
    let slots = CanonicalDocumentRenderer
        .format_browser_html_slots(&document)
        .unwrap();
    for (name, expected) in [
        ("AUTHOR", "Ada Lovelace"),
        ("DATE", "July 30, 2026"),
        ("KICKER", "Announcement"),
        ("SECTION", "Compatibility"),
        ("SUMMARY", "Every supported shim slot must render."),
        ("HERO", "hero.svg"),
        ("NEXT", "next.html"),
        ("PREVIOUS", "previous.html"),
        ("ABSTRACT", "deliberately separate"),
        ("INTRO", "unsectioned introduction"),
        ("TOC", "href='#1'"),
        ("TOC", "href='#1.1'"),
        ("SECTION1", "own distinct content"),
        ("SECTION2", "must not appear"),
        ("FOOTNOTES", "Fixture footnote body"),
        ("CITED", "Mech Programming Language"),
    ] {
        assert!(
            slots.get(name).is_some_and(|html| html.contains(expected)),
            "{name}: {slots:#?}"
        );
    }
    assert!(slots["TOC"].contains("class='toc mech-toc'"));
    assert!(slots["TOC"].contains("</a><ul class='toc-sub'><li><a href='#1.1'"));
    assert!(slots["SECTION1"].contains("class='mechdown-section mechdown-titled-section'"));
    assert!(slots["FOOTNOTES"].contains("class='mech-footnotes'"));
    assert!(slots["FOOTNOTES"].contains("class='mech-backmatter-heading'>Footnotes</h3>"));
    assert!(slots["CONTENT"].contains("id='1'"));
    assert!(slots["CONTENT"].contains("id='1.1'"));
    assert_eq!(
        slots["CONTENT"]
            .matches("class='mechdown-section mechdown-titled-section'")
            .count(),
        2
    );
    assert!(!slots["SECTION1"].contains("must not appear"));
    assert!(!slots["CONTENT"].contains("unsectioned introduction"));
    assert!(!slots["INTRO"].contains("deliberately separate"));
    assert_eq!(slots["CONTENT"], slots["CONTENTS"]);
    assert!(slots["INTRO"].contains("{{TITLE}}"));
    assert!(slots["INTRO"].contains("mech-inline-mech-code"));
    assert!(slots["INTRO"].contains("data-mech-source"));
    assert!(slots["INTRO"].contains("class='mech-number'>41</span>"));
}

#[test]
fn served_particle_document_formats_its_annotated_compute_heading() {
    let source = include_str!("../../../examples/gpu-particles/particles.mec");
    let document = document(source);
    let slots = CanonicalDocumentRenderer
        .format_browser_html_slots(&document)
        .unwrap();
    assert!(slots["TOC"].contains("particle-field"));
    assert!(slots["CONTENT"].contains("particle-field"));
    assert!(!slots["TOC"].contains("@compute"), "{}", slots["TOC"]);
    assert!(
        slots["CONTENT"].contains("data-mech-annotations='@compute'"),
        "{}",
        slots["CONTENT"]
    );
    assert!(
        slots["CONTENT"].contains("class='mech-section-annotations'>@compute</span>"),
        "{}",
        slots["CONTENT"]
    );
}

#[test]
fn browser_titles_preserve_lf_and_crlf_source() {
    for ending in ["\n", "\r\n"] {
        let source = [
            "Browser Title",
            "=============",
            "section: Examples",
            "=============",
            "answer := 42",
            "The answer is {answer}.",
            "",
        ]
        .join(ending);
        let document = document(&source);
        let html = CanonicalDocumentRenderer
            .format_browser_html(&document)
            .unwrap();
        assert!(html.contains("<h1 class='mech-document-title'>Browser Title</h1>"));
        assert!(html.contains("Examples"));
        assert!(html.contains("mech-inline-mech-code"));
    }
    let source = include_str!("../../../examples/working/fizzbuzz.mec");
    let html = CanonicalDocumentRenderer
        .format_browser_html(&document(source))
        .unwrap();
    assert!(html.contains("<h1 class='mech-document-title'>Fizz Buzz</h1>"));
    assert!(html.contains("mech-block-output"));
}

#[test]
fn canonical_comments_preserve_literal_markup_in_each_scope() {
    let body = "answer := 40 + 2 -- **Result** [docs](https://mech-lang.org) `literal` {{answer + 99}}: {ans}, {ans + 1}.\nanswer + 2 // __Next__: {ans}, {ans + 10}.\n";
    for (source, named) in [
        (body.to_owned(), false),
        (format!("~~~mech\n{body}~~~\n"), false),
        (format!("root := 0\n~~~mech:example\n{body}~~~\n"), true),
    ] {
        for document in [document(&source), streamed_document(&source)] {
            let frontend = CanonicalSourceFrontend;
            let root = frontend.compile_document(&document).unwrap();
            let mut results = vec![execute(
                document.scope_id(),
                CanonicalRenderScope::Root,
                &root,
                91,
            )];
            if named {
                let program = frontend
                    .compile_named_document_scope(&document, "example")
                    .unwrap();
                results.push(execute(
                    document.scope_id(),
                    CanonicalRenderScope::Named("example".into()),
                    &program,
                    92,
                ));
            }
            let renderer = CanonicalDocumentRenderer;
            let html = renderer.render_html(&document, &results).unwrap();
            // The canonical comment production owns literal tokens through
            // the end of the line, including Mechdown-looking punctuation.
            assert!(html.contains("**Result** [docs](https://mech-lang.org) `literal` {{answer + 99}}: {ans}, {ans + 1}."), "{html}");
            assert!(html.contains("__Next__: {ans}, {ans + 10}."), "{html}");
            assert!(
                !html.contains("mech-strong") && !html.contains(">141</span>"),
                "{html}"
            );
            let text = renderer.render_text(&document, &results).unwrap();
            assert!(text.contains(": {ans}, {ans + 1}."), "{text}");
            assert!(text.contains(": {ans}, {ans + 10}."), "{text}");
            let browser = renderer.format_browser_html(&document).unwrap();
            assert_eq!(
                browser.matches("class='mech-inline-mech-code'").count(),
                0,
                "{browser}"
            );
        }
    }
}

#[test]
fn literal_comments_do_not_capture_ans_through_source_and_bytecode() {
    let document = document("~counter := 0\ncounter += 1 -- **Counter** {ans}\ncounter\n");
    let program = CanonicalSourceFrontend.compile_document(&document).unwrap();
    let artifact = program.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut instance = activate(
            ReactiveInstanceId::new(93, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for value in [1, 2, 3] {
            instance.turn(&[]).unwrap();
            let values = (0..artifact.outputs().len())
                .map(|output| {
                    RuntimeValueSnapshot::from_value(instance.copied_output(output).unwrap())
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let results = [CanonicalScopeResults::from_values(
                document.scope_id(),
                CanonicalRenderScope::Root,
                &program,
                &values,
            )
            .unwrap()];
            let html = CanonicalDocumentRenderer
                .render_html(&document, &results)
                .unwrap();
            assert!(html.contains("-- **Counter** {ans}"), "{html}");
            assert!(!html.contains("mech-inline-mech-code"), "{html}");
            assert!(html.contains(&format!("<output class='mech-program-output'><span class='mech-value'>{value}</span></output>")), "{html}");
        }
    }
}

fn assert_pretty_roundtrip(cases: &[(&str, &str)]) {
    for &(source, expected) in cases {
        let formatted = CanonicalDocumentRenderer
            .format_pretty_text(&document(source))
            .unwrap();
        assert_eq!(formatted, expected, "{source:?}");
        assert_eq!(
            CanonicalDocumentRenderer
                .format_pretty_text(&streamed_document(source))
                .unwrap(),
            formatted,
            "streamed {source:?}"
        );
        assert_eq!(
            CanonicalDocumentRenderer
                .format_pretty_text(&document(&formatted))
                .unwrap(),
            formatted,
            "idempotence {source:?}"
        );
    }
}

#[test]
fn canonical_pretty_text_normalizes_context_capability_groups() {
    assert_pretty_roundtrip(&[
        (
            "@io:=cli://stdout{:read(*),:write(line)}\n",
            "@io := cli://stdout { :read(*), :write(line) }\n",
        ),
        (
            "@io := @main {  :read(users/*) ,  :write(*) , }\n",
            "@io := @main { :read(users/*), :write(*), }\n",
        ),
        ("@io:=@main\n", "@io := @main\n"),
        (
            "```mech:worker\r\n@io:=@main{\r\n  :read(*),\r\n  :write(line)\r\n}\r\n```\r\n",
            "```mech:worker\r\n@io := @main {\r\n  :read(*),\r\n  :write(line)\r\n}\r\n```\r\n",
        ),
        (
            "Declaration {{@io:=@main{:read(*),:write(line)}}}.\n",
            "Declaration {{@io := @main { :read(*), :write(line) }}}.\n",
        ),
    ]);
}

#[test]
fn canonical_pretty_text_normalizes_named_call_bindings() {
    assert_pretty_roundtrip(&[
        ("result:=make(x:1,y:2)\n", "result := make(x: 1, y: 2)\n"),
        (
            "result:=make(x : other(y:2),note:\"x:1,y:2\",items:{a:1,b:2})\n",
            "result := make(x: other(y: 2), note: \"x:1,y:2\", items: {a: 1, b: 2})\n",
        ),
        (
            "Calls {make(x:1,y:2)} and {{make(x:1,y:2)}}.\r\n",
            "Calls {make(x: 1, y: 2)} and {{make(x: 1, y: 2)}}.\r\n",
        ),
    ]);
}

#[test]
fn canonical_pretty_text_normalizes_document_option_maps() {
    assert_pretty_roundtrip(&[
        (
            "~~~mech{output:false,color:red}\nx:=1\n~~~\n",
            "~~~mech{output: false, color: red}\nx := 1\n~~~\n",
        ),
        (
            "```text{color:\"red,blue:green\"}\nx:=1\n```\n",
            "```text{color: \"red,blue:green\"}\nx:=1\n```\n",
        ),
        (
            "![Caption  untouched](robot.png){width:wide,color:red}\n",
            "![Caption  untouched](robot.png){width: wide, color: red}\n",
        ),
        (
            "![Caption {1+2}](robot.png){color:red,width:wide}\n",
            "![Caption {1 + 2}](robot.png){color: red, width: wide}\n",
        ),
        (
            "| ![one](one.png){width:wide,color:red} | ![two](two.png){width:wide} |\r\n",
            "| ![one](one.png){width: wide, color: red} | ![two](two.png){width: wide} |\r\n",
        ),
    ]);
}

#[test]
fn browser_heading_annotations_require_complete_compact_identifiers() {
    for (title, annotation) in [
        ("Meet @ home today", None),
        ("Meet @home today", None),
        ("Meet @", None),
        ("Meet @123", None),
        ("Meet @home!", None),
        ("Meet @ home today @compute", Some("@compute")),
        ("Meet\t@compute", Some("@compute")),
        ("Meet @compute", Some("@compute")),
        ("Meet @gpu @cpu", Some("@gpu @cpu")),
        ("Meet @💡", Some("@💡")),
    ] {
        for ending in ["\n", "\r\n"] {
            let source = format!("1. {title}{ending}----------------{ending}");
            for doc in [document(&source), streamed_document(&source)] {
                let slots = CanonicalDocumentRenderer
                    .format_browser_html_slots(&doc)
                    .unwrap();
                let label = annotation.map_or(title, |suffix| {
                    title.strip_suffix(suffix).unwrap().trim_end()
                });
                assert!(
                    slots["TOC"].contains(&format!(">{label}</a>")),
                    "{title:?}: {}",
                    slots["TOC"]
                );
                if let Some(annotation) = annotation {
                    assert!(
                        slots["CONTENT"].contains(&format!("data-mech-annotations='{annotation}'")),
                        "{title:?}: {}",
                        slots["CONTENT"]
                    );
                } else {
                    assert!(
                        !slots["CONTENT"].contains("data-mech-annotations"),
                        "{title:?}: {}",
                        slots["CONTENT"]
                    );
                    assert!(
                        slots["CONTENT"].contains(&format!(">{label}</h2>")),
                        "{title:?}: {}",
                        slots["CONTENT"]
                    );
                }
            }
        }
    }
}

#[test]
fn figure_summary_copies_do_not_duplicate_live_output_mounts() {
    let doc = document("| ![one {1}](one.png) | ![two {2}](two.png) |\n");
    let renderer = CanonicalDocumentRenderer;
    let program = CanonicalSourceFrontend.compile_document(&doc).unwrap();
    let ranges = program
        .document_outputs()
        .iter()
        .filter(|binding| binding.kind == SourceDocumentOutputKind::Inline)
        .map(|binding| program.source_map().outputs[binding.output as usize].range)
        .collect::<Vec<_>>();
    assert_eq!(ranges.len(), 2);
    let addresses = ranges
        .iter()
        .enumerate()
        .map(|(index, range)| (*range, 100 + index as u64))
        .collect::<Vec<_>>();
    for html in [
        renderer.format_browser_html(&doc).unwrap(),
        renderer.format_html_body_live(&doc, &addresses).unwrap(),
        renderer.format_browser_html_slots(&doc).unwrap()["INTRO"].clone(),
    ] {
        let (panels, summary) = html
            .split_once("<figcaption class='mech-figure-table-caption'>")
            .unwrap();
        assert_eq!(panels.matches("id='").count(), 2, "{html}");
        assert!(!summary.contains("id='"), "{html}");
        assert!(!summary.contains("data-mech-output-address"), "{html}");
        assert!(!summary.contains("data-mech-source"), "{html}");
        assert!(
            summary.contains("two <code class='mech-inline'>{2}</code>"),
            "{html}"
        );
    }
    let results = [execute(
        doc.scope_id(),
        CanonicalRenderScope::Root,
        &program,
        910,
    )];
    let html = renderer.render_html(&doc, &results).unwrap();
    let (_, summary) = html
        .split_once("<figcaption class='mech-figure-table-caption'>")
        .unwrap();
    assert!(
        summary.contains(">1</span>") && summary.contains(">2</span>"),
        "{html}"
    );
}

#[test]
fn table_of_contents_copies_do_not_duplicate_live_heading_mounts() {
    let doc = document("1. Count {1}\n-------------\n");
    let slots = CanonicalDocumentRenderer
        .format_browser_html_slots(&doc)
        .unwrap();
    assert!(!slots["TOC"].contains("id='"), "{}", slots["TOC"]);
    assert!(
        !slots["TOC"].contains("data-mech-output-address"),
        "{}",
        slots["TOC"]
    );
    assert!(
        slots["TOC"].contains("<code class='mech-inline'>{1}</code>"),
        "{}",
        slots["TOC"]
    );
    assert_eq!(
        slots["CONTENT"].matches("data-mech-output-address").count(),
        1,
        "{}",
        slots["CONTENT"]
    );
}

#[test]
fn canonical_pretty_text_normalizes_tuple_value_and_tagged_pattern_separators() {
    assert_pretty_roundtrip(&[
        ("pair:=(1,2)\n", "pair := (1, 2)\n"),
        ("pair:=:some((1,2))\n", "pair := :some((1, 2))\n"),
        (
            "answer:=pair?\n| `some(x,y)=>x.\n",
            "answer := pair?\n| `some(x, y) => x.\n",
        ),
        (
            "answer:=pair?\n| :some(x,y)=>x.\n",
            "answer := pair?\n| :some(x, y) => x.\n",
        ),
    ]);
}

#[test]
fn canonical_pretty_text_normalizes_grouped_import_separators() {
    assert_pretty_roundtrip(&[
        ("+> math/{sin,cos}\n", "+> math/{sin, cos}\n"),
        ("+> math/{ sin , cos }\r\n", "+> math/{sin, cos}\r\n"),
        (
            "+> math/{sin,cos} -- keep a,b\ntext := \"sin,cos\"\n",
            "+> math/{sin, cos} -- keep a,b\ntext := \"sin,cos\"\n",
        ),
        (
            "Report\n======\n+> math/{sin,cos}\nauthor: Keep a,b\n======\n",
            "Report\n======\n+> math/{sin, cos}\nauthor: Keep a,b\n======\n",
        ),
    ]);
}

#[test]
fn repeated_section_numbers_have_distinct_heading_and_toc_anchors() {
    let source = "1. First\n---------\nFirst body.\n\n1. Second\n----------\nSecond body.\n\n(1.1) Child A\nChild body.\n\n(1.1) Child B\nOther child body.\n";
    for ending in ["\n", "\r\n"] {
        let source = source.replace('\n', ending);
        for doc in [document(&source), streamed_document(&source)] {
            let renderer = CanonicalDocumentRenderer;
            for slots in [
                renderer.format_browser_html_slots(&doc).unwrap(),
                renderer.format_static_html_slots(&doc).unwrap(),
            ] {
                let links = slots["TOC"]
                    .split("href='#")
                    .skip(1)
                    .map(|link| link.split_once('\'').unwrap().0)
                    .collect::<Vec<_>>();
                assert_eq!(links.len(), 4, "{}", slots["TOC"]);
                assert_eq!(
                    links
                        .iter()
                        .copied()
                        .collect::<std::collections::HashSet<_>>()
                        .len(),
                    4,
                    "{}",
                    slots["TOC"]
                );
                assert_eq!(links[0], "1");
                assert_eq!(links[2], "1.1");
                for html in [
                    slots["CONTENT"].clone(),
                    renderer.format_html(&doc).unwrap(),
                    renderer.format_browser_html(&doc).unwrap(),
                    renderer.format_html_body_live(&doc, &[]).unwrap(),
                ] {
                    for anchor in &links {
                        assert_eq!(html.matches(&format!("id='{anchor}'")).count(), 1, "{html}");
                    }
                    for heading in ["First", "Second", "Child A", "Child B"] {
                        assert!(html.contains(heading), "{html}");
                    }
                }
            }
        }
    }
}

#[test]
fn context_source_roles_and_long_capability_rows_survive_canonical_rendering() {
    let source = "@filters:=compute://filters/kernel{:read(sample/result.0),:read(sample/result.2),:read(turns),:write(input/control),:write(input/camera),:write(input/measurement),:write(turn)}\n";
    let expected = "@filters := compute://filters/kernel {\n  :read(sample/result.0),\n  :read(sample/result.2),\n  :read(turns),\n  :write(input/control),\n  :write(input/camera),\n  :write(input/measurement),\n  :write(turn)\n}\n";
    assert_pretty_roundtrip(&[(source, expected)]);
    let doc = document(&format!("{expected}0\n"));
    let renderer = CanonicalDocumentRenderer;
    for html in [
        renderer.format_html(&doc).unwrap(),
        renderer.render_repl_source_html(&doc).unwrap().unwrap(),
    ] {
        for role in [
            "mech-context-name",
            "mech-context-provider",
            "mech-context-scheme-op",
            "mech-context-path",
            "mech-context-capability",
            "mech-atom-sigil",
            "mech-atom-name",
        ] {
            assert!(html.contains(role), "missing {role}: {html}");
        }
        assert!(html.contains(">filters/kernel</span>"), "{html}");
        assert_eq!(
            html.matches("mech-context-capability-row").count(),
            7,
            "{html}"
        );
    }
    assert_pretty_roundtrip(&[
        (
            "name:=@browser/body/content/input/_value\n",
            "name := @browser/body/content/input/_value\n",
        ),
        (
            "@browser/body/content/output/_value=\"hello\"\n",
            "@browser/body/content/output/_value = \"hello\"\n",
        ),
    ]);
    let html = renderer
        .render_repl_source_html(&document("name := @out/line\n"))
        .unwrap()
        .unwrap();
    assert!(
        html.contains("class='mech-context-reference'>@out/line</span>"),
        "{html}"
    );
}

#[test]
fn canonical_presentation_repeated_inline_and_fence_addresses_resolve_distinct_values() {
    let source = "~x := 1\n\nFirst {x}.\n\n~~~mech\nx += 1\nx\n~~~\n\nSecond {x}.\n\n~~~mech\nx += 1\nx\n~~~\n";
    let doc = document(source);
    let program = CanonicalSourceFrontend.compile_document(&doc).unwrap();
    let result = execute(doc.scope_id(), CanonicalRenderScope::Root, &program, 913);
    let outputs = mech_runtime::canonical_document_presentation_outputs(&doc).unwrap();
    assert_eq!(outputs.len(), 4);
    let ids = outputs
        .iter()
        .map(|output| output.output_id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), outputs.len());
    let html = CanonicalDocumentRenderer.format_browser_html(&doc).unwrap();
    for output in &outputs {
        assert_eq!(
            html.matches(&format!("id='{}:0'", output.output_id))
                .count(),
            1,
            "{html}"
        );
    }
    let text = CanonicalDocumentRenderer
        .render_text(&doc, &[result])
        .unwrap();
    assert!(text.contains("First 1."), "{text}");
    assert!(text.contains("Second 2."), "{text}");
    assert_eq!(text.matches("=> 2").count(), 1, "{text}");
    assert_eq!(text.matches("=> 3").count(), 1, "{text}");
    assert_eq!(
        mech_core::root_document_program_output_id(),
        mech_runtime::canonical_document_output_id(
            SourceDocumentOutputKind::Program,
            doc.syntax().range()
        )
    );
}

#[test]
fn pretty_format_preserves_independently_expected_executable_results() {
    for source in [
        "pair:=(2,3)\n(left,right):=pair\nleft * 10 + right\n",
        "record:={left:2,right:3}\nrecord.left * 10 + record.right\n",
        "answer:=math/sub(left:30,right:7)\nanswer\n",
    ] {
        let before = document(source);
        let formatted = CanonicalDocumentRenderer
            .format_pretty_text(&before)
            .unwrap();
        let after = document(&formatted);
        for (instance, doc) in [(914, before), (915, after)] {
            let program = CanonicalSourceFrontend.compile_document(&doc).unwrap();
            let result = execute(
                doc.scope_id(),
                CanonicalRenderScope::Root,
                &program,
                instance,
            );
            let text = CanonicalDocumentRenderer
                .render_text(&doc, &[result])
                .unwrap();
            assert!(text.ends_with("=> 23\n"), "{formatted}: {text}");
        }
    }
}

#[test]
fn canonical_source_styles_preserve_atom_matrix_and_table_roles() {
    let renderer = CanonicalDocumentRenderer;
    let variables = renderer
        .format_html(&document("café := 1\ncafé\n"))
        .unwrap();
    assert_eq!(
        variables.matches("class='mech-var-name'").count(),
        2,
        "{variables}"
    );
    assert_eq!(
        variables.matches("data-mech-var-name='café'").count(),
        2,
        "{variables}"
    );
    assert!(
        renderer
            .render_repl_source_html(&document("café := 1\ncafé\n"))
            .unwrap()
            .unwrap()
            .contains("data-mech-var-name='café'")
    );
    for (source, roles) in [
        (
            "status := :ready\n",
            &["mech-atom", "mech-atom-sigil", "mech-atom-name"][..],
        ),
        (
            "values<[u8]:4,4> := [1 2;3 4]\n",
            &["mech-matrix-size-colon", "mech-matrix-size-separator"][..],
        ),
        (
            "scene := |id<string> x<f64>|\n         | \"robot\" 1 |\n",
            &["mech-statement-table-define", "mech-variable-define-table"][..],
        ),
    ] {
        let doc = document(source);
        assert_eq!(renderer.format_pretty_text(&doc).unwrap(), source);
        let html = renderer.format_html(&doc).unwrap();
        for role in roles {
            assert!(html.contains(role), "missing {role}: {html}");
        }
    }
}

#[test]
fn canonical_pretty_frontmatter_preserves_hero_and_compute_metadata() {
    for source in [
        "Gallery\n=======\nhero: | ![First](first.svg) | ![Second](second.svg) |\n=======\n",
        "particle update @gpu @required(:finite)\n--------------------------------------\nx := 1\n",
    ] {
        assert_pretty_roundtrip(&[(source, source)]);
    }
    let doc = document("Gallery\n=======\nhero: | ![Result {40 + 2}](first.svg) |\n=======\n");
    let slots = CanonicalDocumentRenderer
        .format_browser_html_slots(&doc)
        .unwrap();
    assert_eq!(slots["HERO"].matches("data-mech-output-address").count(), 1);
    assert!(slots["HERO"].contains("mech-subfigure-caption"));
}

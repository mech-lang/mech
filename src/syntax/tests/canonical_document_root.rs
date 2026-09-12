use std::fs;
use std::path::PathBuf;

use mech_syntax::document::parser::canonical::parse_canonical_document_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, CodeBlockSyntax, DocumentId, DocumentSyntax, ParseConfig, ParseLimits,
    RecursiveSyntaxNode, Revision, SyntaxKind, SyntaxNode, TextSnapshot, compact_debug_tree,
    parse_canonical_document, reconstruct_source, validate_lossless,
};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(0x2d7), Revision(1), text).unwrap()
}

fn piece_source(parts: &[&str]) -> TextSnapshot {
    let mut snapshot = TextSnapshot::new(DocumentId(0x2d8), Revision(1), "").unwrap();
    for part in parts {
        snapshot = snapshot.append((*part).to_owned()).unwrap();
    }
    snapshot
}

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .map(|child| count(&child, kind))
            .sum::<usize>()
}

#[test]
fn canonical_document_root_owns_the_whole_source_fixture_corpus() {
    let matrix = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/source-parser-consumer-fixtures.tsv"),
    )
    .unwrap();
    for row in matrix.lines().skip(1) {
        let fields = row.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 6, "invalid fixture row: {row}");
        let text = fs::read_to_string(repository_root().join(fields[1])).unwrap();
        let snapshot = parse_canonical_document(source(&text), ParseConfig::default());
        validate_lossless(&snapshot.root, &snapshot.source).unwrap();
        assert_eq!(
            reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
            text,
            "{}",
            fields[0]
        );
        assert_eq!(snapshot.syntax().kind(), SyntaxKind::Document);
        let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
        if fields[3] == "accept" {
            assert!(
                snapshot.diagnostics.is_empty(),
                "{}: {:#?}",
                fields[0],
                snapshot.diagnostics.as_slice()
            );
            if fields[0] != "empty" {
                assert!(!document.sections().is_empty(), "{}", fields[0]);
            }
        } else {
            assert!(!snapshot.diagnostics.is_empty(), "{}", fields[0]);
        }
    }
}

#[test]
fn canonical_document_is_the_supported_document_root() {
    let parsed = mech_syntax::document::parse_syntax(
        source("answer := 42\n"),
        mech_syntax::document::ParseRoot::Document,
        mech_syntax::document::ParserImplementation::Canonical,
        ParseConfig::default(),
    )
    .unwrap();
    assert_eq!(parsed.syntax().kind(), SyntaxKind::Document);
}

#[test]
fn typed_document_exposes_title_front_matter_sections_and_fences() {
    let text = fs::read_to_string(
        repository_root().join("tests/fixtures/syntax-source-boundary/document.mec"),
    )
    .unwrap();
    let snapshot = parse_canonical_document(source(&text), ParseConfig::default());
    let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
    let title = document.title().expect("title");
    let front_matter = title.front_matter().expect("title front matter");
    let keys = front_matter
        .keys()
        .into_iter()
        .map(|key| key.syntax().text().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["author", "section"]);
    assert!(!document.sections().is_empty());

    let fence = find(snapshot.syntax(), SyntaxKind::CodeBlock)
        .and_then(CodeBlockSyntax::cast)
        .expect("fenced code block");
    let delimiters = fence
        .delimiters()
        .into_iter()
        .map(|token| token.text().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(delimiters, ["~~~", "~~~"]);
}

#[test]
fn mismatched_fence_is_lossless_and_diagnostic() {
    let text = "```mech\nanswer\n~~~\n";
    let snapshot = parse_canonical_document(source(text), ParseConfig::default());
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
    assert!(!snapshot.diagnostics.is_empty());
    let fence = find(snapshot.syntax(), SyntaxKind::CodeBlock)
        .and_then(CodeBlockSyntax::cast)
        .expect("mismatched fence must retain typed code-block structure");
    assert_eq!(fence.delimiters().len(), 2);
    let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
    assert!(
        document
            .missing_tokens()
            .iter()
            .any(|token| token.kind() == SyntaxKind::GraveCodeBlockSigil)
    );
}

#[test]
fn committed_statement_recovery_is_not_replaced_by_a_short_expression() {
    let snapshot = parse_canonical_document(source("x :=\n"), ParseConfig::default());
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert!(!snapshot.diagnostics.is_empty());
    assert!(find(snapshot.syntax(), SyntaxKind::VariableDefine).is_some());
}

#[test]
fn comment_selection_preserves_complete_recursive_negation() {
    let expression = parse_canonical_document(source("--x\n"), ParseConfig::default());
    assert!(
        expression.is_strictly_clean(),
        "{:#?}",
        expression.diagnostics
    );
    assert_eq!(count(&expression.syntax(), SyntaxKind::NegateFactor), 2);
    assert_eq!(count(&expression.syntax(), SyntaxKind::Comment), 0);

    let comment = parse_canonical_document(source("-- note\n"), ParseConfig::default());
    assert!(comment.is_strictly_clean(), "{:#?}", comment.diagnostics);
    assert_eq!(count(&comment.syntax(), SyntaxKind::Comment), 1);
    assert_eq!(count(&comment.syntax(), SyntaxKind::Expression), 0);
}

#[test]
fn comment_selection_preserves_whitespace_and_committed_expression_recovery() {
    let comment = parse_canonical_document_rule_for_test(
        source("\n  -- note"),
        rules::MECH_CODE_ALT,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(comment.is_strictly_clean(), "{:#?}", comment.diagnostics);
    let comment_node = find(comment.syntax(), SyntaxKind::Comment).expect("typed comment");
    assert_eq!(comment_node.range().start.0, 3);

    let expression = parse_canonical_document(source("--x +\n"), ParseConfig::default());
    assert!(!expression.is_strictly_clean());
    assert_eq!(count(&expression.syntax(), SyntaxKind::NegateFactor), 2);
    assert_eq!(count(&expression.syntax(), SyntaxKind::Comment), 0);
}

#[test]
fn distinctive_document_openers_recover_required_closers() {
    let mika = parse_canonical_document(source("~∘~⸢text\n"), ParseConfig::default());
    assert!(!mika.is_strictly_clean());
    assert!(find(mika.syntax(), SyntaxKind::MikaSection).is_some());
    assert!(
        mika.diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code.as_str() == "syntax/missing-mika-section-closer" })
    );

    let inline = parse_canonical_document(source("Text {{x := 1}\n"), ParseConfig::default());
    assert!(!inline.is_strictly_clean());
    assert!(find(inline.syntax(), SyntaxKind::InlineMechCode).is_some());
    assert!(
        inline
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code.as_str() == "syntax/missing-inline-mech-closer" })
    );

    for text in ["```", "```mech"] {
        let fence = parse_canonical_document(source(text), ParseConfig::default());
        assert!(!fence.is_strictly_clean(), "{text:?}");
        assert!(find(fence.syntax(), SyntaxKind::CodeBlock).is_some());
        assert!(fence.diagnostics.iter().any(|diagnostic| {
            diagnostic.code.as_str() == "syntax/missing-codeblock-header-newline"
        }));
    }
}

#[test]
fn consecutive_underlined_subtitles_start_distinct_sections() {
    let snapshot = parse_canonical_document(
        source("1.First\n-----\n2.Second\n------\n"),
        ParseConfig::default(),
    );
    assert!(snapshot.is_strictly_clean(), "{:#?}", snapshot.diagnostics);
    let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
    assert_eq!(document.sections().len(), 2);
    assert_eq!(count(document.syntax(), SyntaxKind::UlSubtitle), 2);
}

#[test]
fn document_resource_limits_remain_hard_and_lossless() {
    let text = "x := [1, 2, 3]\n".repeat(64);
    let limits = ParseLimits {
        fuel: 64,
        max_events: 96,
        max_diagnostics: 4,
        max_recovery_bytes: 4_096,
        ..ParseLimits::default()
    };
    let snapshot = parse_canonical_document(source(&text), ParseConfig { limits });
    assert!(snapshot.stats.events_emitted <= u64::from(limits.max_events));
    assert!(snapshot.diagnostics.len() <= limits.max_diagnostics as usize);
    validate_lossless(&snapshot.root, &snapshot.source).unwrap();
    assert_eq!(
        reconstruct_source(&snapshot.root, &snapshot.source).unwrap(),
        text
    );
}

#[test]
fn piece_backed_documents_match_contiguous_documents() {
    let text = "# Demo\n\n~~~mech\nanswer := 42\n~~~\n";
    let contiguous = parse_canonical_document(source(text), ParseConfig::default());
    let piece_backed = parse_canonical_document(
        piece_source(&["# De", "mo\n\n~", "~~mech\nans", "wer := 42\n~~", "~\n"]),
        ParseConfig::default(),
    );
    assert_eq!(
        compact_debug_tree(&piece_backed.syntax()),
        compact_debug_tree(&contiguous.syntax())
    );
    assert_eq!(
        piece_backed.diagnostics.as_slice(),
        contiguous.diagnostics.as_slice()
    );
    validate_lossless(&piece_backed.root, &piece_backed.source).unwrap();
}

#[test]
fn repeated_document_units_remain_measured_linear() {
    const SLACK: u64 = 2_048;
    let measurements = [8_usize, 16, 32]
        .into_iter()
        .map(|size| {
            let text = "answer := 1\n".repeat(size);
            let parsed = parse_canonical_document(source(&text), ParseConfig::default());
            assert!(
                parsed.is_strictly_clean(),
                "{size}: {:#?}",
                parsed.diagnostics
            );
            validate_lossless(&parsed.root, &parsed.source).unwrap();
            (parsed.stats.parser_steps, parsed.stats.events_emitted)
        })
        .collect::<Vec<_>>();
    for pair in measurements.windows(2) {
        let (small_steps, small_events) = pair[0];
        let (large_steps, large_events) = pair[1];
        assert!(
            large_steps <= small_steps.saturating_mul(2).saturating_add(SLACK),
            "parser steps were not linear: {measurements:?}"
        );
        assert!(
            large_events <= small_events.saturating_mul(2).saturating_add(SLACK),
            "parser events were not linear: {measurements:?}"
        );
    }
}

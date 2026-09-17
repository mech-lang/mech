use mech_syntax::document::{
    DocumentId, FragmentKind, IdGenerator, ParseConfig, ParseContext, Revision, SyntaxKind,
    SyntaxNode, TextRange, TextSize, TextSnapshot, compact_debug_tree, parse_canonical_grammar,
    parse_fragment, reconstruct_source_range, validate_lossless_range,
};

fn nodes_of_kind(root: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    let mut nodes = Vec::new();
    if root.kind() == kind {
        nodes.push(root.clone());
    }
    for child in root.children() {
        nodes.extend(nodes_of_kind(&child, kind));
    }
    nodes
}

fn global_source(fragment: &str) -> (TextSnapshot, TextRange) {
    let prefix = "physical-prefix:";
    let suffix = ":right-context";
    let text = format!("{prefix}{fragment}{suffix}");
    let range = TextRange::new(
        TextSize(prefix.len() as u32),
        TextSize((prefix.len() + fragment.len()) as u32),
    );
    (
        TextSnapshot::new(DocumentId(81), Revision(12), text).unwrap(),
        range,
    )
}

fn parse_whole(text: &str) -> mech_syntax::document::SyntaxSnapshot {
    parse_canonical_grammar(
        TextSnapshot::new(DocumentId(82), Revision(12), text).unwrap(),
        ParseConfig::default(),
    )
}

fn assert_fragment(kind: FragmentKind, fragment: &str, whole_source: &str, occurrence: usize) {
    let (source, range) = global_source(fragment);
    let mut ids = IdGenerator::new();
    let parsed = parse_fragment(
        &source,
        range,
        kind,
        ParseContext::for_kind(kind),
        ParseConfig::default(),
        &mut ids,
    );

    assert!(parsed.matched, "{kind:?} did not match");
    assert!(
        parsed.consumed_complete,
        "{kind:?} did not consume exactly its bounded range: {:?}",
        parsed.consumed
    );
    assert_eq!(parsed.source.document(), DocumentId(81));
    assert_eq!(parsed.source.revision(), Revision(12));
    assert_eq!(parsed.range, range);
    assert_eq!(parsed.consumed, range);
    assert_eq!(parsed.root.kind, kind.syntax_kind());
    assert_eq!(parsed.syntax().range(), range);
    assert_eq!(parsed.syntax().text().unwrap(), fragment);
    assert!(
        parsed
            .syntax()
            .tokens()
            .iter()
            .all(|token| range.contains_range(token.range())),
        "{kind:?} emitted a token outside its global source range"
    );
    validate_lossless_range(&parsed.root, &parsed.source, range).unwrap();
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, range).unwrap(),
        fragment
    );

    let whole = parse_whole(whole_source);
    assert!(
        whole.diagnostics.is_empty(),
        "{kind:?} whole-root comparison source did not parse: {:#?}",
        whole.diagnostics.as_slice()
    );
    let expected = nodes_of_kind(&whole.syntax(), kind.syntax_kind())
        .into_iter()
        .nth(occurrence)
        .unwrap_or_else(|| panic!("whole parse has no {kind:?} occurrence {occurrence}"));
    assert_eq!(expected.text().unwrap(), fragment);
    assert_eq!(
        compact_debug_tree(&parsed.syntax()),
        compact_debug_tree(&expected),
        "{kind:?} fragment tree differs from the whole-root tree"
    );
}

#[test]
fn all_six_grammar_fragment_roots_use_global_bounded_ranges() {
    assert_fragment(
        FragmentKind::Grammar,
        "one:=\"a\";two:=one;",
        "one:=\"a\";two:=one;",
        0,
    );
    assert_fragment(
        FragmentKind::GrammarRule,
        "target:=\"a\";",
        "target:=\"a\";next:=\"b\";",
        0,
    );
    assert_fragment(
        FragmentKind::GrammarExpression,
        "\"a\"|other",
        "target:=\"a\"|other;",
        0,
    );
    assert_fragment(
        FragmentKind::GrammarTerm,
        "\"a\",other",
        "target:=\"a\",other;",
        0,
    );
    assert_fragment(FragmentKind::GrammarFactor, "?other", "target:=?other;", 0);
    assert_fragment(
        FragmentKind::GrammarTerminalToken,
        "\"ab\"",
        "target:=\"ab\";",
        0,
    );
}

#[test]
fn grammar_fragment_context_mode_is_required() {
    let (source, range) = global_source("\"a\"|other");
    let mut ids = IdGenerator::new();
    let parsed = parse_fragment(
        &source,
        range,
        FragmentKind::GrammarExpression,
        ParseContext {
            mode: mech_syntax::document::ParseMode::Mech,
            ..ParseContext::for_kind(FragmentKind::GrammarExpression)
        },
        ParseConfig::default(),
        &mut ids,
    );
    assert!(!parsed.matched);
    assert!(!parsed.consumed_complete);
}

#[test]
fn grammar_rule_fragment_uses_right_context_without_consuming_it() {
    let text = "prefix:target:=\"a\";next:=\"b\";";
    let start = text.find("target").unwrap();
    let end = text.find("next").unwrap();
    let source = TextSnapshot::new(DocumentId(81), Revision(12), text).unwrap();
    let range = TextRange::new(TextSize(start as u32), TextSize(end as u32));
    let mut ids = IdGenerator::new();
    let parsed = parse_fragment(
        &source,
        range,
        FragmentKind::GrammarRule,
        ParseContext::for_kind(FragmentKind::GrammarRule),
        ParseConfig::default(),
        &mut ids,
    );

    assert!(parsed.matched);
    assert!(parsed.consumed_complete);
    assert_eq!(parsed.consumed.end, TextSize(end as u32));
    assert_eq!(parsed.syntax().text().unwrap(), "target:=\"a\";");
    assert!(
        parsed
            .syntax()
            .tokens()
            .iter()
            .all(|token| token.range().end <= TextSize(end as u32))
    );
}

#[test]
fn grammar_fragment_diagnostic_can_inspect_bounded_right_context() {
    let text = "prefix:target:=\"a\"next:=\"b\";";
    let start = text.find("target").unwrap();
    let end = text.find("next").unwrap();
    let source = TextSnapshot::new(DocumentId(81), Revision(12), text).unwrap();
    let range = TextRange::new(TextSize(start as u32), TextSize(end as u32));
    let mut ids = IdGenerator::new();
    let parsed = parse_fragment(
        &source,
        range,
        FragmentKind::GrammarRule,
        ParseContext::for_kind(FragmentKind::GrammarRule),
        ParseConfig::default(),
        &mut ids,
    );

    assert!(parsed.matched);
    assert!(parsed.consumed_complete);
    let diagnostic = parsed
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "syntax/missing-semicolon")
        .expect("missing semicolon diagnostic");
    assert_eq!(
        diagnostic
            .found
            .as_ref()
            .and_then(|found| found.text.as_deref()),
        Some("n")
    );
    assert_eq!(parsed.consumed.end, TextSize(end as u32));
    assert_eq!(parsed.syntax().text().unwrap(), "target:=\"a\"");
}

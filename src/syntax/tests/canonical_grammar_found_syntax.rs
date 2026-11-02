use mech_syntax::document::{
    DocumentId, FoundSyntax, ParseConfig, Revision, SyntaxKind, TextSnapshot,
    parse_canonical_grammar,
};

fn first_found(text: &str) -> FoundSyntax {
    let snapshot = parse_canonical_grammar(
        TextSnapshot::new(DocumentId(91), Revision(3), text).unwrap(),
        ParseConfig::default(),
    );
    snapshot
        .diagnostics
        .iter()
        .find_map(|diagnostic| diagnostic.found.clone())
        .unwrap_or_else(|| panic!("expected a diagnostic with found syntax for {text:?}"))
}

fn assert_found(source: &str, kind: SyntaxKind, text: Option<&str>) {
    assert_eq!(
        first_found(source),
        FoundSyntax {
            kind: Some(kind),
            text: text.map(str::to_owned),
        },
        "wrong canonical found syntax for {source:?}",
    );
}

#[test]
fn canonical_grammar_diagnostics_classify_exact_terminals() {
    for (source, kind, text) in [
        ("@", SyntaxKind::At, "@"),
        ("\"", SyntaxKind::Quote, "\""),
        ("[", SyntaxKind::LeftBracket, "["),
        ("]", SyntaxKind::RightBracket, "]"),
        ("|", SyntaxKind::Bar, "|"),
        (",", SyntaxKind::Comma, ","),
        (":=", SyntaxKind::DefineOperatorToken, ":="),
        ("¬", SyntaxKind::Not, "¬"),
    ] {
        assert_found(source, kind, Some(text));
    }
}

#[test]
fn canonical_grammar_diagnostics_use_the_longest_filtered_terminal() {
    assert_found(" \n : \t =", SyntaxKind::DefineOperatorToken, Some(":="));
}

#[test]
fn canonical_grammar_diagnostics_classify_fallback_graphemes_and_eof() {
    assert_found("rule := [\"x\" Δ];", SyntaxKind::Alpha, Some("Δ"));
    assert_found("rule := [\"x\" ٣];", SyntaxKind::Digit, Some("٣"));
    assert_found("rule := [\"x\" 👩‍🔬];", SyntaxKind::Emoji, Some("👩‍🔬"));
    assert_found(
        "rule := [\"x\" .\u{301}];",
        SyntaxKind::Any,
        Some(".\u{301}"),
    );
    assert_found("rule :=", SyntaxKind::Eof, None);
}

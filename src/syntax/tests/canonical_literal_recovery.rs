use mech_syntax::document::parser::canonical::parse_canonical_phase_2c_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ExpectedSyntax, FixApplicability, ParseConfig, RecoveryAction, Revision, RuleId,
    SyntaxKind, SyntaxNode, TextRange, TextSize, TextSnapshot, TokenFlags,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(921), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2c_rule_for_test(source(text), rule, ParseConfig::default()).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

#[test]
fn string_selection_rewinds_an_incomplete_raw_candidate_before_utf8_prefix() {
    let parsed = parse("\"\"\"abc", rules::STRING);
    assert!(parsed.is_strictly_clean());
    assert_eq!(parsed.consumed, TextRange::new(TextSize::ZERO, TextSize(2)));
    assert!(find_node(&parsed.syntax(), SyntaxKind::Utf8String).is_some());
    assert!(find_node(&parsed.syntax(), SyntaxKind::RawString).is_none());
    assert!(parsed.diagnostics.is_empty());

    let four_quotes = parse("\"\"\"\"", rules::STRING);
    assert!(four_quotes.is_strictly_clean());
    assert_eq!(four_quotes.consumed.end.0, 2);

    let raw = parse("\"\"\"\"\"\"", rules::STRING);
    assert!(raw.is_strictly_clean());
    assert_eq!(raw.consumed.end.0, 6);
    assert!(find_node(&raw.syntax(), SyntaxKind::RawString).is_some());
}

#[test]
fn direct_unclosed_strings_emit_only_their_own_structured_recovery() {
    let utf8 = parse("\"", rules::UTF8_STRING);
    assert!(utf8.matched);
    let diagnostic = utf8.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/unclosed-utf8-string");
    assert_eq!(
        diagnostic.expected,
        vec![ExpectedSyntax::Token(SyntaxKind::Quote)]
    );
    assert_eq!(
        diagnostic.recovery,
        Some(RecoveryAction::Insert {
            syntax: ExpectedSyntax::Token(SyntaxKind::Quote),
            at: TextSize(1),
        })
    );
    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(
        diagnostic.fixes[0].applicability,
        FixApplicability::MachineApplicable
    );

    let raw = parse("\"\"\"unterminated", rules::RAW_STRING);
    assert!(raw.matched);
    let diagnostic = raw.diagnostics.iter().next().unwrap();
    assert_eq!(diagnostic.code.as_str(), "syntax/unclosed-raw-string");
    assert_eq!(
        diagnostic.expected,
        vec![ExpectedSyntax::Production("triple closing quote".into())]
    );
    let missing = find_node(&raw.syntax(), SyntaxKind::Missing).unwrap();
    let missing_quotes = missing
        .tokens()
        .into_iter()
        .filter(|token| token.kind() == SyntaxKind::Quote)
        .collect::<Vec<_>>();
    assert_eq!(missing_quotes.len(), 3);
    assert!(missing_quotes.iter().all(|token| {
        token.range() == TextRange::empty(TextSize(15))
            && token.flags().contains(TokenFlags::MISSING)
    }));
}

#[test]
fn based_prefixes_commit_missing_payload_recovery_with_physical_found_syntax() {
    for (rule, input, code, expected, at) in [
        (
            rules::DECIMAL_LITERAL,
            "0d",
            "syntax/missing-decimal-digits",
            "decimal digits",
            2,
        ),
        (
            rules::DECIMAL_LITERAL,
            "0dX",
            "syntax/missing-decimal-digits",
            "decimal digits",
            2,
        ),
        (
            rules::HEXADECIMAL_LITERAL,
            "0x",
            "syntax/missing-hexadecimal-digits",
            "hexadecimal digits",
            2,
        ),
        (
            rules::OCTAL_LITERAL,
            "0o",
            "syntax/missing-octal-digits",
            "octal digits",
            2,
        ),
        (
            rules::BINARY_LITERAL,
            "0b",
            "syntax/missing-binary-digits",
            "binary digits",
            2,
        ),
    ] {
        let parsed = parse(input, rule);
        assert!(parsed.matched, "{input:?}");
        let diagnostic = parsed.diagnostics.iter().next().unwrap();
        assert_eq!(diagnostic.code.as_str(), code, "{input:?}");
        assert_eq!(
            diagnostic.expected,
            vec![ExpectedSyntax::Production(expected.into())],
            "{input:?}",
        );
        assert_eq!(
            diagnostic.recovery,
            Some(RecoveryAction::Insert {
                syntax: ExpectedSyntax::Production(expected.into()),
                at: TextSize(at),
            }),
            "{input:?}",
        );
        assert!(diagnostic.fixes.is_empty(), "{input:?}");
    }
}

#[test]
fn missing_hexadecimal_payload_is_a_production_without_a_synthetic_digit() {
    let parsed = parse("0x", rules::HEXADECIMAL_LITERAL);
    let missing = find_node(&parsed.syntax(), SyntaxKind::Missing)
        .expect("the missing hexadecimal payload has a MISSING node");

    assert!(
        missing.children_with_tokens().is_empty(),
        "a hexadecimal payload is a production, so recovery must not invent a digit token",
    );
    assert!(
        missing.tokens().is_empty(),
        "the MISSING node must contain no synthetic tokens",
    );
}

#[test]
fn incomplete_losing_candidates_restore_without_diagnostics() {
    for (rule, input) in [
        (rules::ATOM, ":"),
        (rules::RATIONAL_LITERAL, "1/"),
        (rules::FLOAT_FULL, "1."),
        (rules::COMPLEX_NUMBER, "1+"),
    ] {
        let parsed = parse(input, rule);
        assert!(!parsed.matched, "{rule:?} on {input:?}");
        assert!(parsed.diagnostics.is_empty(), "{rule:?} on {input:?}");
        assert_eq!(
            parsed.consumed,
            TextRange::empty(TextSize::ZERO),
            "{input:?}"
        );
    }
}

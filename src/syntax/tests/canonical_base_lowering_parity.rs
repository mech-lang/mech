use mech_core::{Identifier, Token, TokenKind};
use mech_syntax::document::parser::canonical::parse_canonical_base_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSize, TextSnapshot,
    lower_legacy_digit_sequence, lower_legacy_identifier, lower_legacy_identifier_path_segment,
};

#[derive(Debug, Eq, PartialEq)]
struct NormalizedToken {
    kind: TokenKind,
    text: String,
    start: (usize, usize),
    end: (usize, usize),
}

fn normalize_token(token: &Token) -> NormalizedToken {
    NormalizedToken {
        kind: token.kind,
        text: token.chars.iter().collect(),
        start: (token.src_range.start.row, token.src_range.start.col),
        end: (token.src_range.end.row, token.src_range.end.col),
    }
}

fn normalize_tokens(tokens: &[Token]) -> Vec<NormalizedToken> {
    tokens.iter().map(normalize_token).collect()
}

fn normalize_identifier(identifier: &Identifier) -> NormalizedToken {
    normalize_token(&identifier.name)
}

fn canonical_node(
    input: &str,
    rule: RuleId,
    kind: SyntaxKind,
    expected_prefix: &str,
) -> SyntaxNode {
    let source = TextSnapshot::new(DocumentId(81), Revision(0), input).unwrap();
    let parsed = parse_canonical_base_rule_for_test(source, rule, ParseConfig::default())
        .expect("Phase 2A base rule must have a canonical parser");
    assert!(parsed.matched, "{rule:?} did not match {input:?}");
    assert!(parsed.diagnostics.is_empty(), "{rule:?} on {input:?}");
    assert_eq!(
        parsed.consumed.end,
        TextSize(expected_prefix.len() as u32),
        "{rule:?} consumed the wrong prefix of {input:?}"
    );
    let node = parsed
        .syntax()
        .first_child(kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"));
    assert_eq!(node.text().unwrap(), expected_prefix);
    node
}

#[test]
fn digit_sequence_values_match_the_legacy_parser() {
    for input in ["1_024", "٣_٤"] {
        let syntax = canonical_node(
            input,
            rules::DIGIT_SEQUENCE,
            SyntaxKind::DigitSequence,
            input,
        );
        let canonical = lower_legacy_digit_sequence(&syntax).unwrap();

        let graphemes = mech_syntax::graphemes::init_tag(input);
        let (remaining, legacy) =
            mech_syntax::digit_sequence(mech_syntax::ParseString::new(&graphemes)).unwrap();
        assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
        assert!(remaining.error_log.is_empty(), "{input:?}");
        assert_eq!(
            normalize_tokens(&canonical),
            normalize_tokens(&legacy),
            "{input:?}"
        );
    }
}

#[test]
fn identifier_values_match_the_legacy_parser() {
    for input in ["a-b", "a/b", "A*", "Δx^2", "💡identifier", "🧑🏽‍🔬-Δ2"] {
        let syntax = canonical_node(input, rules::IDENTIFIER, SyntaxKind::Identifier, input);
        let canonical = lower_legacy_identifier(&syntax).unwrap();

        let graphemes = mech_syntax::graphemes::init_tag(input);
        let (remaining, legacy) =
            mech_syntax::identifier(mech_syntax::ParseString::new(&graphemes)).unwrap();
        assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
        assert!(remaining.error_log.is_empty(), "{input:?}");
        assert_eq!(
            normalize_identifier(&canonical),
            normalize_identifier(&legacy),
            "{input:?}"
        );
    }
}

#[test]
fn identifier_path_segment_values_and_boundaries_match_the_legacy_parser() {
    for (input, expected_prefix) in [
        ("a-b", "a-b"),
        ("a/b", "a"),
        ("A*", "A"),
        ("Δx^2", "Δx"),
        ("💡segment", "💡segment"),
        ("🧑🏽‍🔬-Δ2", "🧑🏽‍🔬-Δ2"),
    ] {
        let syntax = canonical_node(
            input,
            rules::IDENTIFIER_PATH_SEGMENT,
            SyntaxKind::IdentifierPathSegment,
            expected_prefix,
        );
        let canonical = lower_legacy_identifier_path_segment(&syntax).unwrap();

        let graphemes = mech_syntax::graphemes::init_tag(input);
        let (remaining, legacy) =
            mech_syntax::identifier_path_segment(mech_syntax::ParseString::new(&graphemes))
                .unwrap();
        assert_eq!(
            graphemes[..remaining.cursor].concat(),
            expected_prefix,
            "{input:?}"
        );
        assert!(remaining.error_log.is_empty(), "{input:?}");
        assert_eq!(
            normalize_identifier(&canonical),
            normalize_identifier(&legacy),
            "{input:?}"
        );
    }
}

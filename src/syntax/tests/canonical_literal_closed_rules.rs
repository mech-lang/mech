use mech_syntax::document::parser::canonical::parse_canonical_phase_2c_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    reconstruct_source_range, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(920), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2c_rule_for_test(source(text), rule, ParseConfig::default())
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2C direct rule"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

#[test]
fn every_literal_rule_accepts_a_closed_contract_example() {
    let cases = [
        (rules::EMPTY, "___"),
        (rules::ATOM, ":atom"),
        (rules::STRING, "\"text\""),
        (rules::UTF8_STRING, "\"text\""),
        (rules::RAW_STRING, "\"\"\"raw\"\"\""),
        (rules::BOOLEAN, "true"),
        (rules::TRUE_LITERAL, "✓"),
        (rules::FALSE_LITERAL, "✗"),
        (rules::NUMBER, "1"),
        (rules::COMPLEX_NUMBER, "2i"),
        (rules::REAL_NUMBER, "-1"),
        (rules::UNTYPED_REAL_NUMBER, "-1"),
        (rules::RATIONAL_LITERAL, "1/2"),
        (rules::SCIENTIFIC_LITERAL, "1.0e3"),
        (rules::FLOAT_DECIMAL_START, ".5"),
        (rules::FLOAT_FULL, "1.0"),
        (rules::FLOAT_LITERAL, ".5"),
        (rules::INTEGER_LITERAL, "1u8"),
        (rules::TYPED_INTEGER, "1u8"),
        (rules::UNTYPED_INTEGER, "1_000"),
        (rules::DECIMAL_LITERAL, "0d12"),
        (rules::HEXADECIMAL_LITERAL, "0xG_"),
        (rules::OCTAL_LITERAL, "0o9"),
        (rules::BINARY_LITERAL, "0b9"),
    ];

    for (rule, input) in cases {
        let parsed = parse(input, rule);
        assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
        assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
        assert_eq!(
            reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
            input,
            "{rule:?} on {input:?}",
        );
    }
}

#[test]
fn ordered_number_selection_retains_the_required_syntax_shapes() {
    for input in ["1e3", "1e+3", "1e-3"] {
        let parsed = parse(input, rules::NUMBER);
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert!(
            find_node(&parsed.syntax(), SyntaxKind::TypedInteger).is_some(),
            "{input:?} must choose typed-integer",
        );
        assert!(
            find_node(&parsed.syntax(), SyntaxKind::ScientificLiteral).is_none(),
            "{input:?} must not choose scientific-literal",
        );
    }

    for input in ["1.0e3", "1.0e+3", "1.0e-3", "1.0e+-3", "1.0e3u8"] {
        let parsed = parse(input, rules::NUMBER);
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert!(
            find_node(&parsed.syntax(), SyntaxKind::ScientificLiteral).is_some(),
            "{input:?} must choose scientific-literal",
        );
    }

    let decimal_start = parse(".5", rules::NUMBER);
    assert!(find_node(&decimal_start.syntax(), SyntaxKind::FloatDecimalStart).is_some());
    let float_full = parse("1.5", rules::NUMBER);
    assert!(find_node(&float_full.syntax(), SyntaxKind::FloatFull).is_some());

    let integer_prefix = parse("1.", rules::NUMBER);
    assert!(integer_prefix.is_strictly_clean());
    assert_eq!(integer_prefix.consumed.end.0, 1);
    assert!(find_node(&integer_prefix.syntax(), SyntaxKind::IntegerLiteral).is_some());

    let rational = parse("1u8/2u16", rules::RATIONAL_LITERAL);
    assert!(rational.is_strictly_clean());
    assert!(find_node(&rational.syntax(), SyntaxKind::RationalLiteral).is_some());
}

#[test]
fn complex_components_preserve_the_required_direct_sign_shape() {
    for input in ["2i", "1+2i", "1-2i", "1+-2i", "1--2i"] {
        let parsed = parse(input, rules::COMPLEX_NUMBER);
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
        assert!(find_node(&parsed.syntax(), SyntaxKind::ComplexNumber).is_some());
    }
}

#[test]
fn token_productions_preserve_prefix_behavior_without_identifier_boundaries() {
    for (rule, input, consumed) in [
        (rules::BOOLEAN, "truex", 4),
        (rules::BOOLEAN, "falsehood", 5),
        (rules::BOOLEAN, "true-value", 4),
    ] {
        let parsed = parse(input, rule);
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert_eq!(parsed.consumed.end.0, consumed, "{input:?}");
    }
}

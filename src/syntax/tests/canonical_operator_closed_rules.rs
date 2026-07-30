use mech_syntax::document::parser::canonical::parse_canonical_phase_2d_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(924), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2d_rule_for_test(source(text), rule, ParseConfig::default())
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2D direct rule"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn contains_token(root: &SyntaxNode, kind: SyntaxKind) -> bool {
    root.tokens().into_iter().any(|token| token.kind() == kind)
}

fn assert_match(rule: RuleId, input: &str, expected_kind: Option<SyntaxKind>) {
    let parsed = parse(input, rule);
    assert_eq!(parsed.rule, rule, "{input:?}");
    assert!(parsed.matched, "{rule:?} did not accept {input:?}");
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(
        parsed.consumed,
        TextRange::new(TextSize::ZERO, parsed.source.byte_len())
    );
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        input,
        "{rule:?} did not reconstruct {input:?}",
    );
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();

    if let Some(kind) = expected_kind {
        assert!(
            find_node(&parsed.syntax(), kind).is_some(),
            "{rule:?} did not emit {kind:?} for {input:?}",
        );
    }
}

fn assert_match_prefix(rule: RuleId, input: &str, consumed: usize, expected_kind: SyntaxKind) {
    let parsed = parse(input, rule);
    assert!(parsed.matched, "{rule:?} did not accept {input:?}");
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(
        parsed.consumed,
        TextRange::new(TextSize::ZERO, TextSize(consumed as u32))
    );
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        &input[..consumed],
        "{rule:?} did not preserve its consumed prefix for {input:?}",
    );
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    assert!(
        find_node(&parsed.syntax(), expected_kind).is_some(),
        "{input:?}"
    );
}

fn assert_no_match(rule: RuleId, input: &str) {
    let parsed = parse(input, rule);
    assert!(!parsed.matched, "{rule:?} unexpectedly accepted {input:?}");
    assert_eq!(
        parsed.consumed,
        TextRange::empty(TextSize::ZERO),
        "{input:?}"
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{rule:?} emitted a diagnostic for {input:?}"
    );
}

#[test]
fn every_closed_operator_rule_accepts_a_direct_contract() {
    let cases = [
        (
            rules::ADD_SUB_OPERATOR,
            "+",
            Some(SyntaxKind::AddSubOperator),
        ),
        (
            rules::MUL_DIV_OPERATOR,
            "*",
            Some(SyntaxKind::MulDivOperator),
        ),
        (rules::POWER_OPERATOR, "^", Some(SyntaxKind::PowerOperator)),
        (
            rules::MATRIX_OPERATOR,
            "**",
            Some(SyntaxKind::MatrixOperator),
        ),
        (
            rules::RANGE_OPERATOR,
            "..=",
            Some(SyntaxKind::RangeOperator),
        ),
        (
            rules::COMPARISON_OPERATOR,
            "===",
            Some(SyntaxKind::ComparisonOperator),
        ),
        (rules::LOGIC_OPERATOR, "&&", Some(SyntaxKind::LogicOperator)),
        (rules::TABLE_OPERATOR, "⋈", Some(SyntaxKind::TableOperator)),
        (rules::SET_OPERATOR, "∪", Some(SyntaxKind::SetOperator)),
        (rules::ADD, "+", Some(SyntaxKind::AddOperation)),
        (rules::SUBTRACT, "-", Some(SyntaxKind::SubtractOperation)),
        (
            rules::RAW_SUBTRACT,
            "-",
            Some(SyntaxKind::RawSubtractOperation),
        ),
        (
            rules::SPACED_SUBTRACT,
            " - ",
            Some(SyntaxKind::SpacedSubtractOperation),
        ),
        (rules::MULTIPLY, "*", Some(SyntaxKind::MultiplyOperation)),
        (rules::DIVIDE, "/", Some(SyntaxKind::DivideOperation)),
        (rules::MODULUS, "%", Some(SyntaxKind::ModulusOperation)),
        (rules::POWER, "^", Some(SyntaxKind::PowerOperation)),
        (
            rules::MATRIX_MULTIPLY,
            "**",
            Some(SyntaxKind::MatrixMultiplyOperation),
        ),
        (
            rules::MATRIX_SOLVE,
            "\\",
            Some(SyntaxKind::MatrixSolveOperation),
        ),
        (
            rules::DOT_PRODUCT,
            "·",
            Some(SyntaxKind::DotProductOperation),
        ),
        (
            rules::CROSS_PRODUCT,
            "⨯",
            Some(SyntaxKind::CrossProductOperation),
        ),
        (rules::TRANSPOSE, "'", None),
        (
            rules::RANGE_INCLUSIVE,
            "..=",
            Some(SyntaxKind::RangeInclusiveOperation),
        ),
        (
            rules::RANGE_EXCLUSIVE,
            "..",
            Some(SyntaxKind::RangeExclusiveOperation),
        ),
        (rules::NOT_EQUAL, "!=", Some(SyntaxKind::NotEqualOperation)),
        (rules::EQUAL_TO, "==", Some(SyntaxKind::EqualToOperation)),
        (
            rules::STRICT_NOT_EQUAL,
            "!==",
            Some(SyntaxKind::StrictNotEqualOperation),
        ),
        (
            rules::STRICT_EQUAL,
            "===",
            Some(SyntaxKind::StrictEqualOperation),
        ),
        (
            rules::GREATER_THAN,
            ">",
            Some(SyntaxKind::GreaterThanOperation),
        ),
        (rules::LESS_THAN, "<", Some(SyntaxKind::LessThanOperation)),
        (
            rules::GREATER_THAN_EQUAL,
            ">=",
            Some(SyntaxKind::GreaterThanEqualOperation),
        ),
        (
            rules::LESS_THAN_EQUAL,
            "<=",
            Some(SyntaxKind::LessThanEqualOperation),
        ),
        (rules::OR, "||", Some(SyntaxKind::OrOperation)),
        (rules::AND, "&&", Some(SyntaxKind::AndOperation)),
        (rules::NOT, "!", Some(SyntaxKind::NotOperation)),
        (rules::XOR, "^^", Some(SyntaxKind::XorOperation)),
        (rules::JOIN, "⋈", Some(SyntaxKind::JoinOperation)),
        (rules::LEFT_JOIN, "⟕", Some(SyntaxKind::LeftJoinOperation)),
        (rules::RIGHT_JOIN, "⟖", Some(SyntaxKind::RightJoinOperation)),
        (rules::FULL_JOIN, "⟗", Some(SyntaxKind::FullJoinOperation)),
        (
            rules::LEFT_SEMI_JOIN,
            "⋉",
            Some(SyntaxKind::LeftSemiJoinOperation),
        ),
        (
            rules::LEFT_ANTI_JOIN,
            "▷",
            Some(SyntaxKind::LeftAntiJoinOperation),
        ),
        (rules::UNION_OP, "∪", Some(SyntaxKind::UnionOperation)),
        (
            rules::INTERSECTION,
            "∩",
            Some(SyntaxKind::IntersectionOperation),
        ),
        (
            rules::DIFFERENCE,
            "∖",
            Some(SyntaxKind::DifferenceOperation),
        ),
        (
            rules::COMPLEMENT,
            "∁",
            Some(SyntaxKind::ComplementOperation),
        ),
        (rules::SUBSET, "⊆", Some(SyntaxKind::SubsetOperation)),
        (rules::SUPERSET, "⊇", Some(SyntaxKind::SupersetOperation)),
        (
            rules::PROPER_SUBSET,
            "⊊",
            Some(SyntaxKind::ProperSubsetOperation),
        ),
        (
            rules::PROPER_SUPERSET,
            "⊋",
            Some(SyntaxKind::ProperSupersetOperation),
        ),
        (rules::ELEMENT_OF, "∈", Some(SyntaxKind::ElementOfOperation)),
        (
            rules::NOT_ELEMENT_OF,
            "∉",
            Some(SyntaxKind::NotElementOfOperation),
        ),
        (
            rules::SYMMETRIC_DIFFERENCE,
            " Δ ",
            Some(SyntaxKind::SymmetricDifferenceOperation),
        ),
    ];

    assert_eq!(cases.len(), 53);
    for (rule, input, kind) in cases {
        assert_match(rule, input, kind);
    }

    let transpose = parse("'", rules::TRANSPOSE);
    assert!(contains_token(&transpose.syntax(), SyntaxKind::Apostrophe));
}

#[test]
fn every_accepted_operator_spelling_has_its_direct_leaf_shape() {
    let cases = [
        (rules::ADD, "+", SyntaxKind::AddOperation),
        (rules::RAW_SUBTRACT, "-", SyntaxKind::RawSubtractOperation),
        (rules::MULTIPLY, "*", SyntaxKind::MultiplyOperation),
        (rules::MULTIPLY, "×", SyntaxKind::MultiplyOperation),
        (rules::DIVIDE, "/", SyntaxKind::DivideOperation),
        (rules::DIVIDE, "÷", SyntaxKind::DivideOperation),
        (rules::MODULUS, "%", SyntaxKind::ModulusOperation),
        (rules::POWER, "^", SyntaxKind::PowerOperation),
        (
            rules::MATRIX_MULTIPLY,
            "**",
            SyntaxKind::MatrixMultiplyOperation,
        ),
        (rules::MATRIX_SOLVE, "\\", SyntaxKind::MatrixSolveOperation),
        (rules::DOT_PRODUCT, "·", SyntaxKind::DotProductOperation),
        (rules::DOT_PRODUCT, "•", SyntaxKind::DotProductOperation),
        (rules::CROSS_PRODUCT, "⨯", SyntaxKind::CrossProductOperation),
        (
            rules::RANGE_INCLUSIVE,
            "..=",
            SyntaxKind::RangeInclusiveOperation,
        ),
        (
            rules::RANGE_EXCLUSIVE,
            "..",
            SyntaxKind::RangeExclusiveOperation,
        ),
        (rules::NOT_EQUAL, "!=", SyntaxKind::NotEqualOperation),
        (rules::NOT_EQUAL, "¬=", SyntaxKind::NotEqualOperation),
        (rules::NOT_EQUAL, "≠", SyntaxKind::NotEqualOperation),
        (rules::EQUAL_TO, "==", SyntaxKind::EqualToOperation),
        (rules::EQUAL_TO, "⩵", SyntaxKind::EqualToOperation),
        (
            rules::STRICT_NOT_EQUAL,
            "!==",
            SyntaxKind::StrictNotEqualOperation,
        ),
        (
            rules::STRICT_NOT_EQUAL,
            "!≡",
            SyntaxKind::StrictNotEqualOperation,
        ),
        (
            rules::STRICT_NOT_EQUAL,
            "¬≡",
            SyntaxKind::StrictNotEqualOperation,
        ),
        (
            rules::STRICT_NOT_EQUAL,
            "¬==",
            SyntaxKind::StrictNotEqualOperation,
        ),
        (rules::STRICT_EQUAL, "===", SyntaxKind::StrictEqualOperation),
        (rules::STRICT_EQUAL, "≡", SyntaxKind::StrictEqualOperation),
        (rules::GREATER_THAN, ">", SyntaxKind::GreaterThanOperation),
        (rules::LESS_THAN, "<", SyntaxKind::LessThanOperation),
        (
            rules::GREATER_THAN_EQUAL,
            ">=",
            SyntaxKind::GreaterThanEqualOperation,
        ),
        (
            rules::GREATER_THAN_EQUAL,
            "≥",
            SyntaxKind::GreaterThanEqualOperation,
        ),
        (
            rules::LESS_THAN_EQUAL,
            "<=",
            SyntaxKind::LessThanEqualOperation,
        ),
        (
            rules::LESS_THAN_EQUAL,
            "≤",
            SyntaxKind::LessThanEqualOperation,
        ),
        (rules::OR, "||", SyntaxKind::OrOperation),
        (rules::OR, "∨", SyntaxKind::OrOperation),
        (rules::OR, "⋁", SyntaxKind::OrOperation),
        (rules::AND, "&&", SyntaxKind::AndOperation),
        (rules::AND, "∧", SyntaxKind::AndOperation),
        (rules::AND, "⋀", SyntaxKind::AndOperation),
        (rules::NOT, "!", SyntaxKind::NotOperation),
        (rules::NOT, "¬", SyntaxKind::NotOperation),
        (rules::XOR, "^^", SyntaxKind::XorOperation),
        (rules::XOR, "⊕", SyntaxKind::XorOperation),
        (rules::XOR, "⊻", SyntaxKind::XorOperation),
        (rules::JOIN, "⋈", SyntaxKind::JoinOperation),
        (rules::LEFT_JOIN, "⟕", SyntaxKind::LeftJoinOperation),
        (rules::RIGHT_JOIN, "⟖", SyntaxKind::RightJoinOperation),
        (rules::FULL_JOIN, "⟗", SyntaxKind::FullJoinOperation),
        (
            rules::LEFT_SEMI_JOIN,
            "⋉",
            SyntaxKind::LeftSemiJoinOperation,
        ),
        (
            rules::LEFT_ANTI_JOIN,
            "▷",
            SyntaxKind::LeftAntiJoinOperation,
        ),
        (rules::UNION_OP, "∪", SyntaxKind::UnionOperation),
        (rules::INTERSECTION, "∩", SyntaxKind::IntersectionOperation),
        (rules::DIFFERENCE, "∖", SyntaxKind::DifferenceOperation),
        (rules::COMPLEMENT, "∁", SyntaxKind::ComplementOperation),
        (rules::SUBSET, "⊆", SyntaxKind::SubsetOperation),
        (rules::SUPERSET, "⊇", SyntaxKind::SupersetOperation),
        (rules::PROPER_SUBSET, "⊊", SyntaxKind::ProperSubsetOperation),
        (rules::PROPER_SUBSET, "⊂", SyntaxKind::ProperSubsetOperation),
        (
            rules::PROPER_SUPERSET,
            "⊋",
            SyntaxKind::ProperSupersetOperation,
        ),
        (
            rules::PROPER_SUPERSET,
            "⊃",
            SyntaxKind::ProperSupersetOperation,
        ),
        (rules::ELEMENT_OF, "∈", SyntaxKind::ElementOfOperation),
        (
            rules::NOT_ELEMENT_OF,
            "∉",
            SyntaxKind::NotElementOfOperation,
        ),
        (
            rules::SYMMETRIC_DIFFERENCE,
            " Δ ",
            SyntaxKind::SymmetricDifferenceOperation,
        ),
    ];

    assert_eq!(cases.len() + 1, 63, "including transparent transpose");
    for (rule, input, kind) in cases {
        assert_match(rule, input, Some(kind));
    }

    let transpose = parse("'", rules::TRANSPOSE);
    assert!(transpose.matched);
    assert!(contains_token(&transpose.syntax(), SyntaxKind::Apostrophe));
    assert_eq!(
        reconstruct_source_range(&transpose.root, &transpose.source, transpose.consumed).unwrap(),
        "'"
    );
}

#[test]
fn overlapping_operator_forms_keep_leaf_prefix_and_aggregate_selection_behavior() {
    assert_match(rules::MULTIPLY, "*", Some(SyntaxKind::MultiplyOperation));
    assert_match(rules::MULTIPLY, "×", Some(SyntaxKind::MultiplyOperation));
    assert_no_match(rules::MULTIPLY, "**");
    assert_match(
        rules::MATRIX_MULTIPLY,
        "**",
        Some(SyntaxKind::MatrixMultiplyOperation),
    );
    assert_match(
        rules::MATRIX_OPERATOR,
        "**",
        Some(SyntaxKind::MatrixMultiplyOperation),
    );

    assert_match(
        rules::RAW_SUBTRACT,
        "-",
        Some(SyntaxKind::RawSubtractOperation),
    );
    assert_no_match(rules::RAW_SUBTRACT, "--");
    assert_no_match(rules::SUBTRACT, "-- comment");
    assert_match(
        rules::SUBTRACT,
        " - ",
        Some(SyntaxKind::SpacedSubtractOperation),
    );
    assert_match(rules::SUBTRACT, "-", Some(SyntaxKind::RawSubtractOperation));

    assert_match(rules::DIVIDE, "/", Some(SyntaxKind::DivideOperation));
    assert_match(rules::DIVIDE, "÷", Some(SyntaxKind::DivideOperation));
    assert_no_match(rules::DIVIDE, "//");
    assert_no_match(rules::DIVIDE, "// comment");

    assert_match_prefix(
        rules::RANGE_INCLUSIVE,
        "..=",
        3,
        SyntaxKind::RangeInclusiveOperation,
    );
    assert_match_prefix(
        rules::RANGE_EXCLUSIVE,
        "..=",
        2,
        SyntaxKind::RangeExclusiveOperation,
    );
    assert_match(
        rules::RANGE_OPERATOR,
        "..=",
        Some(SyntaxKind::RangeInclusiveOperation),
    );
    assert_match(
        rules::RANGE_OPERATOR,
        "..",
        Some(SyntaxKind::RangeExclusiveOperation),
    );

    assert_match_prefix(rules::EQUAL_TO, "===", 2, SyntaxKind::EqualToOperation);
    assert_match(
        rules::COMPARISON_OPERATOR,
        "===",
        Some(SyntaxKind::StrictEqualOperation),
    );
    assert_match_prefix(rules::NOT_EQUAL, "!==", 2, SyntaxKind::NotEqualOperation);
    assert_match(
        rules::COMPARISON_OPERATOR,
        "!==",
        Some(SyntaxKind::StrictNotEqualOperation),
    );
    assert_match_prefix(
        rules::GREATER_THAN,
        ">=",
        1,
        SyntaxKind::GreaterThanOperation,
    );
    assert_match(
        rules::COMPARISON_OPERATOR,
        ">=",
        Some(SyntaxKind::GreaterThanEqualOperation),
    );
    assert_match_prefix(rules::LESS_THAN, "<=", 1, SyntaxKind::LessThanOperation);
    assert_match(
        rules::COMPARISON_OPERATOR,
        "<=",
        Some(SyntaxKind::LessThanEqualOperation),
    );
    assert_no_match(rules::LESS_THAN, "<-");
    assert_no_match(rules::COMPARISON_OPERATOR, "<-");
}

#[test]
fn horizontal_space_and_required_space_follow_the_canonical_rules() {
    for input in ["+", " + ", "\t+\t", "\u{00A0}+\u{2009}"] {
        assert_match(rules::ADD, input, Some(SyntaxKind::AddOperation));
    }
    assert_match_prefix(rules::ADD, " +\n", 2, SyntaxKind::AddOperation);

    assert_no_match(rules::SYMMETRIC_DIFFERENCE, "Δ");
    assert_match(
        rules::SYMMETRIC_DIFFERENCE,
        " Δ ",
        Some(SyntaxKind::SymmetricDifferenceOperation),
    );
    assert_match(
        rules::SYMMETRIC_DIFFERENCE,
        "\tΔ\u{2009}",
        Some(SyntaxKind::SymmetricDifferenceOperation),
    );
    assert_no_match(rules::SYMMETRIC_DIFFERENCE, "\nΔ\n");
}

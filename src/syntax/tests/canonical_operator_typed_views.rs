use mech_syntax::document::ast::{
    AddSubOperatorSyntax, CanonicalOperator, ComparisonOperatorSyntax, LogicOperatorSyntax,
    MatrixOperatorSyntax, MulDivOperatorSyntax, OperatorSyntax, PowerOperatorSyntax,
    RangeOperatorSyntax, SetOperatorSyntax, SubtractOperationSyntax, TableOperatorSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2d_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange,
    TextSize, TextSnapshot,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(925), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn parse_typed<N: AstNode>(input: &str, rule: RuleId, kind: SyntaxKind) -> N {
    let parsed =
        parse_canonical_phase_2d_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2D direct rule"));
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    let node = find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"));
    N::cast(node).unwrap_or_else(|| panic!("{kind:?} did not cast for {input:?}"))
}

#[test]
fn aggregate_views_expose_the_selected_operator_meaning() {
    let add = parse_typed::<AddSubOperatorSyntax>(
        "+",
        rules::ADD_SUB_OPERATOR,
        SyntaxKind::AddSubOperator,
    );
    let selected = add.selected().expect("add selector must have a child");
    assert_eq!(selected.semantic(), Some(CanonicalOperator::Add));
    assert_eq!(selected.syntax().kind(), SyntaxKind::AddOperation);

    let subtract = parse_typed::<AddSubOperatorSyntax>(
        " - ",
        rules::ADD_SUB_OPERATOR,
        SyntaxKind::AddSubOperator,
    );
    let selected = subtract
        .selected()
        .expect("subtract selector must have a child");
    assert_eq!(selected.semantic(), Some(CanonicalOperator::Subtract));
    assert_eq!(selected.syntax().kind(), SyntaxKind::SubtractOperation);
    let spelling = find_node(selected.syntax(), SyntaxKind::SpacedSubtractOperation)
        .expect("spaced subtraction must retain its spelling node");
    assert_eq!(spelling.kind(), SyntaxKind::SpacedSubtractOperation);

    let mul_div = parse_typed::<MulDivOperatorSyntax>(
        "*",
        rules::MUL_DIV_OPERATOR,
        SyntaxKind::MulDivOperator,
    );
    assert_eq!(
        mul_div.selected().and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::Multiply),
    );

    let power =
        parse_typed::<PowerOperatorSyntax>("^", rules::POWER_OPERATOR, SyntaxKind::PowerOperator);
    assert_eq!(
        power.selected().and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::Power),
    );

    let matrix = parse_typed::<MatrixOperatorSyntax>(
        "**",
        rules::MATRIX_OPERATOR,
        SyntaxKind::MatrixOperator,
    );
    assert_eq!(
        matrix.selected().and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::MatrixMultiply),
    );

    let comparison = parse_typed::<ComparisonOperatorSyntax>(
        "!==",
        rules::COMPARISON_OPERATOR,
        SyntaxKind::ComparisonOperator,
    );
    assert_eq!(
        comparison
            .selected()
            .and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::StrictNotEqual),
    );

    let range =
        parse_typed::<RangeOperatorSyntax>("..=", rules::RANGE_OPERATOR, SyntaxKind::RangeOperator);
    assert_eq!(
        range.selected().and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::RangeInclusive),
    );

    let logic =
        parse_typed::<LogicOperatorSyntax>("∧", rules::LOGIC_OPERATOR, SyntaxKind::LogicOperator);
    assert_eq!(
        logic.selected().and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::And),
    );

    let table =
        parse_typed::<TableOperatorSyntax>("⟗", rules::TABLE_OPERATOR, SyntaxKind::TableOperator);
    assert_eq!(
        table.selected().and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::FullOuterJoin),
    );

    let set = parse_typed::<SetOperatorSyntax>("⊂", rules::SET_OPERATOR, SyntaxKind::SetOperator);
    assert_eq!(
        set.selected().and_then(|operator| operator.semantic()),
        Some(CanonicalOperator::ProperSubset),
    );
}

#[test]
fn subtraction_views_distinguish_raw_and_spaced_spellings() {
    let raw =
        parse_typed::<SubtractOperationSyntax>("-", rules::SUBTRACT, SyntaxKind::SubtractOperation);
    assert!(raw.raw().is_some());
    assert!(raw.spaced().is_none());

    let spaced = parse_typed::<SubtractOperationSyntax>(
        " - ",
        rules::SUBTRACT,
        SyntaxKind::SubtractOperation,
    );
    assert!(spaced.raw().is_none());
    let spelling = spaced.spaced().expect("spaced subtraction spelling");
    assert!(spelling.raw().is_some());
}

#[test]
fn generic_operator_view_maps_every_semantic_leaf_kind() {
    let cases = [
        (
            rules::ADD,
            "+",
            SyntaxKind::AddOperation,
            CanonicalOperator::Add,
        ),
        (
            rules::SUBTRACT,
            "-",
            SyntaxKind::SubtractOperation,
            CanonicalOperator::Subtract,
        ),
        (
            rules::RAW_SUBTRACT,
            "-",
            SyntaxKind::RawSubtractOperation,
            CanonicalOperator::Subtract,
        ),
        (
            rules::SPACED_SUBTRACT,
            " - ",
            SyntaxKind::SpacedSubtractOperation,
            CanonicalOperator::Subtract,
        ),
        (
            rules::MULTIPLY,
            "*",
            SyntaxKind::MultiplyOperation,
            CanonicalOperator::Multiply,
        ),
        (
            rules::DIVIDE,
            "/",
            SyntaxKind::DivideOperation,
            CanonicalOperator::Divide,
        ),
        (
            rules::MODULUS,
            "%",
            SyntaxKind::ModulusOperation,
            CanonicalOperator::Modulus,
        ),
        (
            rules::POWER,
            "^",
            SyntaxKind::PowerOperation,
            CanonicalOperator::Power,
        ),
        (
            rules::MATRIX_MULTIPLY,
            "**",
            SyntaxKind::MatrixMultiplyOperation,
            CanonicalOperator::MatrixMultiply,
        ),
        (
            rules::MATRIX_SOLVE,
            "\\",
            SyntaxKind::MatrixSolveOperation,
            CanonicalOperator::MatrixSolve,
        ),
        (
            rules::DOT_PRODUCT,
            "·",
            SyntaxKind::DotProductOperation,
            CanonicalOperator::DotProduct,
        ),
        (
            rules::CROSS_PRODUCT,
            "⨯",
            SyntaxKind::CrossProductOperation,
            CanonicalOperator::CrossProduct,
        ),
        (
            rules::RANGE_INCLUSIVE,
            "..=",
            SyntaxKind::RangeInclusiveOperation,
            CanonicalOperator::RangeInclusive,
        ),
        (
            rules::RANGE_EXCLUSIVE,
            "..",
            SyntaxKind::RangeExclusiveOperation,
            CanonicalOperator::RangeExclusive,
        ),
        (
            rules::NOT_EQUAL,
            "!=",
            SyntaxKind::NotEqualOperation,
            CanonicalOperator::NotEqual,
        ),
        (
            rules::EQUAL_TO,
            "==",
            SyntaxKind::EqualToOperation,
            CanonicalOperator::EqualTo,
        ),
        (
            rules::STRICT_NOT_EQUAL,
            "!==",
            SyntaxKind::StrictNotEqualOperation,
            CanonicalOperator::StrictNotEqual,
        ),
        (
            rules::STRICT_EQUAL,
            "===",
            SyntaxKind::StrictEqualOperation,
            CanonicalOperator::StrictEqual,
        ),
        (
            rules::GREATER_THAN,
            ">",
            SyntaxKind::GreaterThanOperation,
            CanonicalOperator::GreaterThan,
        ),
        (
            rules::LESS_THAN,
            "<",
            SyntaxKind::LessThanOperation,
            CanonicalOperator::LessThan,
        ),
        (
            rules::GREATER_THAN_EQUAL,
            ">=",
            SyntaxKind::GreaterThanEqualOperation,
            CanonicalOperator::GreaterThanEqual,
        ),
        (
            rules::LESS_THAN_EQUAL,
            "<=",
            SyntaxKind::LessThanEqualOperation,
            CanonicalOperator::LessThanEqual,
        ),
        (
            rules::OR,
            "||",
            SyntaxKind::OrOperation,
            CanonicalOperator::Or,
        ),
        (
            rules::AND,
            "&&",
            SyntaxKind::AndOperation,
            CanonicalOperator::And,
        ),
        (
            rules::NOT,
            "!",
            SyntaxKind::NotOperation,
            CanonicalOperator::Not,
        ),
        (
            rules::XOR,
            "^^",
            SyntaxKind::XorOperation,
            CanonicalOperator::Xor,
        ),
        (
            rules::JOIN,
            "⋈",
            SyntaxKind::JoinOperation,
            CanonicalOperator::InnerJoin,
        ),
        (
            rules::LEFT_JOIN,
            "⟕",
            SyntaxKind::LeftJoinOperation,
            CanonicalOperator::LeftOuterJoin,
        ),
        (
            rules::RIGHT_JOIN,
            "⟖",
            SyntaxKind::RightJoinOperation,
            CanonicalOperator::RightOuterJoin,
        ),
        (
            rules::FULL_JOIN,
            "⟗",
            SyntaxKind::FullJoinOperation,
            CanonicalOperator::FullOuterJoin,
        ),
        (
            rules::LEFT_SEMI_JOIN,
            "⋉",
            SyntaxKind::LeftSemiJoinOperation,
            CanonicalOperator::LeftSemiJoin,
        ),
        (
            rules::LEFT_ANTI_JOIN,
            "▷",
            SyntaxKind::LeftAntiJoinOperation,
            CanonicalOperator::LeftAntiJoin,
        ),
        (
            rules::UNION_OP,
            "∪",
            SyntaxKind::UnionOperation,
            CanonicalOperator::Union,
        ),
        (
            rules::INTERSECTION,
            "∩",
            SyntaxKind::IntersectionOperation,
            CanonicalOperator::Intersection,
        ),
        (
            rules::DIFFERENCE,
            "∖",
            SyntaxKind::DifferenceOperation,
            CanonicalOperator::Difference,
        ),
        (
            rules::COMPLEMENT,
            "∁",
            SyntaxKind::ComplementOperation,
            CanonicalOperator::Complement,
        ),
        (
            rules::SUBSET,
            "⊆",
            SyntaxKind::SubsetOperation,
            CanonicalOperator::Subset,
        ),
        (
            rules::SUPERSET,
            "⊇",
            SyntaxKind::SupersetOperation,
            CanonicalOperator::Superset,
        ),
        (
            rules::PROPER_SUBSET,
            "⊂",
            SyntaxKind::ProperSubsetOperation,
            CanonicalOperator::ProperSubset,
        ),
        (
            rules::PROPER_SUPERSET,
            "⊃",
            SyntaxKind::ProperSupersetOperation,
            CanonicalOperator::ProperSuperset,
        ),
        (
            rules::ELEMENT_OF,
            "∈",
            SyntaxKind::ElementOfOperation,
            CanonicalOperator::ElementOf,
        ),
        (
            rules::NOT_ELEMENT_OF,
            "∉",
            SyntaxKind::NotElementOfOperation,
            CanonicalOperator::NotElementOf,
        ),
        (
            rules::SYMMETRIC_DIFFERENCE,
            " Δ ",
            SyntaxKind::SymmetricDifferenceOperation,
            CanonicalOperator::SymmetricDifference,
        ),
    ];

    assert_eq!(cases.len(), 43);
    for (rule, input, kind, expected) in cases {
        let operator = parse_typed::<OperatorSyntax>(input, rule, kind);
        assert_eq!(operator.semantic(), Some(expected), "{rule:?} on {input:?}");
    }
}

#[test]
fn operator_token_ranges_exclude_surrounding_canonical_whitespace() {
    let operator = parse_typed::<OperatorSyntax>(
        "  >=\t",
        rules::GREATER_THAN_EQUAL,
        SyntaxKind::GreaterThanEqualOperation,
    );
    let range = operator
        .operator_token_range()
        .expect("operator token range");
    assert_eq!(range, TextRange::new(TextSize(2), TextSize(4)));
    assert_eq!(source("  >=\t").text(range).unwrap(), ">=");

    let operator =
        parse_typed::<OperatorSyntax>("\u{00A0}+\u{2009}", rules::ADD, SyntaxKind::AddOperation);
    let range = operator
        .operator_token_range()
        .expect("operator token range");
    assert_eq!(range, TextRange::new(TextSize(2), TextSize(3)));
    assert_eq!(source("\u{00A0}+\u{2009}").text(range).unwrap(), "+");
}

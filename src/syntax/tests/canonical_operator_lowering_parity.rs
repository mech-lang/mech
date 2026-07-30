use mech_core::nodes::{
    AddSubOp, ComparisonOp, FormulaOperator, LogicOp, MulDivOp, PowerOp, RangeOp, SetOp, TableOp,
    VecOp,
};
use mech_syntax::document::ast::{
    AddSubOperatorSyntax, ComparisonOperatorSyntax, LogicOperatorSyntax, MatrixOperatorSyntax,
    MulDivOperatorSyntax, PowerOperatorSyntax, RangeOperatorSyntax, SetOperatorSyntax,
    TableOperatorSyntax,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2d_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextSnapshot,
    lower_legacy_add_sub_operator, lower_legacy_comparison_operator, lower_legacy_logic_operator,
    lower_legacy_matrix_operator, lower_legacy_mul_div_operator, lower_legacy_power_operator,
    lower_legacy_range_operator, lower_legacy_set_operator, lower_legacy_table_operator,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(924), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn canonical<N: AstNode>(input: &str, rule: RuleId, kind: SyntaxKind) -> N {
    let parsed =
        parse_canonical_phase_2d_rule_for_test(source(input), rule, ParseConfig::default())
            .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2D direct rule"));
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.consumed.end, parsed.source.byte_len(), "{input:?}");
    let node = find_node(&parsed.syntax(), kind)
        .unwrap_or_else(|| panic!("{rule:?} did not emit {kind:?} for {input:?}"));
    N::cast(node).unwrap_or_else(|| panic!("{kind:?} did not cast for {input:?}"))
}

fn legacy<Output>(
    input: &str,
    parser: for<'source> fn(
        mech_syntax::ParseString<'source>,
    ) -> mech_syntax::ParseResult<'source, Output>,
) -> Output {
    let graphemes = mech_syntax::graphemes::init_tag(input);
    let (remaining, value) = parser(mech_syntax::ParseString::new(&graphemes))
        .unwrap_or_else(|error| panic!("legacy parser rejected {input:?}: {error:?}"));
    assert_eq!(remaining.cursor, graphemes.len(), "{input:?}");
    assert!(remaining.error_log.is_empty(), "{input:?}");
    value
}

#[test]
fn public_aggregate_lowerers_match_legacy_operator_values() {
    for input in ["+", "-", " - "] {
        let syntax = canonical::<AddSubOperatorSyntax>(
            input,
            rules::ADD_SUB_OPERATOR,
            SyntaxKind::AddSubOperator,
        );
        assert_eq!(
            lower_legacy_add_sub_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::add_sub_operator),
            "{input:?}"
        );
    }

    for input in ["*", "×", "/", "÷", "%"] {
        let syntax = canonical::<MulDivOperatorSyntax>(
            input,
            rules::MUL_DIV_OPERATOR,
            SyntaxKind::MulDivOperator,
        );
        assert_eq!(
            lower_legacy_mul_div_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::mul_div_operator),
            "{input:?}"
        );
    }

    for input in ["^"] {
        let syntax = canonical::<PowerOperatorSyntax>(
            input,
            rules::POWER_OPERATOR,
            SyntaxKind::PowerOperator,
        );
        assert_eq!(
            lower_legacy_power_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::power_operator),
            "{input:?}"
        );
    }

    for input in ["**", "\\", "·", "•", "⨯"] {
        let syntax = canonical::<MatrixOperatorSyntax>(
            input,
            rules::MATRIX_OPERATOR,
            SyntaxKind::MatrixOperator,
        );
        assert_eq!(
            lower_legacy_matrix_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::matrix_operator),
            "{input:?}"
        );
    }

    for input in ["..=", ".."] {
        let syntax = canonical::<RangeOperatorSyntax>(
            input,
            rules::RANGE_OPERATOR,
            SyntaxKind::RangeOperator,
        );
        assert_eq!(
            lower_legacy_range_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::range_operator),
            "{input:?}"
        );
    }

    for input in [
        "===", "≡", "!==", "!≡", "¬≡", "¬==", "!=", "¬=", "≠", "==", "⩵", ">=", "≥", ">", "<=",
        "≤", "<",
    ] {
        let syntax = canonical::<ComparisonOperatorSyntax>(
            input,
            rules::COMPARISON_OPERATOR,
            SyntaxKind::ComparisonOperator,
        );
        assert_eq!(
            lower_legacy_comparison_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::comparison_operator),
            "{input:?}"
        );
    }

    for input in ["&&", "∧", "⋀", "||", "∨", "⋁", "^^", "⊕", "⊻"] {
        let syntax = canonical::<LogicOperatorSyntax>(
            input,
            rules::LOGIC_OPERATOR,
            SyntaxKind::LogicOperator,
        );
        assert_eq!(
            lower_legacy_logic_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::logic_operator),
            "{input:?}"
        );
    }

    for (input, value) in [
        ("⋈", TableOp::InnerJoin),
        ("⟕", TableOp::LeftOuterJoin),
        ("⟖", TableOp::RightOuterJoin),
        ("⟗", TableOp::FullOuterJoin),
        ("⋉", TableOp::LeftSemiJoin),
        ("▷", TableOp::LeftAntiJoin),
    ] {
        let syntax = canonical::<TableOperatorSyntax>(
            input,
            rules::TABLE_OPERATOR,
            SyntaxKind::TableOperator,
        );
        assert_eq!(
            lower_legacy_table_operator(&syntax).unwrap(),
            FormulaOperator::Table(value),
            "{input:?}"
        );
    }

    for input in [
        "∪", "∩", "∖", "∁", "⊆", "⊇", "⊊", "⊂", "⊋", "⊃", "∈", "∉", " Δ ",
    ] {
        let syntax =
            canonical::<SetOperatorSyntax>(input, rules::SET_OPERATOR, SyntaxKind::SetOperator);
        assert_eq!(
            lower_legacy_set_operator(&syntax).unwrap(),
            legacy(input, mech_syntax::set_operator),
            "{input:?}"
        );
    }
}

#[test]
fn aggregate_lowerers_return_the_declared_compatibility_wrappers() {
    assert_eq!(
        lower_legacy_add_sub_operator(&canonical::<AddSubOperatorSyntax>(
            "+",
            rules::ADD_SUB_OPERATOR,
            SyntaxKind::AddSubOperator,
        ))
        .unwrap(),
        FormulaOperator::AddSub(AddSubOp::Add),
    );
    assert_eq!(
        lower_legacy_mul_div_operator(&canonical::<MulDivOperatorSyntax>(
            "%",
            rules::MUL_DIV_OPERATOR,
            SyntaxKind::MulDivOperator,
        ))
        .unwrap(),
        FormulaOperator::MulDiv(MulDivOp::Mod),
    );
    assert_eq!(
        lower_legacy_power_operator(&canonical::<PowerOperatorSyntax>(
            "^",
            rules::POWER_OPERATOR,
            SyntaxKind::PowerOperator,
        ))
        .unwrap(),
        FormulaOperator::Power(PowerOp::Pow),
    );
    assert_eq!(
        lower_legacy_matrix_operator(&canonical::<MatrixOperatorSyntax>(
            "⨯",
            rules::MATRIX_OPERATOR,
            SyntaxKind::MatrixOperator,
        ))
        .unwrap(),
        FormulaOperator::Vec(VecOp::Cross),
    );
    assert_eq!(
        lower_legacy_range_operator(&canonical::<RangeOperatorSyntax>(
            "..=",
            rules::RANGE_OPERATOR,
            SyntaxKind::RangeOperator,
        ))
        .unwrap(),
        RangeOp::Inclusive,
    );
    assert_eq!(
        lower_legacy_comparison_operator(&canonical::<ComparisonOperatorSyntax>(
            "!==",
            rules::COMPARISON_OPERATOR,
            SyntaxKind::ComparisonOperator,
        ))
        .unwrap(),
        FormulaOperator::Comparison(ComparisonOp::StrictNotEqual),
    );
    assert_eq!(
        lower_legacy_logic_operator(&canonical::<LogicOperatorSyntax>(
            "∧",
            rules::LOGIC_OPERATOR,
            SyntaxKind::LogicOperator,
        ))
        .unwrap(),
        FormulaOperator::Logic(LogicOp::And),
    );
    assert_eq!(
        lower_legacy_table_operator(&canonical::<TableOperatorSyntax>(
            "⟗",
            rules::TABLE_OPERATOR,
            SyntaxKind::TableOperator,
        ))
        .unwrap(),
        FormulaOperator::Table(TableOp::FullOuterJoin),
    );
    assert_eq!(
        lower_legacy_set_operator(&canonical::<SetOperatorSyntax>(
            "⊂",
            rules::SET_OPERATOR,
            SyntaxKind::SetOperator,
        ))
        .unwrap(),
        FormulaOperator::Set(SetOp::ProperSubset),
    );
}

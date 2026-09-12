//! Canonical closed expression-operator productions for Phase 2D.
//!
//! This module deliberately stops at the operator layer.  The recursive
//! expression parents select these productions in a later closed phase.

use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::combinator::{self, Attempt};
use super::{base, statements};

/// The complete closed operator set directly ported by Phase 2D.
pub(crate) const PHASE_2D_OPERATOR_RULES: &[RuleId; 53] = &[
    rules::ADD_SUB_OPERATOR,
    rules::MUL_DIV_OPERATOR,
    rules::POWER_OPERATOR,
    rules::MATRIX_OPERATOR,
    rules::RANGE_OPERATOR,
    rules::COMPARISON_OPERATOR,
    rules::LOGIC_OPERATOR,
    rules::TABLE_OPERATOR,
    rules::SET_OPERATOR,
    rules::ADD,
    rules::SUBTRACT,
    rules::RAW_SUBTRACT,
    rules::SPACED_SUBTRACT,
    rules::MULTIPLY,
    rules::DIVIDE,
    rules::MODULUS,
    rules::POWER,
    rules::MATRIX_MULTIPLY,
    rules::MATRIX_SOLVE,
    rules::DOT_PRODUCT,
    rules::CROSS_PRODUCT,
    rules::TRANSPOSE,
    rules::RANGE_INCLUSIVE,
    rules::RANGE_EXCLUSIVE,
    rules::NOT_EQUAL,
    rules::EQUAL_TO,
    rules::STRICT_NOT_EQUAL,
    rules::STRICT_EQUAL,
    rules::GREATER_THAN,
    rules::LESS_THAN,
    rules::GREATER_THAN_EQUAL,
    rules::LESS_THAN_EQUAL,
    rules::OR,
    rules::AND,
    rules::NOT,
    rules::XOR,
    rules::JOIN,
    rules::LEFT_JOIN,
    rules::RIGHT_JOIN,
    rules::FULL_JOIN,
    rules::LEFT_SEMI_JOIN,
    rules::LEFT_ANTI_JOIN,
    rules::UNION_OP,
    rules::INTERSECTION,
    rules::DIFFERENCE,
    rules::COMPLEMENT,
    rules::SUBSET,
    rules::SUPERSET,
    rules::PROPER_SUBSET,
    rules::PROPER_SUPERSET,
    rules::ELEMENT_OF,
    rules::NOT_ELEMENT_OF,
    rules::SYMMETRIC_DIFFERENCE,
];

#[derive(Clone, Copy)]
enum OperatorAtom {
    CanonicalRule(RuleId),
    Text(&'static str),
}

#[derive(Clone, Copy)]
enum OperatorGuard {
    None,
    NotCommentSigil,
    NotMatrixMultiply,
    NotGeneratorArrow,
}

/// Whether `rule` belongs to the Phase 2D closed operator layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2D_OPERATOR_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2D operator production.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::ADD_SUB_OPERATOR => parse_add_sub_operator(parser),
        rules::MUL_DIV_OPERATOR => parse_mul_div_operator(parser),
        rules::POWER_OPERATOR => parse_power_operator(parser),
        rules::MATRIX_OPERATOR => parse_matrix_operator(parser),
        rules::RANGE_OPERATOR => parse_range_operator(parser),
        rules::COMPARISON_OPERATOR => parse_comparison_operator(parser),
        rules::LOGIC_OPERATOR => parse_logic_operator(parser),
        rules::TABLE_OPERATOR => parse_table_operator(parser),
        rules::SET_OPERATOR => parse_set_operator(parser),
        rules::ADD => parse_add(parser),
        rules::SUBTRACT => parse_subtract(parser),
        rules::RAW_SUBTRACT => parse_raw_subtract(parser),
        rules::SPACED_SUBTRACT => parse_spaced_subtract(parser),
        rules::MULTIPLY => parse_multiply(parser),
        rules::DIVIDE => parse_divide(parser),
        rules::MODULUS => parse_modulus(parser),
        rules::POWER => parse_power(parser),
        rules::MATRIX_MULTIPLY => parse_matrix_multiply(parser),
        rules::MATRIX_SOLVE => parse_matrix_solve(parser),
        rules::DOT_PRODUCT => parse_dot_product(parser),
        rules::CROSS_PRODUCT => parse_cross_product(parser),
        rules::TRANSPOSE => parse_transpose(parser),
        rules::RANGE_INCLUSIVE => parse_range_inclusive(parser),
        rules::RANGE_EXCLUSIVE => parse_range_exclusive(parser),
        rules::NOT_EQUAL => parse_not_equal(parser),
        rules::EQUAL_TO => parse_equal_to(parser),
        rules::STRICT_NOT_EQUAL => parse_strict_not_equal(parser),
        rules::STRICT_EQUAL => parse_strict_equal(parser),
        rules::GREATER_THAN => parse_greater_than(parser),
        rules::LESS_THAN => parse_less_than(parser),
        rules::GREATER_THAN_EQUAL => parse_greater_than_equal(parser),
        rules::LESS_THAN_EQUAL => parse_less_than_equal(parser),
        rules::OR => parse_or(parser),
        rules::AND => parse_and(parser),
        rules::NOT => parse_not(parser),
        rules::XOR => parse_xor(parser),
        rules::JOIN => parse_join(parser),
        rules::LEFT_JOIN => parse_left_join(parser),
        rules::RIGHT_JOIN => parse_right_join(parser),
        rules::FULL_JOIN => parse_full_join(parser),
        rules::LEFT_SEMI_JOIN => parse_left_semi_join(parser),
        rules::LEFT_ANTI_JOIN => parse_left_anti_join(parser),
        rules::UNION_OP => parse_union_op(parser),
        rules::INTERSECTION => parse_intersection(parser),
        rules::DIFFERENCE => parse_difference(parser),
        rules::COMPLEMENT => parse_complement(parser),
        rules::SUBSET => parse_subset(parser),
        rules::SUPERSET => parse_superset(parser),
        rules::PROPER_SUBSET => parse_proper_subset(parser),
        rules::PROPER_SUPERSET => parse_proper_superset(parser),
        rules::ELEMENT_OF => parse_element_of(parser),
        rules::NOT_ELEMENT_OF => parse_not_element_of(parser),
        rules::SYMMETRIC_DIFFERENCE => parse_symmetric_difference(parser),
        _ => unreachable!("Phase 2D support guard rejects every other RuleId"),
    })
}

pub(crate) fn parse_add_sub_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::ADD_SUB_OPERATOR,
        SyntaxKind::AddSubOperator,
        &[parse_add, parse_subtract],
    )
}

pub(crate) fn parse_mul_div_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::MUL_DIV_OPERATOR,
        SyntaxKind::MulDivOperator,
        &[parse_multiply, parse_divide, parse_modulus],
    )
}

pub(crate) fn parse_power_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::POWER_OPERATOR,
        SyntaxKind::PowerOperator,
        &[parse_power],
    )
}

pub(crate) fn parse_matrix_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::MATRIX_OPERATOR,
        SyntaxKind::MatrixOperator,
        &[
            parse_matrix_multiply,
            parse_matrix_solve,
            parse_dot_product,
            parse_cross_product,
        ],
    )
}

pub(crate) fn parse_range_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::RANGE_OPERATOR,
        SyntaxKind::RangeOperator,
        &[parse_range_inclusive, parse_range_exclusive],
    )
}

pub(crate) fn parse_comparison_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::COMPARISON_OPERATOR,
        SyntaxKind::ComparisonOperator,
        &[
            parse_strict_equal,
            parse_strict_not_equal,
            parse_not_equal,
            parse_equal_to,
            parse_greater_than_equal,
            parse_greater_than,
            parse_less_than_equal,
            parse_less_than,
        ],
    )
}

pub(crate) fn parse_logic_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::LOGIC_OPERATOR,
        SyntaxKind::LogicOperator,
        &[parse_and, parse_or, parse_xor],
    )
}

pub(crate) fn parse_table_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::TABLE_OPERATOR,
        SyntaxKind::TableOperator,
        &[
            parse_join,
            parse_left_join,
            parse_right_join,
            parse_full_join,
            parse_left_semi_join,
            parse_left_anti_join,
        ],
    )
}

pub(crate) fn parse_set_operator(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::SET_OPERATOR,
        SyntaxKind::SetOperator,
        &[
            parse_union_op,
            parse_intersection,
            parse_difference,
            parse_complement,
            parse_subset,
            parse_superset,
            parse_proper_subset,
            parse_proper_superset,
            parse_element_of,
            parse_not_element_of,
            parse_symmetric_difference,
        ],
    )
}

pub(crate) fn parse_add(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::ADD,
        SyntaxKind::AddOperation,
        &[OperatorAtom::CanonicalRule(rules::PLUS)],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_subtract(parser: &mut Parser<'_>) -> Attempt {
    parse_aggregate(
        parser,
        rules::SUBTRACT,
        SyntaxKind::SubtractOperation,
        &[parse_spaced_subtract, parse_raw_subtract],
    )
}

pub(crate) fn parse_raw_subtract(parser: &mut Parser<'_>) -> Attempt {
    parse_exact_leaf(
        parser,
        rules::RAW_SUBTRACT,
        SyntaxKind::RawSubtractOperation,
        &[OperatorAtom::CanonicalRule(rules::DASH)],
        OperatorGuard::NotCommentSigil,
    )
}

pub(crate) fn parse_spaced_subtract(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SPACED_SUBTRACT, |parser| {
        let subtract = parser.start();
        if !base::parse_rule(parser, rules::WS1E)
            || !parse_raw_subtract(parser).accepted()
            || !base::parse_rule(parser, rules::WS1E)
        {
            subtract.abandon(parser);
            return Attempt::NoMatch;
        }
        subtract.complete(parser, SyntaxKind::SpacedSubtractOperation);
        Attempt::Matched
    })
}

pub(crate) fn parse_multiply(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::MULTIPLY,
        SyntaxKind::MultiplyOperation,
        &[OperatorAtom::Text("*"), OperatorAtom::Text("×")],
        OperatorGuard::NotMatrixMultiply,
    )
}

pub(crate) fn parse_divide(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::DIVIDE,
        SyntaxKind::DivideOperation,
        &[
            OperatorAtom::CanonicalRule(rules::SLASH),
            OperatorAtom::Text("÷"),
        ],
        OperatorGuard::NotCommentSigil,
    )
}

pub(crate) fn parse_modulus(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::MODULUS,
        SyntaxKind::ModulusOperation,
        &[OperatorAtom::CanonicalRule(rules::PERCENT)],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_power(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::POWER,
        SyntaxKind::PowerOperation,
        &[OperatorAtom::CanonicalRule(rules::CARET)],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_matrix_multiply(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::MATRIX_MULTIPLY,
        SyntaxKind::MatrixMultiplyOperation,
        &[OperatorAtom::Text("**")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_matrix_solve(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::MATRIX_SOLVE,
        SyntaxKind::MatrixSolveOperation,
        &[OperatorAtom::CanonicalRule(rules::BACKSLASH)],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_dot_product(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::DOT_PRODUCT,
        SyntaxKind::DotProductOperation,
        &[OperatorAtom::Text("·"), OperatorAtom::Text("•")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_cross_product(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::CROSS_PRODUCT,
        SyntaxKind::CrossProductOperation,
        &[OperatorAtom::Text("⨯")],
        OperatorGuard::None,
    )
}

/// Parse the transparent apostrophe transpose marker.
pub(crate) fn parse_transpose(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::TRANSPOSE, |parser| {
        base::parse_rule(parser, rules::APOSTROPHE)
            .then_some(Attempt::Matched)
            .unwrap_or(Attempt::NoMatch)
    })
}

pub(crate) fn parse_range_inclusive(parser: &mut Parser<'_>) -> Attempt {
    parse_exact_leaf(
        parser,
        rules::RANGE_INCLUSIVE,
        SyntaxKind::RangeInclusiveOperation,
        &[OperatorAtom::Text("..=")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_range_exclusive(parser: &mut Parser<'_>) -> Attempt {
    parse_exact_leaf(
        parser,
        rules::RANGE_EXCLUSIVE,
        SyntaxKind::RangeExclusiveOperation,
        &[OperatorAtom::Text("..")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_not_equal(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::NOT_EQUAL,
        SyntaxKind::NotEqualOperation,
        &[
            OperatorAtom::Text("!="),
            OperatorAtom::Text("¬="),
            OperatorAtom::Text("≠"),
        ],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_equal_to(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::EQUAL_TO,
        SyntaxKind::EqualToOperation,
        &[OperatorAtom::Text("=="), OperatorAtom::Text("⩵")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_strict_not_equal(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::STRICT_NOT_EQUAL,
        SyntaxKind::StrictNotEqualOperation,
        &[
            OperatorAtom::Text("!=="),
            OperatorAtom::Text("!≡"),
            OperatorAtom::Text("¬≡"),
            OperatorAtom::Text("¬=="),
        ],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_strict_equal(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::STRICT_EQUAL,
        SyntaxKind::StrictEqualOperation,
        &[OperatorAtom::Text("==="), OperatorAtom::Text("≡")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_greater_than(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::GREATER_THAN,
        SyntaxKind::GreaterThanOperation,
        &[OperatorAtom::CanonicalRule(rules::RIGHT_ANGLE1)],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_less_than(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::LESS_THAN,
        SyntaxKind::LessThanOperation,
        &[OperatorAtom::CanonicalRule(rules::LEFT_ANGLE1)],
        OperatorGuard::NotGeneratorArrow,
    )
}

pub(crate) fn parse_greater_than_equal(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::GREATER_THAN_EQUAL,
        SyntaxKind::GreaterThanEqualOperation,
        &[OperatorAtom::Text(">="), OperatorAtom::Text("≥")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_less_than_equal(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::LESS_THAN_EQUAL,
        SyntaxKind::LessThanEqualOperation,
        &[OperatorAtom::Text("<="), OperatorAtom::Text("≤")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_or(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::OR,
        SyntaxKind::OrOperation,
        &[
            OperatorAtom::Text("||"),
            OperatorAtom::Text("∨"),
            OperatorAtom::Text("⋁"),
        ],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_and(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::AND,
        SyntaxKind::AndOperation,
        &[
            OperatorAtom::Text("&&"),
            OperatorAtom::Text("∧"),
            OperatorAtom::Text("⋀"),
        ],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_not(parser: &mut Parser<'_>) -> Attempt {
    parse_exact_leaf(
        parser,
        rules::NOT,
        SyntaxKind::NotOperation,
        &[
            OperatorAtom::CanonicalRule(rules::EXCLAMATION),
            OperatorAtom::CanonicalRule(rules::NEGATE),
        ],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_xor(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::XOR,
        SyntaxKind::XorOperation,
        &[
            OperatorAtom::Text("^^"),
            OperatorAtom::Text("⊕"),
            OperatorAtom::Text("⊻"),
        ],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_join(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::JOIN,
        SyntaxKind::JoinOperation,
        &[OperatorAtom::Text("⋈")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_left_join(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::LEFT_JOIN,
        SyntaxKind::LeftJoinOperation,
        &[OperatorAtom::Text("⟕")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_right_join(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::RIGHT_JOIN,
        SyntaxKind::RightJoinOperation,
        &[OperatorAtom::Text("⟖")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_full_join(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::FULL_JOIN,
        SyntaxKind::FullJoinOperation,
        &[OperatorAtom::Text("⟗")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_left_semi_join(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::LEFT_SEMI_JOIN,
        SyntaxKind::LeftSemiJoinOperation,
        &[OperatorAtom::Text("⋉")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_left_anti_join(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::LEFT_ANTI_JOIN,
        SyntaxKind::LeftAntiJoinOperation,
        &[OperatorAtom::Text("▷")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_union_op(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::UNION_OP,
        SyntaxKind::UnionOperation,
        &[OperatorAtom::Text("∪")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_intersection(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::INTERSECTION,
        SyntaxKind::IntersectionOperation,
        &[OperatorAtom::Text("∩")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_difference(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::DIFFERENCE,
        SyntaxKind::DifferenceOperation,
        &[OperatorAtom::Text("∖")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_complement(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::COMPLEMENT,
        SyntaxKind::ComplementOperation,
        &[OperatorAtom::Text("∁")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_subset(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::SUBSET,
        SyntaxKind::SubsetOperation,
        &[OperatorAtom::Text("⊆")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_superset(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::SUPERSET,
        SyntaxKind::SupersetOperation,
        &[OperatorAtom::Text("⊇")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_proper_subset(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::PROPER_SUBSET,
        SyntaxKind::ProperSubsetOperation,
        &[OperatorAtom::Text("⊊"), OperatorAtom::Text("⊂")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_proper_superset(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::PROPER_SUPERSET,
        SyntaxKind::ProperSupersetOperation,
        &[OperatorAtom::Text("⊋"), OperatorAtom::Text("⊃")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_element_of(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::ELEMENT_OF,
        SyntaxKind::ElementOfOperation,
        &[OperatorAtom::Text("∈")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_not_element_of(parser: &mut Parser<'_>) -> Attempt {
    parse_ws0_leaf(
        parser,
        rules::NOT_ELEMENT_OF,
        SyntaxKind::NotElementOfOperation,
        &[OperatorAtom::Text("∉")],
        OperatorGuard::None,
    )
}

pub(crate) fn parse_symmetric_difference(parser: &mut Parser<'_>) -> Attempt {
    parse_ws1_leaf(
        parser,
        rules::SYMMETRIC_DIFFERENCE,
        SyntaxKind::SymmetricDifferenceOperation,
        &[OperatorAtom::Text("Δ")],
    )
}

fn parse_exact_leaf(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    alternatives: &[OperatorAtom],
    guard: OperatorGuard,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let leaf = parser.start();
        if !guard_allows(parser, guard) || !parse_atom_alternatives(parser, alternatives) {
            leaf.abandon(parser);
            return Attempt::NoMatch;
        }
        leaf.complete(parser, kind);
        Attempt::Matched
    })
}

fn parse_ws0_leaf(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    alternatives: &[OperatorAtom],
    guard: OperatorGuard,
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let leaf = parser.start();
        if !base::parse_rule(parser, rules::WS0E)
            || !guard_allows(parser, guard)
            || !parse_atom_alternatives(parser, alternatives)
            || !base::parse_rule(parser, rules::WS0E)
        {
            leaf.abandon(parser);
            return Attempt::NoMatch;
        }
        leaf.complete(parser, kind);
        Attempt::Matched
    })
}

fn parse_ws1_leaf(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    alternatives: &[OperatorAtom],
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let leaf = parser.start();
        if !base::parse_rule(parser, rules::WS1E)
            || !parse_atom_alternatives(parser, alternatives)
            || !base::parse_rule(parser, rules::WS1E)
        {
            leaf.abandon(parser);
            return Attempt::NoMatch;
        }
        leaf.complete(parser, kind);
        Attempt::Matched
    })
}

fn parse_aggregate(
    parser: &mut Parser<'_>,
    rule: RuleId,
    kind: SyntaxKind,
    alternatives: &[fn(&mut Parser<'_>) -> Attempt],
) -> Attempt {
    combinator::transactional(parser, rule, |parser| {
        let aggregate = parser.start();
        for alternative in alternatives {
            if alternative(parser).accepted() {
                aggregate.complete(parser, kind);
                return Attempt::Matched;
            }
        }
        aggregate.abandon(parser);
        Attempt::NoMatch
    })
}

fn parse_atom_alternatives(parser: &mut Parser<'_>, alternatives: &[OperatorAtom]) -> bool {
    alternatives.iter().copied().any(|atom| match atom {
        OperatorAtom::CanonicalRule(rule) => base::parse_rule(parser, rule),
        OperatorAtom::Text(literal) => base::parse_exact_tag(parser, literal, SyntaxKind::Text),
    })
}

fn guard_allows(parser: &mut Parser<'_>, guard: OperatorGuard) -> bool {
    match guard {
        OperatorGuard::None => true,
        OperatorGuard::NotCommentSigil => not_ahead(parser, statements::parse_comment_sigil),
        OperatorGuard::NotMatrixMultiply => {
            not_ahead(parser, |parser| parse_matrix_multiply(parser).accepted())
        }
        OperatorGuard::NotGeneratorArrow => not_ahead(parser, |parser| {
            base::parse_rule(parser, rules::GENERATOR_ARROW)
        }),
    }
}

/// Probe a canonical child without retaining events, nodes, or diagnostics.
fn not_ahead(parser: &mut Parser<'_>, probe: impl FnOnce(&mut Parser<'_>) -> bool) -> bool {
    let checkpoint = parser.checkpoint();
    let matched = probe(parser);
    parser.rewind(checkpoint);
    !matched
}

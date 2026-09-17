//! Canonical closed expression-operator productions for Phase 2D.
//!
//! This module deliberately stops at the operator layer.  The recursive
//! expression parents select these productions in a later closed phase.

use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::combinator::Attempt;
use super::{base, statements};

mod continuation;
mod spec;
pub(crate) use continuation::{Continuation, Progress};

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
fn drive(parser: &mut Parser<'_>, rule: RuleId) -> Attempt {
    let mut continuation = Continuation::new(rule);
    loop {
        let mut allowance = u64::MAX;
        match continuation.advance(parser, true, &mut allowance) {
            Progress::Complete(result) => return result,
            Progress::NeedsProcessing => {}
            _ => unreachable!("final operator input"),
        }
    }
}
pub(crate) fn parse_add_sub_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::ADD_SUB_OPERATOR)
}

pub(crate) fn parse_mul_div_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MUL_DIV_OPERATOR)
}

pub(crate) fn parse_power_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::POWER_OPERATOR)
}

pub(crate) fn parse_matrix_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MATRIX_OPERATOR)
}

pub(crate) fn parse_range_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RANGE_OPERATOR)
}

pub(crate) fn parse_comparison_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::COMPARISON_OPERATOR)
}

pub(crate) fn parse_logic_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::LOGIC_OPERATOR)
}

pub(crate) fn parse_table_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TABLE_OPERATOR)
}

pub(crate) fn parse_set_operator(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SET_OPERATOR)
}

pub(crate) fn parse_add(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::ADD)
}

pub(crate) fn parse_subtract(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SUBTRACT)
}

pub(crate) fn parse_raw_subtract(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RAW_SUBTRACT)
}

pub(crate) fn parse_spaced_subtract(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SPACED_SUBTRACT)
}

pub(crate) fn parse_multiply(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MULTIPLY)
}

pub(crate) fn parse_divide(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::DIVIDE)
}

pub(crate) fn parse_modulus(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MODULUS)
}

pub(crate) fn parse_power(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::POWER)
}

pub(crate) fn parse_matrix_multiply(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MATRIX_MULTIPLY)
}

pub(crate) fn parse_matrix_solve(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::MATRIX_SOLVE)
}

pub(crate) fn parse_dot_product(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::DOT_PRODUCT)
}

pub(crate) fn parse_cross_product(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::CROSS_PRODUCT)
}

pub(crate) fn parse_transpose(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::TRANSPOSE)
}

pub(crate) fn parse_range_inclusive(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RANGE_INCLUSIVE)
}

pub(crate) fn parse_range_exclusive(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RANGE_EXCLUSIVE)
}

pub(crate) fn parse_not_equal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::NOT_EQUAL)
}

pub(crate) fn parse_equal_to(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::EQUAL_TO)
}

pub(crate) fn parse_strict_not_equal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::STRICT_NOT_EQUAL)
}

pub(crate) fn parse_strict_equal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::STRICT_EQUAL)
}

pub(crate) fn parse_greater_than(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::GREATER_THAN)
}

pub(crate) fn parse_less_than(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::LESS_THAN)
}

pub(crate) fn parse_greater_than_equal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::GREATER_THAN_EQUAL)
}

pub(crate) fn parse_less_than_equal(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::LESS_THAN_EQUAL)
}

pub(crate) fn parse_or(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::OR)
}

pub(crate) fn parse_and(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::AND)
}

pub(crate) fn parse_not(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::NOT)
}

pub(crate) fn parse_xor(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::XOR)
}

pub(crate) fn parse_join(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::JOIN)
}

pub(crate) fn parse_left_join(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::LEFT_JOIN)
}

pub(crate) fn parse_right_join(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::RIGHT_JOIN)
}

pub(crate) fn parse_full_join(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::FULL_JOIN)
}

pub(crate) fn parse_left_semi_join(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::LEFT_SEMI_JOIN)
}

pub(crate) fn parse_left_anti_join(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::LEFT_ANTI_JOIN)
}

pub(crate) fn parse_union_op(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::UNION_OP)
}

pub(crate) fn parse_intersection(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::INTERSECTION)
}

pub(crate) fn parse_difference(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::DIFFERENCE)
}

pub(crate) fn parse_complement(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::COMPLEMENT)
}

pub(crate) fn parse_subset(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SUBSET)
}

pub(crate) fn parse_superset(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SUPERSET)
}

pub(crate) fn parse_proper_subset(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::PROPER_SUBSET)
}

pub(crate) fn parse_proper_superset(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::PROPER_SUPERSET)
}

pub(crate) fn parse_element_of(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::ELEMENT_OF)
}

pub(crate) fn parse_not_element_of(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::NOT_ELEMENT_OF)
}

pub(crate) fn parse_symmetric_difference(parser: &mut Parser<'_>) -> Attempt {
    drive(parser, rules::SYMMETRIC_DIFFERENCE)
}

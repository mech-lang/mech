//! Canonical control operators entry points.
//! Recognition runs through shared retained primitive phases.

use crate::document::RuleId;

use super::super::Parser;
use super::super::rule::rules;
use super::combinator::Attempt;

/// The Phase 2G direct statement-boundary and control operator rules.
pub(crate) const PHASE_2G_CONTROL_RULES: &[RuleId; 9] = &[
    rules::STATEMENT_SEPARATOR,
    rules::OP_ASSIGN_OPERATOR,
    rules::ADD_ASSIGN_OPERATOR,
    rules::SUB_ASSIGN_OPERATOR,
    rules::MUL_ASSIGN_OPERATOR,
    rules::DIV_ASSIGN_OPERATOR,
    rules::EXP_ASSIGN_OPERATOR,
    rules::SEND_OPERATOR,
    rules::GUARD_OPERATOR,
];

/// Whether `rule` belongs to the Phase 2G control-operator layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2G_CONTROL_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2G control operator.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::STATEMENT_SEPARATOR => parse_statement_separator(parser),
        rules::OP_ASSIGN_OPERATOR => parse_op_assign_operator(parser),
        rules::ADD_ASSIGN_OPERATOR => parse_add_assign_operator(parser),
        rules::SUB_ASSIGN_OPERATOR => parse_sub_assign_operator(parser),
        rules::MUL_ASSIGN_OPERATOR => parse_mul_assign_operator(parser),
        rules::DIV_ASSIGN_OPERATOR => parse_div_assign_operator(parser),
        rules::EXP_ASSIGN_OPERATOR => parse_exp_assign_operator(parser),
        rules::SEND_OPERATOR => parse_send_operator(parser),
        rules::GUARD_OPERATOR => parse_guard_operator(parser),
        _ => unreachable!("Phase 2G control support guard rejects every other RuleId"),
    })
}

pub(crate) fn parse_statement_separator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::STATEMENT_SEPARATOR)
}
pub(crate) fn parse_op_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::OP_ASSIGN_OPERATOR)
}
pub(crate) fn parse_add_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::ADD_ASSIGN_OPERATOR)
}
pub(crate) fn parse_sub_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::SUB_ASSIGN_OPERATOR)
}
pub(crate) fn parse_mul_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::MUL_ASSIGN_OPERATOR)
}
pub(crate) fn parse_div_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::DIV_ASSIGN_OPERATOR)
}
pub(crate) fn parse_exp_assign_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::EXP_ASSIGN_OPERATOR)
}
pub(crate) fn parse_send_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::SEND_OPERATOR)
}
pub(crate) fn parse_guard_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::GUARD_OPERATOR)
}

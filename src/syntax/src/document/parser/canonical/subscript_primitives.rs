//! Canonical subscript primitives entry points.
//! Recognition runs through shared retained primitive phases.

use crate::document::RuleId;

use super::super::Parser;
use super::super::rule::rules;
use super::combinator::Attempt;

/// The Phase 2G subscript primitives with no recursive parent dependency.
pub(crate) const PHASE_2G_SUBSCRIPT_RULES: &[RuleId; 4] = &[
    rules::SELECT_ALL,
    rules::SWIZZLE_SUBSCRIPT,
    rules::DOT_SUBSCRIPT,
    rules::DOT_SUBSCRIPT_INT,
];

/// Whether `rule` belongs to the Phase 2G subscript primitive layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2G_SUBSCRIPT_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2G subscript primitive.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::SELECT_ALL => parse_select_all(parser),
        rules::SWIZZLE_SUBSCRIPT => parse_swizzle_subscript(parser),
        rules::DOT_SUBSCRIPT => parse_dot_subscript(parser),
        rules::DOT_SUBSCRIPT_INT => parse_dot_subscript_int(parser),
        _ => unreachable!("Phase 2G subscript support guard rejects every other RuleId"),
    })
}

pub(crate) fn parse_select_all(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::SELECT_ALL)
}
pub(crate) fn parse_swizzle_subscript(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::SWIZZLE_SUBSCRIPT)
}
pub(crate) fn parse_dot_subscript(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::DOT_SUBSCRIPT)
}
pub(crate) fn parse_dot_subscript_int(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::DOT_SUBSCRIPT_INT)
}

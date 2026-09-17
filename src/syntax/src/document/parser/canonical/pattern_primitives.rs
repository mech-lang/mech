//! Canonical pattern primitives entry points.
//! Recognition runs through shared retained primitive phases.
//! Candidate Phase 2I parents compose these leaves through recursive_core.

use crate::document::RuleId;

use super::super::Parser;
use super::super::rule::rules;
use super::combinator::Attempt;

/// The Phase 2G pattern primitives.
pub(crate) const PHASE_2G_PATTERN_RULES: &[RuleId; 2] = &[rules::WILDCARD, rules::SPREAD_OPERATOR];

/// Whether `rule` belongs to the Phase 2G pattern primitive layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2G_PATTERN_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2G pattern primitive.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::WILDCARD => parse_wildcard(parser),
        rules::SPREAD_OPERATOR => parse_spread_operator(parser),
        _ => unreachable!("Phase 2G pattern support guard rejects every other RuleId"),
    })
}

pub(crate) fn parse_wildcard(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::WILDCARD)
}
pub(crate) fn parse_spread_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::SPREAD_OPERATOR)
}

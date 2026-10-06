//! Canonical pattern primitives entry points.
//! Recognition runs through shared retained primitive phases.
//! Candidate executable grammar parents compose these leaves through recursive_core.

use crate::document::RuleId;

#[cfg(test)]
use super::super::Parser;
use super::super::rule::rules;
#[cfg(test)]
use super::combinator::Attempt;

/// The executable primitives pattern primitives.
pub(crate) const PRIMITIVE_PATTERN_RULES: &[RuleId; 2] = &[rules::WILDCARD, rules::SPREAD_OPERATOR];

/// Whether `rule` belongs to the executable primitives pattern primitive layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PRIMITIVE_PATTERN_RULES.contains(&rule)
}

/// Dispatch one exact executable primitives pattern primitive.
#[cfg(test)]
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::WILDCARD => parse_wildcard(parser),
        rules::SPREAD_OPERATOR => parse_spread_operator(parser),
        _ => unreachable!("executable primitives pattern support guard rejects every other RuleId"),
    })
}

#[cfg(test)]
pub(crate) fn parse_wildcard(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::WILDCARD)
}
#[cfg(test)]
pub(crate) fn parse_spread_operator(parser: &mut Parser<'_>) -> Attempt {
    super::primitives::parse_rule(parser, rules::SPREAD_OPERATOR)
}

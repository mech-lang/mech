//! Canonical closed pattern primitives for Phase 2G.
//!
//! Pattern parents remain unported; this module only exposes their acyclic
//! wildcard and spread leaves through the hidden direct-rule surface.

use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

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

/// Parse the direct wildcard prefix. A following asterisk is deliberately
/// left for the caller, matching the legacy prefix parser.
pub(crate) fn parse_wildcard(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::WILDCARD, |parser| {
        let wildcard = parser.start();
        if !base::parse_rule(parser, rules::ASTERISK) {
            wildcard.abandon(parser);
            return Attempt::NoMatch;
        }
        wildcard.complete(parser, SyntaxKind::WildcardPattern);
        Attempt::Matched
    })
}

/// Parse either whitespace-aware spread terminal in its formal source order.
///
/// This rule is transparent: the fixed-terminal tokens remain directly in
/// the lossless fragment without introducing a wrapper node.
pub(crate) fn parse_spread_operator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SPREAD_OPERATOR, |parser| {
        if base::parse_rule(parser, rules::SPREAD_OPERATOR_A)
            || base::parse_rule(parser, rules::SPREAD_OPERATOR_U)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

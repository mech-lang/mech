//! Canonical direct subscript primitives for the Phase 2G closed island.
//!
//! The complete `subscript` parent remains outside this phase. These direct
//! productions only retain the exact prefix recognized by their legacy peers.

use crate::document::{RuleId, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};
use super::literals;

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

/// Parse the direct `:` selector used by bracket and brace parents.
pub(crate) fn parse_select_all(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SELECT_ALL, |parser| {
        let subscript = parser.start();
        if !base::parse_rule(parser, rules::COLON) {
            subscript.abandon(parser);
            return Attempt::NoMatch;
        }
        subscript.complete(parser, SyntaxKind::SelectAllSubscript);
        Attempt::Matched
    })
}

/// Parse a dot followed by two or more comma-separated identifiers.
///
/// Each repeated `, identifier` pair is transactional. In particular, a
/// trailing comma is retained for a future parent instead of being consumed by
/// this direct leaf.
pub(crate) fn parse_swizzle_subscript(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::SWIZZLE_SUBSCRIPT, |parser| {
        let subscript = parser.start();
        if !base::parse_rule(parser, rules::PERIOD)
            || !base::parse_rule(parser, rules::IDENTIFIER)
            || !base::parse_rule(parser, rules::COMMA)
            || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            subscript.abandon(parser);
            return Attempt::NoMatch;
        }

        loop {
            let checkpoint = parser.checkpoint();
            if !base::parse_rule(parser, rules::COMMA)
                || !base::parse_rule(parser, rules::IDENTIFIER)
            {
                parser.rewind(checkpoint);
                break;
            }
            if parser.is_halted() {
                break;
            }
        }

        subscript.complete(parser, SyntaxKind::SwizzleSubscript);
        Attempt::Matched
    })
}

/// Parse one identifier dot subscript.
pub(crate) fn parse_dot_subscript(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::DOT_SUBSCRIPT, |parser| {
        let subscript = parser.start();
        if !base::parse_rule(parser, rules::PERIOD) || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            subscript.abandon(parser);
            return Attempt::NoMatch;
        }
        subscript.complete(parser, SyntaxKind::DotSubscript);
        Attempt::Matched
    })
}

/// Parse one integer-literal dot subscript using the Phase 2C literal rule.
pub(crate) fn parse_dot_subscript_int(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::DOT_SUBSCRIPT_INT, |parser| {
        let subscript = parser.start();
        if !base::parse_rule(parser, rules::PERIOD)
            || !literals::parse_integer_literal(parser).accepted()
        {
            subscript.abandon(parser);
            return Attempt::NoMatch;
        }
        subscript.complete(parser, SyntaxKind::DotSubscriptInt);
        Attempt::Matched
    })
}

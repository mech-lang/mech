//! Canonical context-path productions for the Phase 2C closed island.
//!
//! These direct productions intentionally do not provide a `var` dispatcher.
//! Their enclosing expression and variable grammar remains outside this phase.

use crate::document::SyntaxKind;

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

/// Parse the transparent `context-address-path-token` production.
///
/// The production emits its existing lexical token directly; it deliberately
/// has no syntax-node wrapper of its own.
pub(crate) fn parse_context_address_path_token(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_ADDRESS_PATH_TOKEN, |parser| {
        if base::parse_rule(parser, rules::ALPHA_TOKEN)
            || base::parse_rule(parser, rules::DIGIT_TOKEN)
            || base::parse_rule(parser, rules::DASH)
            || base::parse_rule(parser, rules::SLASH)
            || base::parse_rule(parser, rules::UNDERSCORE)
            || base::parse_rule(parser, rules::PERIOD)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse one or more exact context-address path tokens.
pub(crate) fn parse_context_address_path(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_ADDRESS_PATH, |parser| {
        let path = parser.start();
        let mut matched_any = false;

        loop {
            let before = parser.offset();
            if !parse_context_address_path_token(parser).accepted() {
                break;
            }
            matched_any = true;
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        if !matched_any {
            path.abandon(parser);
            return Attempt::NoMatch;
        }

        path.complete(parser, SyntaxKind::ContextAddressPath);
        Attempt::Matched
    })
}

/// Parse the closed `@context/path` production.
///
/// Incomplete prefixes stay noncommitting so a future enclosing alternative
/// can select another production without inheriting speculative diagnostics.
pub(crate) fn parse_prefixed_context_path(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::PREFIXED_CONTEXT_PATH, |parser| {
        let path = parser.start();
        if !base::parse_rule(parser, rules::AT)
            || !base::parse_rule(parser, rules::IDENTIFIER_PATH_SEGMENT)
            || !base::parse_rule(parser, rules::SLASH)
            || !parse_context_address_path(parser).accepted()
        {
            path.abandon(parser);
            return Attempt::NoMatch;
        }

        path.complete(parser, SyntaxKind::PrefixedContextPath);
        Attempt::Matched
    })
}

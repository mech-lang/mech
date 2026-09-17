//! Primitive canonical kind productions for the Phase 2C closed island.
//!
//! The complete `kind` dispatcher is deliberately absent: it reaches the
//! recursive expression and annotation closure that is not part of this phase.

use crate::document::SyntaxKind;

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

/// Parse the exact `kind-any` production.
pub(crate) fn parse_kind_any(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_ANY, |parser| {
        let kind = parser.start();
        if !base::parse_rule(parser, rules::ASTERISK) {
            kind.abandon(parser);
            return Attempt::NoMatch;
        }
        kind.complete(parser, SyntaxKind::KindAny);
        Attempt::Matched
    })
}

/// Parse the exact `kind-empty` production.
pub(crate) fn parse_kind_empty(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_EMPTY, |parser| {
        let kind = parser.start();
        if !base::parse_rule(parser, rules::UNDERSCORE) {
            kind.abandon(parser);
            return Attempt::NoMatch;
        }

        loop {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::UNDERSCORE) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        kind.complete(parser, SyntaxKind::KindEmpty);
        Attempt::Matched
    })
}

/// Parse the exact `kind-atom` production.
///
/// A colon without an identifier is a losing candidate and is restored in
/// full, with no provisional recovery diagnostic.
pub(crate) fn parse_kind_atom(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::KIND_ATOM, |parser| {
        let kind = parser.start();
        if !base::parse_rule(parser, rules::COLON) || !base::parse_rule(parser, rules::IDENTIFIER)
        {
            kind.abandon(parser);
            return Attempt::NoMatch;
        }
        kind.complete(parser, SyntaxKind::KindAtom);
        Attempt::Matched
    })
}

//! Canonical statement productions whose complete dependency closure is
//! available in Phase 2B.

use crate::document::SyntaxKind;

use super::super::Parser;
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

/// Parse the transparent `comment-sigil` production.
pub(crate) fn parse_comment_sigil(parser: &mut Parser<'_>) -> bool {
    let checkpoint = parser.checkpoint();
    let matched = parser.with_canonical_rule(rules::COMMENT_SIGIL, |parser| {
        if parser.cursor().grapheme_literal_end("--").is_some() {
            return base::parse_rule(parser, rules::DASH) && base::parse_rule(parser, rules::DASH);
        }
        if parser.cursor().grapheme_literal_end("//").is_some() {
            return base::parse_rule(parser, rules::SLASH)
                && base::parse_rule(parser, rules::SLASH);
        }
        false
    });
    if !matched {
        parser.rewind(checkpoint);
    }
    matched
}

/// Parse the canonical comment production as physical source content.
///
/// Comment text is deliberately retained as raw `any-token` children. Its
/// compatibility-value interpretation is separate from this concrete syntax
/// production.
pub(crate) fn parse_comment(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::COMMENT, |parser| {
        let comment = parser.start();
        while base::parse_rule(parser, rules::SPACE_TAB) {
            if parser.is_halted() {
                break;
            }
        }
        if !parse_comment_sigil(parser) {
            comment.abandon(parser);
            return Attempt::NoMatch;
        }

        while !at_line_end(parser) {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::ANY_TOKEN) {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        comment.complete(parser, SyntaxKind::Comment);
        Attempt::Matched
    })
}

fn at_line_end(parser: &Parser<'_>) -> bool {
    parser.is_eof() || matches!(parser.cursor().byte(), Some(b'\r' | b'\n'))
}

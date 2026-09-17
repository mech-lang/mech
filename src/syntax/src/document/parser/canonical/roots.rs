//! Canonical root and fragment dispatch.

use crate::document::{NodeFlags, SyntaxKind};

use super::super::Parser;
use super::super::rule::rules;
use super::grammar;

pub(crate) fn parse_grammar_root(parser: &mut Parser<'_>) {
    parser.with_canonical_rule(rules::PARSE_GRAMMAR, |parser| {
        let document = parser.start();
        let _ = grammar::parse_grammar(parser);
        document.complete_with_flags(parser, SyntaxKind::GrammarDocument, NodeFlags::REPARSE_ROOT);
    });
}

pub(crate) fn parse_grammar_fragment(parser: &mut Parser<'_>, kind: SyntaxKind) -> bool {
    match kind {
        SyntaxKind::Grammar => grammar::parse_grammar(parser),
        SyntaxKind::GrammarRule => grammar::parse_grammar_rule(parser).accepted(),
        SyntaxKind::GrammarExpression => grammar::parse_grammar_expression(parser).accepted(),
        SyntaxKind::GrammarTerm => grammar::parse_grammar_term(parser).accepted(),
        SyntaxKind::GrammarFactor => grammar::parse_grammar_factor(parser).accepted(),
        SyntaxKind::GrammarTerminalToken => {
            grammar::parse_grammar_terminal_token(parser).accepted()
        }
        _ => false,
    }
}

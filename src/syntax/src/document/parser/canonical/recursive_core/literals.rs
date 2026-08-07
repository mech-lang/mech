use crate::document::SyntaxKind;

use super::super::super::Parser;
use super::super::super::rule::rules;
use super::super::{combinator, literals as leaves};
use super::Attempt;
use super::kinds;

pub(super) fn parse_literal(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::LITERAL, |parser| {
        let node = parser.start();
        let selected = [
            leaves::parse_number,
            leaves::parse_string,
            leaves::parse_atom,
            leaves::parse_boolean,
            leaves::parse_empty,
            kinds::parse_kind_annotation,
        ]
        .into_iter()
        .find_map(|parse| {
            let result = parse(parser);
            (result != Attempt::NoMatch).then_some(result)
        })
        .unwrap_or(Attempt::NoMatch);

        match selected {
            Attempt::NoMatch => {
                node.abandon(parser);
                return Attempt::NoMatch;
            }
            Attempt::Committed => {
                node.complete(parser, SyntaxKind::Literal);
                return Attempt::Committed;
            }
            Attempt::Matched => {}
        }

        if kinds::parse_kind_annotation(parser) == Attempt::Committed {
            node.complete(parser, SyntaxKind::Literal);
            return Attempt::Committed;
        }
        node.complete(parser, SyntaxKind::Literal);
        Attempt::Matched
    })
}

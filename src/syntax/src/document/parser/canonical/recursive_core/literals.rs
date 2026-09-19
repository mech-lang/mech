use super::super::super::Parser;
use super::super::super::rule::rules;
use super::Attempt;

pub(super) fn parse_literal(parser: &mut Parser<'_>) -> Attempt {
    super::Continuation::new(rules::LITERAL).drive(parser)
}

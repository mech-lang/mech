#[cfg(test)]
use super::super::super::rule::rules;
use super::super::super::{Parser, marker::Marker};
#[cfg(test)]
use super::{Attempt, Continuation};
use crate::document::SyntaxKind;
#[cfg(test)]
pub(super) fn parse_expression(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::EXPRESSION).drive(parser)
}
#[cfg(test)]
pub(super) fn parse_formula(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FORMULA).drive(parser)
}
pub(super) fn finish_provisional_formula_marker(parser: &mut Parser<'_>, marker: Marker) {
    if parser.is_halted() {
        marker.complete(parser, SyntaxKind::Expression);
    } else {
        marker.abandon(parser);
    }
}

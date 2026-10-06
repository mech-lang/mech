use super::super::super::{Parser, rule::rules};
use super::{Attempt, Continuation};
pub(super) fn parse_subscript(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::SUBSCRIPT).drive(parser)
}
pub(super) fn parse_slice(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::SLICE).drive(parser)
}
pub(super) fn parse_bracket_subscript(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::BRACKET_SUBSCRIPT).drive(parser)
}
pub(super) fn parse_brace_subscript(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::BRACE_SUBSCRIPT).drive(parser)
}
pub(super) fn parse_formula_subscript(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FORMULA_SUBSCRIPT).drive(parser)
}
pub(super) fn parse_range_subscript(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::RANGE_SUBSCRIPT).drive(parser)
}

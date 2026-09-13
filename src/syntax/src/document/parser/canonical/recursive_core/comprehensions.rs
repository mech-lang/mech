use super::super::super::{Parser, rule::rules};
use super::{Attempt, Continuation};
pub(super) fn parse_set_comprehension(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::SET_COMPREHENSION).drive(parser)
}
pub(super) fn parse_matrix_comprehension(parser: &mut Parser<'_>) -> Attempt {
    super::structures::matrix_comprehension(parser)
}
pub(super) fn parse_comprehension_qualifier(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::COMPREHENSION_QUALIFIER).drive(parser)
}
pub(super) fn parse_generator(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::GENERATOR).drive(parser)
}

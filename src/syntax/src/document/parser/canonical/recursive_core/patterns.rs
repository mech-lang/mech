use super::super::super::{Parser, rule::rules};
use super::{Attempt, Continuation};
pub(super) fn parse_pattern(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PATTERN).drive(parser)
}
pub(super) fn parse_pattern_tuple_struct(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PATTERN_TUPLE_STRUCT).drive(parser)
}
pub(super) fn parse_pattern_array_item(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PATTERN_ARRAY_ITEM).drive(parser)
}
pub(super) fn parse_pattern_array_token(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PATTERN_ARRAY_TOKEN).drive(parser)
}
pub(super) fn parse_pattern_array(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PATTERN_ARRAY).drive(parser)
}
pub(super) fn parse_pattern_atom_struct(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PATTERN_ATOM_STRUCT).drive(parser)
}
pub(super) fn parse_pattern_tuple(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::PATTERN_TUPLE).drive(parser)
}

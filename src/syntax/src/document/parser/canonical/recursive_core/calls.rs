use super::super::super::{Parser, rule::rules};
use super::{Attempt, Continuation};
pub(super) fn parse_argument_list(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::ARGUMENT_LIST).drive(parser)
}
pub(super) fn parse_function_call(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FUNCTION_CALL).drive(parser)
}
pub(super) fn parse_call_arg_with_binding(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::CALL_ARG_WITH_BINDING).drive(parser)
}
pub(super) fn parse_call_arg(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::CALL_ARG).drive(parser)
}

use super::super::super::{Parser, rule::rules};
use super::{Attempt, Continuation};
pub(super) fn parse_fsm_pipe(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FSM_PIPE).drive(parser)
}
pub(super) fn parse_fsm_instance(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FSM_INSTANCE).drive(parser)
}
pub(super) fn parse_fsm_args(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FSM_ARGS).drive(parser)
}
pub(super) fn parse_fsm_state_transition(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FSM_STATE_TRANSITION).drive(parser)
}
pub(super) fn parse_fsm_async_transition(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FSM_ASYNC_TRANSITION).drive(parser)
}
pub(super) fn parse_fsm_output(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FSM_OUTPUT).drive(parser)
}
pub(super) fn parse_fsm_value(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::FSM_VALUE).drive(parser)
}

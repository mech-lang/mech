use super::super::super::{Parser, rule::rules};
use super::{Attempt, Continuation};
pub(super) fn parse_var(parser: &mut Parser<'_>) -> Attempt {
    Continuation::new(rules::VAR).drive(parser)
}
pub(super) fn parse_variable_define(parser: &mut Parser<'_>) -> Attempt {
    variable_definition(parser).0
}
/// Whether the definition operator was physically recognized, including recovery.
pub(super) fn variable_definition(parser: &mut Parser<'_>) -> (Attempt, bool) {
    let mut owner = Continuation::new(rules::VARIABLE_DEFINE);
    let result = owner.drive(parser);
    (result, owner.definition_operator())
}

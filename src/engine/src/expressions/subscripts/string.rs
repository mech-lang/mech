use super::Environment;
use crate::{FunctionValueRepresentation, InterpreterExecution, MResult, Subscript, ValueCell};

#[cfg(feature = "subscript_formula")]
pub(crate) fn reset_current_string_access_expression_live(p: &InterpreterExecution<'_>) {
    *p.current_string_access_expression_live.borrow_mut() = false;
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn current_string_access_expression_live(p: &InterpreterExecution<'_>) -> bool {
    *p.current_string_access_expression_live.borrow()
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn take_current_string_access_expression_live(p: &InterpreterExecution<'_>) -> bool {
    let value = *p.current_string_access_expression_live.borrow();
    *p.current_string_access_expression_live.borrow_mut() = false;
    value
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn mark_current_string_access_expression_live(p: &InterpreterExecution<'_>) {
    *p.current_string_access_expression_live.borrow_mut() = true;
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn mark_string_access_value_live(p: &InterpreterExecution<'_>, value: &ValueCell) {
    p.string_access_live_values
        .borrow_mut()
        .insert(value.reactive_cell_id());
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn string_access_value_is_marked_live(
    p: &InterpreterExecution<'_>,
    value: &ValueCell,
) -> bool {
    p.string_access_live_values
        .borrow()
        .contains(&value.reactive_cell_id())
}

#[cfg(feature = "subscript_formula")]
fn value_is_mutable_symbol(value: &ValueCell, p: &InterpreterExecution<'_>) -> bool {
    let state_brrw = p.state.borrow();
    let symbols_brrw = state_brrw.symbol_table.borrow();
    symbols_brrw
        .mutable_variables
        .values()
        .any(|symbol| symbol.same_cell(value))
}

#[cfg(feature = "subscript_formula")]
pub(super) fn string_access_argument_is_live(
    value: &ValueCell,
    p: &InterpreterExecution<'_>,
) -> bool {
    string_access_value_is_marked_live(p, value)
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn string_access_input_is_live(value: &ValueCell, p: &InterpreterExecution<'_>) -> bool {
    value_is_mutable_symbol(value, p) || string_access_argument_is_live(value, p)
}

#[cfg(feature = "subscript_formula")]
pub(super) fn string_access_index_argument(
    raw_index: ValueCell,
    _sbscrpt: &Subscript,
    _env: Option<&Environment>,
    _p: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    #[cfg(feature = "matrix")]
    {
        if matches!(
            raw_index.representation(),
            FunctionValueRepresentation::Matrix { .. }
        ) {
            return Ok(raw_index);
        }
        return crate::intrinsics::access::matrix::canonical_reactive_scalar_index(raw_index, _p);
    }
    #[cfg(not(feature = "matrix"))]
    return Ok(raw_index);
}

use super::super::variables::addressed_identifier_hash;
use super::{Environment, factor};
use crate::{
    Expression, Factor, InterpreterExecution, LegacyValue, MResult, MutableReference, Subscript,
    ValueKind,
};

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
fn string_access_scalar_addr(value: &LegacyValue) -> Option<usize> {
    match value {
        LegacyValue::MutableReference(reference) => string_access_scalar_addr(&reference.borrow()),
        LegacyValue::Typed(value, _) => string_access_scalar_addr(value),
        LegacyValue::String(value) => Some(value.addr()),
        LegacyValue::Index(value) => Some(value.addr()),

        #[cfg(feature = "u8")]
        LegacyValue::U8(value) => Some(value.addr()),
        #[cfg(feature = "u16")]
        LegacyValue::U16(value) => Some(value.addr()),
        #[cfg(feature = "u32")]
        LegacyValue::U32(value) => Some(value.addr()),
        #[cfg(feature = "u64")]
        LegacyValue::U64(value) => Some(value.addr()),
        #[cfg(feature = "u128")]
        LegacyValue::U128(value) => Some(value.addr()),

        #[cfg(feature = "i8")]
        LegacyValue::I8(value) => Some(value.addr()),
        #[cfg(feature = "i16")]
        LegacyValue::I16(value) => Some(value.addr()),
        #[cfg(feature = "i32")]
        LegacyValue::I32(value) => Some(value.addr()),
        #[cfg(feature = "i64")]
        LegacyValue::I64(value) => Some(value.addr()),
        #[cfg(feature = "i128")]
        LegacyValue::I128(value) => Some(value.addr()),

        #[cfg(feature = "f32")]
        LegacyValue::F32(value) => Some(value.addr()),
        #[cfg(feature = "f64")]
        LegacyValue::F64(value) => Some(value.addr()),

        _ => None,
    }
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn mark_string_access_value_live(p: &InterpreterExecution<'_>, value: &LegacyValue) {
    if let Some(addr) = string_access_scalar_addr(value) {
        p.string_access_live_values.borrow_mut().insert(addr);
    }
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn string_access_value_is_marked_live(
    p: &InterpreterExecution<'_>,
    value: &LegacyValue,
) -> bool {
    string_access_scalar_addr(value)
        .map(|addr| p.string_access_live_values.borrow().contains(&addr))
        .unwrap_or(false)
}

#[cfg(feature = "subscript_formula")]
fn subscript_formula_is_mutable_symbol(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> bool {
    if env.is_some() {
        return false;
    }
    let Subscript::Formula(fctr) = sbscrpt else {
        return false;
    };
    let Factor::Expression(expr) = fctr else {
        return false;
    };
    let Expression::Var(var) = expr.as_ref() else {
        return false;
    };
    let id = addressed_identifier_hash(&var.name, &var.context);
    let state_brrw = p.state.borrow();
    let symbols_brrw = state_brrw.symbol_table.borrow();
    symbols_brrw.get_mutable(id).is_some()
}

#[cfg(feature = "subscript_formula")]
fn mutable_reference_is_mutable_symbol(
    reference: &MutableReference,
    p: &InterpreterExecution<'_>,
) -> bool {
    let state_brrw = p.state.borrow();
    let symbols_brrw = state_brrw.symbol_table.borrow();
    symbols_brrw
        .mutable_variables
        .values()
        .any(|symbol| symbol.same_handle(reference))
}

#[cfg(feature = "subscript_formula")]
fn value_is_mutable_symbol_reference(value: &LegacyValue, p: &InterpreterExecution<'_>) -> bool {
    match value {
        LegacyValue::MutableReference(reference) => {
            mutable_reference_is_mutable_symbol(reference, p)
        }
        _ => false,
    }
}

#[cfg(feature = "subscript_formula")]
fn mutable_reference_is_live_plan_output(
    reference: &MutableReference,
    p: &InterpreterExecution<'_>,
) -> bool {
    let current = reference.borrow();
    string_access_value_is_marked_live(p, &current)
}

#[cfg(feature = "subscript_formula")]
pub(super) fn string_access_argument_is_live(
    value: &LegacyValue,
    p: &InterpreterExecution<'_>,
) -> bool {
    string_access_value_is_marked_live(p, value)
}

#[cfg(feature = "subscript_formula")]
pub(crate) fn string_access_input_is_live(
    value: &LegacyValue,
    p: &InterpreterExecution<'_>,
) -> bool {
    value_is_mutable_symbol_reference(value, p) || string_access_argument_is_live(value, p)
}

#[cfg(feature = "subscript_formula")]
pub(super) fn string_access_source_argument(
    value: &LegacyValue,
    p: &InterpreterExecution<'_>,
) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference)
            if matches!(value.deref_kind(), ValueKind::String)
                && !mutable_reference_is_mutable_symbol(reference, p)
                && !mutable_reference_is_live_plan_output(reference, p) =>
        {
            reference.borrow().clone()
        }
        _ => value.clone(),
    }
}

#[cfg(feature = "subscript_formula")]
pub(super) fn string_access_index_argument(
    raw_index: LegacyValue,
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    match &raw_index {
        LegacyValue::MutableReference(reference)
            if subscript_formula_is_mutable_symbol(sbscrpt, env, p)
                || mutable_reference_is_live_plan_output(reference, p) =>
        {
            reference.borrow().as_index()?;
            Ok(raw_index)
        }
        _ => raw_index.as_index(),
    }
}

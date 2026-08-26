use super::{Environment, UndefinedVariableError};
#[cfg(not(all(feature = "kind_annotation", feature = "convert")))]
use crate::{FeatureNotEnabledError, MechError};
use crate::{
    Identifier, InterpreterExecution, LegacyValue, MResult, MutableReference, Var, hash_str,
};
#[cfg(all(feature = "kind_annotation", feature = "convert"))]
use crate::{execute_catalog_operation, kind_annotation};

fn maybe_cast_variable_to_kind(
    variable: &Var,
    value: LegacyValue,
    #[cfg(all(feature = "kind_annotation", feature = "convert"))]
    interpreter: &InterpreterExecution<'_>,
    #[cfg(not(all(feature = "kind_annotation", feature = "convert")))] _: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let Some(annotation) = &variable.kind else {
        return Ok(value);
    };

    #[cfg(all(feature = "kind_annotation", feature = "convert"))]
    {
        let target_kind = {
            let state = interpreter.state.borrow();
            kind_annotation(&annotation.kind, interpreter)?.to_value_kind(&state.kinds)?
        };

        return execute_catalog_operation(
            interpreter,
            &interpreter.plan(),
            "convert/kind",
            vec![value, LegacyValue::Kind(target_kind)],
        );
    }

    #[cfg(not(all(feature = "kind_annotation", feature = "convert")))]
    {
        Err(MechError::new(FeatureNotEnabledError, None)
            .with_compiler_loc()
            .with_tokens(annotation.tokens()))
    }
}

pub(super) fn addressed_identifier_name(name: &Identifier, context: &Option<Identifier>) -> String {
    match context {
        Some(context) => format!("@{}/{}", context.to_string(), name.to_string()),
        None => name.to_string(),
    }
}

pub(super) fn addressed_identifier_hash(name: &Identifier, context: &Option<Identifier>) -> u64 {
    match context {
        Some(_) => hash_str(&addressed_identifier_name(name, context)),
        None => name.hash(),
    }
}

#[cfg(feature = "symbol_table")]
pub fn var(
    v: &Var,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let id = addressed_identifier_hash(&v.name, &v.context);
    let name = addressed_identifier_name(&v.name, &v.context);
    #[cfg(feature = "subscript_formula")]
    let mark_if_live_symbol = |value: &MutableReference| {
        use super::{
            mark_current_string_access_expression_live, string_access_value_is_marked_live,
        };

        let state_brrw = p.state.borrow();
        let symbols_brrw = state_brrw.symbol_table.borrow();
        if symbols_brrw.get_mutable(id).is_some()
            || string_access_value_is_marked_live(p, &value.borrow())
        {
            mark_current_string_access_expression_live(p);
        }
    };
    #[cfg(not(feature = "subscript_formula"))]
    let mark_if_live_symbol = |_: &MutableReference| {};
    if let Some(value) = env.and_then(|env| env.get(&id)) {
        return maybe_cast_variable_to_kind(v, value.clone(), p);
    }

    let symbol_value = {
        let state = p.state.borrow();
        let symbols = state.symbol_table.borrow();
        symbols.get(id)
    };
    if let Some(value) = symbol_value {
        mark_if_live_symbol(&value.legacy_ref());
        return maybe_cast_variable_to_kind(
            v,
            LegacyValue::MutableReference(value.legacy_ref()),
            p,
        );
    }
    if v.context.is_some() {
        return lower_missing_addressed_variable(v, p);
    }

    Err(
        crate::MechError::new(UndefinedVariableError { id, name }, None)
            .with_compiler_loc()
            .with_tokens(v.tokens()),
    )
}

fn lower_missing_addressed_variable(
    variable: &Var,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<LegacyValue> {
    let id = addressed_identifier_hash(&variable.name, &variable.context);
    let addressed_name = addressed_identifier_name(&variable.name, &variable.context);
    let output = crate::context_read(variable, interpreter)?;

    {
        let symbols = interpreter.symbols();
        let mut symbols = symbols.borrow_mut();
        symbols.insert_cell(id, output.clone(), false);
        symbols.dictionary.borrow_mut().insert(id, addressed_name);
    }

    maybe_cast_variable_to_kind(
        variable,
        LegacyValue::MutableReference(output.legacy_ref()),
        interpreter,
    )
}

use super::{Environment, UndefinedVariableError};
#[cfg(feature = "kind_annotation")]
use crate::{ConvertKind, execute_initialized_indexed_compiler, kind_annotation};
use crate::{Identifier, InterpreterExecution, MResult, MutableReference, Value, Var, hash_str};

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
pub fn var(v: &Var, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
    let plan = p.plan();
    let maybe_cast_to_kind = |value: Value| -> MResult<Value> {
        match &v.kind {
            Some(kind_anntn) => {
                let target_kind = {
                    let state_brrw = p.state.borrow();
                    kind_annotation(&kind_anntn.kind, p)?.to_value_kind(&state_brrw.kinds)?
                };
                execute_initialized_indexed_compiler(
                    p,
                    &plan,
                    &ConvertKind {},
                    vec![value, Value::Kind(target_kind)],
                )
            }
            None => Ok(value),
        }
    };

    let id = addressed_identifier_hash(&v.name, &v.context);
    let name = addressed_identifier_name(&v.name, &v.context);
    let mark_if_live_symbol = |value: &MutableReference| {
        #[cfg(feature = "subscript_formula")]
        {
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
        }
        #[cfg(not(feature = "subscript_formula"))]
        {
            let _ = value;
        }
    };
    match env {
        Some(env) => match env.get(&id) {
            Some(value) => maybe_cast_to_kind(value.clone()),
            None => {
                let state_brrw = p.state.borrow();
                let symbols_brrw = state_brrw.symbol_table.borrow();
                let symbol_value = symbols_brrw.get(id);
                drop(symbols_brrw);
                drop(state_brrw);
                match symbol_value {
                    Some(value) => {
                        mark_if_live_symbol(&value);
                        maybe_cast_to_kind(Value::MutableReference(value))
                    }
                    None => Err(crate::MechError::new(
                        UndefinedVariableError {
                            id,
                            name: name.clone(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                    .with_tokens(v.tokens())),
                }
            }
        },
        None => {
            let state_brrw = p.state.borrow();
            let symbols_brrw = state_brrw.symbol_table.borrow();
            let symbol_value = symbols_brrw.get(id);
            drop(symbols_brrw);
            drop(state_brrw);
            match symbol_value {
                Some(value) => {
                    mark_if_live_symbol(&value);
                    maybe_cast_to_kind(Value::MutableReference(value))
                }
                None => Err(crate::MechError::new(
                    UndefinedVariableError {
                        id,
                        name: name.clone(),
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(v.tokens())),
            }
        }
    }
}

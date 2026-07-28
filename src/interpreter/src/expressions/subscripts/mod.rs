use super::*;
use crate::*;

mod brace;
mod bracket;
mod dot;
mod string;

#[cfg(feature = "subscript_formula")]
pub(crate) use string::{
  current_string_access_expression_live, mark_current_string_access_expression_live,
  mark_string_access_value_live, reset_current_string_access_expression_live,
  string_access_input_is_live, string_access_value_is_marked_live,
  take_current_string_access_expression_live,
};
#[cfg(feature = "subscript_formula")]
use string::{
  string_access_argument_is_live, string_access_index_argument, string_access_source_argument,
};

#[cfg(all(feature = "subscript_slice", feature = "access"))]
pub fn slice(slc: &Slice, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
    let id = addressed_identifier_hash(&slc.name, &slc.context);
    let name = addressed_identifier_name(&slc.name, &slc.context);
    let val: Value = if let Some(env) = env {
        if let Some(val) = env.get(&id) {
            val.clone()
        } else {
            // fallback to global symbols
            {
                let symbols = p.symbols();
                let symbols_brrw = symbols.borrow();
                match symbols_brrw.get(id) {
                    Some(val) => match symbols_brrw.get_mutable(id) {
                        Some(_) => Value::MutableReference(val.clone()),
                        None => val.borrow().clone(),
                    },
                    None => {
                        return Err(MechError::new(UndefinedVariableError { id, name: name.clone() }, None)
                            .with_compiler_loc()
                            .with_tokens(slc.tokens()));
                    }
                }
            }
        }
    } else {
        let symbols = p.symbols();
        let symbols_brrw = symbols.borrow();
        match symbols_brrw.get(id) {
            Some(val) => match symbols_brrw.get_mutable(id) {
                Some(_) => Value::MutableReference(val.clone()),
                None => val.borrow().clone(),
            },
            None => {
                return Err(MechError::new(UndefinedVariableError { id, name: name.clone() }, None)
                    .with_compiler_loc()
                    .with_tokens(slc.tokens()));
            }
        }
    };
    let mut v = val;
    for s in &slc.subscript {
        v = subscript(s, &v, env, p)?;
    }
    Ok(v)
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    match sbscrpt {
        Subscript::Formula(fctr) => factor(fctr, env, p),
        _ => unreachable!(),
    }
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula_ix(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    match sbscrpt {
        Subscript::Formula(fctr) => {
            let result = factor(fctr, env, p)?;
            result.as_index()
        }
        _ => unreachable!(),
    }
}


#[cfg(feature = "subscript_range")]
pub fn subscript_range(
    sbscrpt: &Subscript,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    match sbscrpt {
        Subscript::Range(rng) => {
            let result = range(rng, env, p)?;
            match result.as_vecusize() {
                Ok(v) => Ok(v.to_value()),
                Err(_) => Err(MechError::new(
                    InvalidIndexKindError {
                        kind: result.kind(),
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(rng.tokens())),
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(all(feature = "subscript", feature = "access"))]
pub fn subscript(
    sbscrpt: &Subscript,
    val: &Value,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    let plan = p.plan();
    match sbscrpt {
        #[cfg(feature = "table")]
        Subscript::Dot(_) => dot::access(sbscrpt, val, p),
        Subscript::DotInt(_) => dot::access(sbscrpt, val, p),
        #[cfg(feature = "swizzle")]
        Subscript::Swizzle(_) => dot::access(sbscrpt, val, p),
        Subscript::Brace(_) => brace::access(sbscrpt, val, env, p),
        #[cfg(feature = "subscript_slice")]
        Subscript::Bracket(_) => bracket::access(sbscrpt, val, env, p),
        _ => unreachable!(),
    }
}

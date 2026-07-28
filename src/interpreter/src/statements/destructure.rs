#[cfg(feature = "tuple")]
use super::VariableAlreadyDefinedError;
#[cfg(feature = "tuple")]
use crate::{
  DestructureExpectedTupleError, InterpreterExecution, MResult, MechError,
  TupleDestructure, TupleDestructureTooManyVarsError, Value, expression,
};

#[cfg(feature = "tuple")]
pub fn tuple_destructure(tpl_dstrct: &TupleDestructure, p: &InterpreterExecution<'_>) -> MResult<Value> {
  let source = expression(&tpl_dstrct.expression, None, p)?;
  let tpl = match &source {
    Value::Tuple(tpl) => tpl,
    Value::MutableReference(r) => {
      let r_brrw = r.borrow();
      &match &*r_brrw {
        Value::Tuple(tpl) => tpl.clone(),
        _ => return Err(MechError::new(
          DestructureExpectedTupleError{ value: source.kind() },
          None
        ).with_compiler_loc().with_tokens(tpl_dstrct.expression.tokens())),
      }
    },
    _ => return Err(MechError::new(
      DestructureExpectedTupleError{ value: source.kind() },
      None
    ).with_compiler_loc().with_tokens(tpl_dstrct.expression.tokens())),
  };
  let symbols = p.symbols();
  let mut symbols_brrw = symbols.borrow_mut();
  for (i, var) in tpl_dstrct.vars.iter().enumerate() {
    let id = var.hash();
    if symbols_brrw.contains(id) {
      return Err(MechError::new(
        VariableAlreadyDefinedError { id },
        None
      ).with_compiler_loc().with_tokens(var.tokens()));
    }
    if let Some(element) = tpl.borrow().get(i) {
      symbols_brrw.insert(id, element.clone(), true);
      symbols_brrw.dictionary.borrow_mut().insert(id, var.name.to_string());
    } else {
      return Err(MechError::new(
        TupleDestructureTooManyVarsError{ value: source.kind() },
        None
      ).with_compiler_loc().with_tokens(var.tokens()));
    }
  }
  Ok(source)
}

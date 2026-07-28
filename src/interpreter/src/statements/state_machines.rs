#[cfg(feature = "state_machines")]
use super::variable_define::detach_variable_value;
#[cfg(feature = "state_machines")]
use crate::{Environment, FsmDeclare, InterpreterExecution, MResult, Value};

#[cfg(feature = "state_machines")]
pub fn fsm_declare(fsm_decl: &FsmDeclare, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
  let result = crate::state_machines::execute_fsm_pipe(&fsm_decl.pipe, env, p)?;
  let id = fsm_decl.fsm.name.hash();
  let name = fsm_decl.fsm.name.to_string();
  #[cfg(feature = "symbol_table")]
  {
    let symbols = p.symbols();
    let mut symbols_brrw = symbols.borrow_mut();
    symbols_brrw.insert(id, detach_variable_value(&result), false);
    symbols_brrw.dictionary.borrow_mut().insert(id, name);
  }
  Ok(result)
}

use super::{Environment, expression};
#[cfg(feature = "subscript_formula")]
use super::{
  current_string_access_expression_live, mark_current_string_access_expression_live,
  mark_string_access_value_live, string_access_input_is_live,
};
use crate::{
  FunctionCall, InterpreterExecution, MResult, MechError, MissingFunctionError, Value,
  execute_native_function_compiler, format_trace, format_trace_args,
};

// Dispatches a function call to whichever implementation is available:
// user-defined functions first, then built-in functions, then native compiled
// functions. Returns an error if the name is not found in any registry.
pub fn function_call(fxn_call: &FunctionCall, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
  let functions = p.functions();
  let fxn_name_id = fxn_call.name.hash();

  // User-defined function: evaluate arguments then run the interpreted body.
  if let Some(user_fxn) = { functions.borrow().user_functions.get(&fxn_name_id).cloned() } {
    let mut input_arg_values = vec![];
    for (_, arg_expr) in fxn_call.args.iter() {
      input_arg_values.push(expression(arg_expr, env, p)?);
    }
    #[cfg(feature = "subscript_formula")]
    let output_is_live = current_string_access_expression_live(p)
      || input_arg_values.iter().any(|value| string_access_input_is_live(value, p));
    let output =
      crate::functions::execute_user_function(&user_fxn, &input_arg_values, p)?;
    #[cfg(feature = "subscript_formula")]
    if output_is_live {
      mark_current_string_access_expression_live(p);
      mark_string_access_value_live(p, &output);
    }
    return Ok(output);
  }

  // Pre-compiled built-in functions.
  if { functions.borrow().functions.contains_key(&fxn_name_id) } {
    todo!();
  }

  // Native function compiler: the compiler picks a concrete implementation
  // based on the runtime argument types, then we execute it immediately.
  let fxn_compiler = {
    functions
      .borrow()
      .function_compilers
      .get(&fxn_name_id)
      .cloned()
  };
  match fxn_compiler {
    Some(fxn_compiler) => {
      let mut input_arg_values = vec![];
      for (_, arg_expr) in fxn_call.args.iter() {
        input_arg_values.push(expression(arg_expr, env, p)?);
      }
      trace_println!(
        p,
        "{}",
        format_trace(
          "fn",
          format!(
            "native {}({})",
            fxn_call.name.to_string(),
            format_trace_args(&input_arg_values)
          ),
        )
      );
      execute_native_function_compiler(fxn_compiler, &input_arg_values, p)
    }
    // No implementation found under this name at all.
    None => Err(MechError::new(
      MissingFunctionError {
        function_id: fxn_name_id,
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(fxn_call.name.tokens())),
  }
}

use crate::*;

// Functions
// ----------------------------------------------------------------------------

pub fn function_define(fxn_def: &FunctionDefine, p: &Interpreter) -> MResult<FunctionDefinition> {
  let fxn_name_id = fxn_def.name.hash();
  let mut new_fxn = FunctionDefinition::new(fxn_name_id, fxn_def.name.to_string(), fxn_def.clone());

  for input_arg in &fxn_def.input {
    new_fxn.input.insert(input_arg.name.hash(), input_arg.kind.clone());
  }

  for output_arg in &fxn_def.output {
    new_fxn.output.insert(output_arg.name.hash(), output_arg.kind.clone());
  }

  let functions = p.functions();
  let mut functions_brrw = functions.borrow_mut();
  functions_brrw.user_functions.insert(fxn_name_id, new_fxn.clone());
  functions_brrw.dictionary.borrow_mut().insert(fxn_name_id, fxn_def.name.to_string());
  p.state.borrow().dictionary.borrow_mut().insert(fxn_name_id, fxn_def.name.to_string());

  Ok(new_fxn)
}

pub fn function_call(fxn_call: &FunctionCall, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let functions = p.functions();
  let fxn_name_id = fxn_call.name.hash();
  let fxns_brrw = functions.borrow();

  if let Some(user_fxn) = fxns_brrw.user_functions.get(&fxn_name_id) {
    let mut input_arg_values = vec![];
    for (_, arg_expr) in fxn_call.args.iter() {
      input_arg_values.push(expression(arg_expr, None, p)?);
    }

    let compiled_fxn = compile_user_function(user_fxn, &input_arg_values, p)?;
    let new_fxn: Box<dyn MechFunction> = Box::new(mech_core::UserFunction { fxn: compiled_fxn });
    let mut plan_brrw = plan.borrow_mut();
    new_fxn.solve();
    let result = new_fxn.out();
    plan_brrw.push(new_fxn);
    return Ok(result);
  }

  match fxns_brrw.functions.get(&fxn_name_id) {
    Some(_) => {
      todo!();
    }
    None => {
      match fxns_brrw.function_compilers.get(&fxn_name_id) {
        Some(fxn_compiler) => {
          let mut input_arg_values = vec![];
          for (_, arg_expr) in fxn_call.args.iter() {
            input_arg_values.push(expression(arg_expr, None, p)?);
          }
          match fxn_compiler.compile(&input_arg_values) {
            Ok(new_fxn) => {
              let mut plan_brrw = plan.borrow_mut();
              new_fxn.solve();
              let result = new_fxn.out();
              plan_brrw.push(new_fxn);
              Ok(result)
            }
            Err(err) => Err(err),
          }
        }
        None => Err(MechError::new(
          MissingFunctionError { function_id: fxn_name_id },
          None,
        ).with_compiler_loc().with_tokens(fxn_call.name.tokens())),
      }
    }
  }
}

fn compile_user_function(
  fxn_def: &FunctionDefinition,
  input_arg_values: &Vec<Value>,
  p: &Interpreter,
) -> MResult<FunctionDefinition> {
  if input_arg_values.len() != fxn_def.input.len() {
    return Err(MechError::new(
      IncorrectNumberOfArguments {
        expected: fxn_def.input.len(),
        found: input_arg_values.len(),
      },
      None,
    ).with_compiler_loc().with_tokens(fxn_def.code.name.tokens()));
  }

  let mut compiled_fxn = FunctionDefinition::new(fxn_def.id, fxn_def.name.clone(), fxn_def.code.clone());
  compiled_fxn.input = fxn_def.input.clone();
  compiled_fxn.output = fxn_def.output.clone();

  let mut scoped_interpreter = Interpreter::new(hash_str(&format!("{:?}{:?}", fxn_def.id, input_arg_values.len())));
  {
    let parent_state = p.state.borrow();
    let mut scoped_state = scoped_interpreter.state.borrow_mut();
    scoped_state.symbol_table = compiled_fxn.symbols.clone();
    scoped_state.plan = compiled_fxn.plan.clone();
    scoped_state.functions = parent_state.functions.clone();
    scoped_state.kinds = parent_state.kinds.clone();
    scoped_state.dictionary = parent_state.dictionary.clone();
    #[cfg(feature = "enum")]
    {
      scoped_state.enums = parent_state.enums.clone();
    }
  }

  bind_function_inputs(&compiled_fxn, input_arg_values, &scoped_interpreter)?;

  for statement_node in &compiled_fxn.code.statements {
    statement(statement_node, None, &scoped_interpreter)?;
  }

  compiled_fxn.out = collect_function_output(&compiled_fxn)?;
  Ok(compiled_fxn)
}

fn bind_function_inputs(
  fxn_def: &FunctionDefinition,
  input_arg_values: &Vec<Value>,
  scoped_interpreter: &Interpreter,
) -> MResult<()> {
  let mut scoped_state = scoped_interpreter.state.borrow_mut();
  for ((arg_id, _), input_value) in fxn_def.input.iter().zip(input_arg_values.iter()) {
    let arg_name = fxn_def.code.input.iter()
      .find(|arg| arg.name.hash() == *arg_id)
      .map(|arg| arg.name.to_string())
      .unwrap_or_else(|| arg_id.to_string());
    let materialized_value = match input_value {
      Value::MutableReference(reference) => reference.borrow().clone(),
      value => value.clone(),
    };
    scoped_state.save_symbol(*arg_id, arg_name, materialized_value, false);
  }
  Ok(())
}

fn collect_function_output(fxn_def: &FunctionDefinition) -> MResult<ValRef> {
  let symbols = fxn_def.symbols.borrow();
  let mut outputs = vec![];

  for output_arg in &fxn_def.code.output {
    let output_id = output_arg.name.hash();
    match symbols.get(output_id) {
      Some(cell) => outputs.push(cell.borrow().clone()),
      None => {
        return Err(MechError::new(
          FunctionOutputUndefinedError { output_id },
          None,
        ).with_compiler_loc().with_tokens(output_arg.tokens()))
      }
    }
  }

  let output_value = match outputs.len() {
    0 => Value::Empty,
    1 => outputs.remove(0),
    _ => Value::Tuple(Ref::new(MechTuple::from_vec(outputs))),
  };

  Ok(Ref::new(output_value))
}

#[derive(Debug, Clone)]
pub struct MissingFunctionError {
  pub function_id: u64,
}

impl MechErrorKind for MissingFunctionError {
  fn name(&self) -> &str { "MissingFunction" }
  fn message(&self) -> String {
    format!("Function with id {} not found", self.function_id)
  }
}

#[derive(Debug, Clone)]
pub struct FunctionOutputUndefinedError {
  pub output_id: u64,
}

impl MechErrorKind for FunctionOutputUndefinedError {
  fn name(&self) -> &str { "FunctionOutputUndefined" }
  fn message(&self) -> String {
    format!("Function output {} was declared but never defined", self.output_id)
  }
}

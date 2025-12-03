use crate::*;

// Functions
// ----------------------------------------------------------------------------

pub fn function_define(fxn_def: &FunctionDefine, p: &Interpreter) -> MResult<FunctionDefinition> {
  /*let fxn_name_id = fxn_def.name.hash();
  let mut new_fxn = FunctionDefinition::new(fxn_name_id,fxn_def.name.to_string(), fxn_def.clone());
  for input_arg in &fxn_def.input {
    let arg_id = input_arg.name.hash();
    new_fxn.input.insert(arg_id,input_arg.kind.clone());
    let in_arg = Value::F64(Ref::new(F64::new(0.0)));
    new_fxn.symbols.borrow_mut().insert(arg_id, in_arg, false);
  }
  let output_arg_ids = fxn_def.output.iter().map(|output_arg| {
    let arg_id = output_arg.name.hash();
    new_fxn.output.insert(arg_id,output_arg.kind.clone());
    arg_id
  }).collect::<Vec<u64>>();
  
  for stmnt in &fxn_def.statements {
    let result = statement(stmnt, new_fxn.plan.clone(), new_fxn.symbols.clone(), functions.clone());
  }    
  // get the output cell
  {
    let symbol_brrw = new_fxn.symbols.borrow();
    for arg_id in output_arg_ids {
      match symbol_brrw.get(arg_id) {
        Some(cell) => new_fxn.out = cell.clone(),
        None => { return Err(MechError{file: file!().to_string(), tokens: fxn_def.output.iter().flat_map(|a| a.tokens()).collect(), msg: "".to_string(), id: line!(), kind: MechErrorKind::OutputUndefinedInFunctionBody(arg_id)});} 
      }
    }
  }
  Ok(new_fxn)*/
  todo!("Function define needs to be redone!");
}

pub fn function_call(fxn_call: &FunctionCall, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let functions = p.functions();
  let fxn_name_id = fxn_call.name.hash();
  let fxns_brrw = functions.borrow();
  match fxns_brrw.functions.get(&fxn_name_id) {
    Some(fxn) => {
      todo!();
    }
    None => { 
      match fxns_brrw.function_compilers.get(&fxn_name_id) {
        Some(fxn_compiler) => {
          let mut input_arg_values = vec![];
          for (arg_name, arg_expr) in fxn_call.args.iter() {
            let result = expression(&arg_expr, p)?;
            input_arg_values.push(result);
          }
          match fxn_compiler.compile(&input_arg_values) {
            Ok(new_fxn) => {
              let mut plan_brrw = plan.borrow_mut();
              new_fxn.solve();
              let result = new_fxn.out();
              plan_brrw.push(new_fxn);
              return Ok(result)
            }
            Err(x) => {return Err(x);}
          }
        }
        None => {return Err(MechError2::new(
            MissingFunctionError{ function_id: fxn_name_id },
            None
          ).with_compiler_loc().with_tokens(fxn_call.name.tokens())
        );}
      }
    }
  }   
  unreachable!()
}

#[derive(Debug, Clone)]
pub struct MissingFunctionError {
  pub function_id: u64,
}
impl MechErrorKind2 for MissingFunctionError {
  fn name(&self) -> &str { "MissingFunction" }
  fn message(&self) -> String {
    format!("Function with id {} not found", self.function_id)
  }
}
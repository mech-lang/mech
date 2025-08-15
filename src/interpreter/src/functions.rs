use crate::*;

// Functions
// ----------------------------------------------------------------------------

pub fn function_define(fxn_def: &FunctionDefine, p: &Interpreter) -> MResult<FunctionDefinition> {
  /*let fxn_name_id = fxn_def.name.hash();
  let mut new_fxn = FunctionDefinition::new(fxn_name_id,fxn_def.name.to_string(), fxn_def.clone());
  for input_arg in &fxn_def.input {
    let arg_id = input_arg.name.hash();
    new_fxn.input.insert(arg_id,input_arg.kind.clone());
    let in_arg = Value::F64(new_ref(F64::new(0.0)));
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
      let mut new_fxn = function_define(&fxn.code, p)?; // This just calles function_define again, it should be smarter.
      for (ix,(arg_name, arg_expr)) in fxn_call.args.iter().enumerate() {
        // Get the value
        let value_ref: ValRef = match arg_name {
          // Arg is called with a name
          Some(arg_id) => {
            match new_fxn.input.get(&arg_id.hash()) {
              // Arg name matches expected name
              Some(kind) => {
                let symbols_brrw = new_fxn.symbols.borrow();
                symbols_brrw.get(arg_id.hash()).unwrap().clone()
              }
              // The argument name doesn't match
              None => { return Err(MechError{file: file!().to_string(), tokens: arg_expr.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UnknownFunctionArgument(arg_id.hash())});}
            }
          }
          // Arg is called positionally (no arg name supplied)
          None => {
            match &new_fxn.input.iter().nth(ix) {
              Some((arg_id,kind)) => {
                let symbols_brrw = new_fxn.symbols.borrow();
                symbols_brrw.get(**arg_id).unwrap().clone()
              }
              None => { return Err(MechError{file: file!().to_string(), tokens: arg_expr.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::TooManyInputArguments(ix+1,new_fxn.input.len())});} 
            }
          }
        };
        let result = expression(&arg_expr, p)?;
        let mut ref_brrw = value_ref.borrow_mut();
        // TODO check types
        match (&mut *ref_brrw, &result) {
          #[cfg(feature = "bool")]
          (Value::Bool(arg_ref), Value::Bool(bool_ref)) => {*arg_ref.borrow_mut() = bool_ref.borrow().clone();}
          #[cfg(feature = "i8")]
          (Value::I8(arg_ref), Value::I8(i8_ref)) => {*arg_ref.borrow_mut() = i8_ref.borrow().clone();}
          #[cfg(feature = "i16")]
          (Value::I16(arg_ref), Value::I16(i16_ref)) => {*arg_ref.borrow_mut() = i16_ref.borrow().clone();}
          #[cfg(feature = "i32")]
          (Value::I32(arg_ref), Value::I32(i32_ref)) => {*arg_ref.borrow_mut() = i32_ref.borrow().clone();}
          #[cfg(feature = "i64")]
          (Value::I64(arg_ref), Value::I64(i64_ref)) => {*arg_ref.borrow_mut() = i64_ref.borrow().clone();}
          #[cfg(feature = "i128")]
          (Value::I128(arg_ref), Value::I128(i128_ref)) => {*arg_ref.borrow_mut() = i128_ref.borrow().clone();}
          #[cfg(feature = "u8")]
          (Value::U8(arg_ref), Value::U8(u8_ref)) => {*arg_ref.borrow_mut() = u8_ref.borrow().clone();}
          #[cfg(feature = "u16")]
          (Value::U16(arg_ref), Value::U16(u16_ref)) => {*arg_ref.borrow_mut() = u16_ref.borrow().clone();}
          #[cfg(feature = "u32")]
          (Value::U32(arg_ref), Value::U32(u32_ref)) => {*arg_ref.borrow_mut() = u32_ref.borrow().clone();}
          #[cfg(feature = "u64")]
          (Value::U64(arg_ref), Value::U64(u64_ref)) => {*arg_ref.borrow_mut() = u64_ref.borrow().clone();}
          #[cfg(feature = "u128")]
          (Value::U128(arg_ref), Value::U128(u128_ref)) => {*arg_ref.borrow_mut() = u128_ref.borrow().clone();}
          #[cfg(feature = "f32")]
          (Value::F32(arg_ref), Value::F32(f32_ref)) => {*arg_ref.borrow_mut() = f32_ref.borrow().clone();}
          #[cfg(feature = "f64")]
          (Value::F64(arg_ref), Value::F64(f64_ref)) => {*arg_ref.borrow_mut() = f64_ref.borrow().clone();}
          #[cfg(feature = "complex")]
          (Value::ComplexNumber(arg_ref), Value::ComplexNumber(complex_ref)) => {*arg_ref.borrow_mut() = complex_ref.borrow().clone();}
          #[cfg(feature = "rational")]
          (Value::RationalNumber(arg_ref), Value::RationalNumber(rational_ref)) => {*arg_ref.borrow_mut() = rational_ref.borrow().clone();}
          #[cfg(feature = "string")]
          (Value::String(arg_ref), Value::String(string_ref)) => {*arg_ref.borrow_mut() = string_ref.borrow().clone();}
          (x,y) => {return Err(MechError{file: file!().to_string(), tokens: arg_expr.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::KindMismatch(x.kind(),y.kind())});}
        }
      }
      // schedule function
      let mut plan_brrw = plan.borrow_mut();
      let result = new_fxn.solve();
      let result_brrw = result.borrow();
      plan_brrw.push(Box::new(UserFunction{fxn: new_fxn.clone()}));
      return Ok(result_brrw.clone())
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
        None => {return Err(MechError{file: file!().to_string(), tokens: fxn_call.name.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::MissingFunction(fxn_name_id)});}
      }
    }
  }   
  unreachable!()
}
use crate::*;

// Statements
// ----------------------------------------------------------------------------

pub fn statement(stmt: &Statement, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match stmt {
    Statement::VariableDefine(var_def) => variable_define(&var_def, plan.clone(), symbols.clone(), functions.clone()),
    Statement::VariableAssign(var_assgn) => variable_assign(&var_assgn, plan.clone(), symbols.clone(), functions.clone()),
    Statement::KindDefine(knd_def) => kind_define(&knd_def, plan.clone(), symbols.clone(), functions.clone()),
    Statement::EnumDefine(enm_def) => enum_define(&enm_def, plan.clone(), symbols.clone(), functions.clone()),
    Statement::FsmDeclare(_) => todo!(),
    Statement::SplitTable => todo!(),
    Statement::FlattenTable => todo!(),
  }
}

pub fn variable_assign(var_assgn: &VariableAssign, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut source = expression(&var_assgn.expression, plan.clone(), symbols.clone(), functions.clone())?;
  let slc = &var_assgn.target;
  let name = slc.name.hash();
  let symbols_brrw = symbols.borrow();
  let sink = match symbols_brrw.get(name) {
    Some(val) => val.borrow().clone(),
    None => {return Err(MechError{file: file!().to_string(), tokens: slc.name.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});}
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      for s in sbscrpt {
        let s_result = subscript_ref(&s, &sink, &source, plan.clone(), symbols.clone(), functions.clone())?;
        return Ok(s_result);
      }
    }
    None => {
      let args = vec![sink,source];
      let fxn = SetValue{}.compile(&args)?;
      fxn.solve();
      let mut plan_brrw = plan.borrow_mut();
      let res = fxn.out();
      plan_brrw.push(fxn);
      return Ok(res);
    }
  }
  unreachable!(); // subscript should have thrown an error if we can't access an element
}

pub fn enum_define(enm_def: &EnumDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = enm_def.name.hash();
  let variants = enm_def.variants.iter().map(|v| (v.name.hash(),None)).collect::<Vec<(u64, Option<Value>)>>();
  let mut fxns_brrw = functions.borrow_mut();
  let enm = MechEnum{id, variants};
  let val = Value::Enum(Box::new(enm.clone()));
  fxns_brrw.enums.insert(id, enm.clone());
  fxns_brrw.kinds.insert(id, val.kind());
  Ok(val)
}

pub fn kind_define(knd_def: &KindDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = knd_def.name.hash();
  let kind = kind_annotation(&knd_def.kind.kind, functions.clone())?;
  let value_kind = kind.to_value_kind(functions.clone())?;
  let mut fxns_brrw = functions.borrow_mut();
  fxns_brrw.kinds.insert(id, value_kind.clone());
  Ok(Value::Kind(value_kind))
}

pub fn variable_define(var_def: &VariableDefine, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let id = var_def.var.name.hash();
  if symbols.borrow().contains(id) {
    return Err(MechError{file: file!().to_string(), tokens: var_def.var.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::VariableRedefined(id)}); 
  }
  let mut result = expression(&var_def.expression, plan.clone(), symbols.clone(), functions.clone())?;
  if let Some(knd_anntn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_anntn.kind,functions.clone())?;
    let target_knd = knd.to_value_kind(functions.clone())?;
    // Do kind checking
    match (&result, &target_knd) {
      // Atom is a variant of an enum
      (Value::Atom(given_variant_id), ValueKind::Enum(enum_id)) => {
        let fxns_brrw = functions.borrow();
        let my_enum = match fxns_brrw.enums.get(enum_id) {
          Some(my_enum) => my_enum,
          None => todo!(),
        };
        // Given atom isn't a variant of the enum
        if !my_enum.variants.iter().any(|(enum_variant, inner_value)| *given_variant_id == *enum_variant) {
          return Err(MechError{file: file!().to_string(), tokens: var_def.expression.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UnknownEnumVairant(*enum_id,*given_variant_id)}); 
        }
      }
      // Atoms can't convert into anything else.
      (Value::Atom(given_variant_id), target_kind) => {
        return Err(MechError{file: file!().to_string(), tokens: var_def.expression.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UnableToConvertValueKind}); 
      }
      // Kind isn't checked
      x => {
        //return Err(MechError{file: file!().to_string(), tokens: var_def.expression.tokens(), msg: format!("{:?}",x).to_string(), id: line!(), kind: MechErrorKind::None}); 
      },
    }
    // Can we convert the kind?
    let convert_fxn = ConvertKind{}.compile(&vec![result.clone(), Value::Kind(target_knd)])?;
    convert_fxn.solve();
    let converted_result = convert_fxn.out();
    let mut plan_brrw = plan.borrow_mut();
    plan_brrw.push(convert_fxn);
    result = converted_result;
  };
  let mut symbols_brrw = symbols.borrow_mut();
  symbols_brrw.insert(id,result.clone());
  symbols_brrw.dictionary.insert(id,var_def.var.name.to_string());
  Ok(result)
}

pub fn subscript_ref(sbscrpt: &Subscript, sink: &Value, source: &Value, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Dot(x) => {
      let key = x.hash();
      let fxn_input: Vec<Value> = vec![sink.clone(), source.clone(), Value::Id(key)];
      let new_fxn = SetColumn{}.compile(&fxn_input)?;
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    Subscript::DotInt(x) => {
      todo!()
    },
    Subscript::Swizzle(x) => {
      unreachable!()
    },
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![sink.clone()];
      match &subs[..] {
        [Subscript::Formula(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = ixes.shape();
          fxn_input.push(ixes);
          match shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixSetScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixSetRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixSetRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_range(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(ixes);
          plan.borrow_mut().push(MatrixSetRange{}.compile(&fxn_input)?);
        },
        [Subscript::All] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixSetAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape1 = result.shape();
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape2 = result.shape();
          fxn_input.push(result);
          match ((shape1[0],shape1[1]),(shape2[0],shape2[1])) {
            ((1,1),(1,1)) => plan.borrow_mut().push(MatrixSetScalarScalar{}.compile(&fxn_input)?),
            ((1,1),(m,1)) => plan.borrow_mut().push(MatrixSetScalarRange{}.compile(&fxn_input)?),
            ((n,1),(1,1)) => plan.borrow_mut().push(MatrixSetRangeScalar{}.compile(&fxn_input)?),
            ((n,1),(m,1)) => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            _ => unreachable!(),
          }          
        },
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let ix = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = ix.shape();
          fxn_input.push(ix);
          match shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixSetAllScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixSetAllRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixSetAllRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        }
        [Subscript::Formula(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let ix = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = ix.shape();
          fxn_input.push(ix);
          fxn_input.push(Value::IndexAll);
          match shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixSetScalarAll{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixSetRangeAll{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixSetRangeAll{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixSetRangeScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixSetScalarRange{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::All,Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixSetAllRange{}.compile(&fxn_input)?);
        },
        [Subscript::Range(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixSetRangeAll{}.compile(&fxn_input)?);
        },
        _ => unreachable!(),
      };
      let plan_brrw = plan.borrow();
      let mut new_fxn = &plan_brrw.last().unwrap();
      new_fxn.solve();
      let res = new_fxn.out();
      return Ok(res);
    },
    Subscript::Brace(x) => todo!(),
    _ => unreachable!(),
  }
}
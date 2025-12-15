use crate::*;
use paste::paste;

#[cfg(feature = "variable_define")]
use crate::stdlib::define::*;

// Statements
// ----------------------------------------------------------------------------

pub fn statement(stmt: &Statement, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match stmt {
    #[cfg(feature = "tuple")]
    Statement::TupleDestructure(tpl_dstrct) => tuple_destructure(&tpl_dstrct, p),
    #[cfg(feature = "variable_define")]
    Statement::VariableDefine(var_def) => variable_define(&var_def, p),
    #[cfg(feature = "variable_assign")]
    Statement::VariableAssign(var_assgn) => variable_assign(&var_assgn, env, p),
    #[cfg(feature = "kind_define")]
    Statement::KindDefine(knd_def) => kind_define(&knd_def, p),
    #[cfg(feature = "enum")]
    Statement::EnumDefine(enm_def) => {
      enum_define(&enm_def, p)?;
      Ok(Value::Empty)
    }
    #[cfg(feature = "math")]
    Statement::OpAssign(op_assgn) => op_assign(&op_assgn, env, p),
    //Statement::FsmDeclare(_) => todo!(),
    //Statement::SplitTable => todo!(),
    //Statement::FlattenTable => todo!(),
    x => return Err(MechError2::new(
        FeatureNotEnabledError,
        None
      ).with_compiler_loc().with_tokens(x.tokens())
    ),
  }
}

#[cfg(feature = "tuple")]
pub fn tuple_destructure(tpl_dstrct: &TupleDestructure, p: &Interpreter) -> MResult<Value> {
  let source = expression(&tpl_dstrct.expression, None, p)?;
  let tpl = match &source {
    Value::Tuple(tpl) => tpl,
    Value::MutableReference(ref r) => {
      let r_brrw = r.borrow();
      &match &*r_brrw {
        Value::Tuple(tpl) => tpl.clone(),
        _ => return Err(MechError2::new(
          DestructureExpectedTupleError{ value: source.kind() },
          None
        ).with_compiler_loc().with_tokens(tpl_dstrct.expression.tokens())),
      }
    },
    _ => return Err(MechError2::new(
      DestructureExpectedTupleError{ value: source.kind() },
      None
    ).with_compiler_loc().with_tokens(tpl_dstrct.expression.tokens())),
  };
  let symbols = p.symbols();
  let mut symbols_brrw = symbols.borrow_mut();
  for (i, var) in tpl_dstrct.vars.iter().enumerate() {
    let id = var.hash();
    if symbols_brrw.contains(id) {
      return Err(MechError2::new(
        VariableAlreadyDefinedError { id },
        None
      ).with_compiler_loc().with_tokens(var.tokens()));
    }
    if let Some(element) = tpl.borrow().get(i) {
      symbols_brrw.insert(id, element.clone(), true);
      symbols_brrw.dictionary.borrow_mut().insert(id, var.name.to_string());
    } else {
      return Err(MechError2::new(
        TupleDestructureTooManyVarsError{ value: source.kind() },
        None
      ).with_compiler_loc().with_tokens(var.tokens()));
    }
  }
  Ok(source)
}

#[cfg(feature = "math")]
pub fn op_assign(op_assgn: &OpAssign, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let mut source = expression(&op_assgn.expression, env, p)?;
  let slc = &op_assgn.target;
  let id = slc.name.hash();
  let sink = { 
    let mut state_brrw = p.state.borrow_mut();
    match state_brrw.get_symbol(id) {
      Some(val) => val.borrow().clone(),
      None => {return Err(MechError2::new(
        UndefinedVariableError { id },
        None,
      ).with_compiler_loc().with_tokens(slc.name.tokens()));
      }
    }
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      // todo: this only works for the first subscript, it needs to work for multiple subscripts
      for s in sbscrpt {
        let fxn = match op_assgn.op {
          #[cfg(feature = "math_add_assign")]
          OpAssignOp::Add => add_assign(&s, &sink, &source, env, p)?,
          #[cfg(feature = "math_sub_assign")]
          OpAssignOp::Sub => sub_assign(&s, &sink, &source, env, p)?,
          #[cfg(feature = "math_div_assign")]
          OpAssignOp::Div => div_assign(&s, &sink, &source, env, p)?,
          #[cfg(feature = "math_mul_assign")]
          OpAssignOp::Mul => mul_assign(&s, &sink, &source, env, p)?,
          _ => todo!(),
        };
        return Ok(fxn);
      }
    }
    None => {
      let args = vec![sink,source];
      let fxn: Box<dyn MechFunction> = match op_assgn.op {
        #[cfg(feature = "math_add_assign")]
        OpAssignOp::Add => AddAssignValue{}.compile(&args)?,
        #[cfg(feature = "math_sub_assign")]
        OpAssignOp::Sub => SubAssignValue{}.compile(&args)?,
        #[cfg(feature = "math_div_assign")]
        OpAssignOp::Div => DivAssignValue{}.compile(&args)?,
        #[cfg(feature = "math_mul_assign")]
        OpAssignOp::Mul => MulAssignValue{}.compile(&args)?,
        _ => todo!(),
      };
      fxn.solve();
      let res = fxn.out();
      p.state.borrow_mut().add_plan_step(fxn);
      return Ok(res);
    }
  }
  unreachable!(); // subscript should have thrown an error if we can't access an element
}

#[cfg(feature = "variable_assign")]
pub fn variable_assign(var_assgn: &VariableAssign, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let mut source = expression(&var_assgn.expression, env, p)?;
  let slc = &var_assgn.target;
  let id = slc.name.hash();
  let sink = {
    let symbols = p.symbols();
    let symbols_brrw = symbols.borrow();
    match symbols_brrw.get_mutable(id) {
      Some(val) => val.borrow().clone(),
      None => {
        if !symbols_brrw.contains(id) {
          return Err(MechError2::new(
            UndefinedVariableError { id },
            Some("(!)> Variables are defined with the `:=` operator. *e.g.*: {{x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens()));
        } else { 
          return Err(MechError2::new(
            NotMutableError { id },
            Some("(!)> Mutable variables are defined with the `~` operator. *e.g.*: {{~x := 123}}".to_string()),
          ).with_compiler_loc().with_tokens(slc.name.tokens()));
        }
      }
    }
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      #[cfg(feature = "subscript")]
      for s in sbscrpt {
        let s_result = subscript_ref(&s, &sink, &source, env, p)?;
        return Ok(s_result);
      }
    }
    #[cfg(feature = "assign")]
    None => {
      let args = vec![sink,source];
      let fxn = AssignValue{}.compile(&args)?;
      fxn.solve();
      let res = fxn.out();
      p.state.borrow_mut().add_plan_step(fxn);
      return Ok(res);
    }
    _ => return Err(MechError2::new(
      FeatureNotEnabledError,
      None
    ).with_compiler_loc().with_tokens(var_assgn.target.tokens())),
  }
  unreachable!(); // subscript should have thrown an error if we can't access an element
}

#[cfg(feature = "enum")]
pub fn enum_define(enm_def: &EnumDefine, p: &Interpreter) -> MResult<()> {
  let id = enm_def.name.hash();
  let variants = enm_def.variants.iter().map(|v| (v.name.hash(),None)).collect::<Vec<(u64, Option<Value>)>>();
  let state = &p.state;
  let mut state_brrw = state.borrow_mut();
  let dictionary = state_brrw.dictionary.clone();
  {
    let mut dictionary_brrw = dictionary.borrow_mut();
    dictionary_brrw.insert(enm_def.name.hash(), enm_def.name.to_string());
    for variant in &enm_def.variants {
      dictionary_brrw.insert(variant.name.hash(), variant.name.to_string());
    }
  }
  let enm = MechEnum{id, variants, names: dictionary};
  let val = Value::Enum(Ref::new(enm.clone()));
  state_brrw.enums.insert(id, enm.clone());
  state_brrw.kinds.insert(id, val.kind());
  Ok(())
}

#[cfg(feature = "kind_define")]
pub fn kind_define(knd_def: &KindDefine, p: &Interpreter) -> MResult<Value> {
  let id = knd_def.name.hash();
  let kind = kind_annotation(&knd_def.kind.kind, p)?;
  let value_kind = kind.to_value_kind(&p.state.borrow().kinds)?;
  let functions = p.functions();
  let mut kinds = &mut p.state.borrow_mut().kinds;
  kinds.insert(id, value_kind.clone());
  Ok(Value::Kind(value_kind))
}

#[cfg(feature = "variable_define")]
pub fn variable_define(var_def: &VariableDefine, p: &Interpreter) -> MResult<Value> {
  let var_id = var_def.var.name.hash();
  let var_name = var_def.var.name.to_string();
  {
    let symbols = p.symbols();
    if symbols.borrow().contains(var_id) {
      return Err(MechError2::new(
        VariableAlreadyDefinedError { id: var_id },
        None
      ).with_compiler_loc().with_tokens(var_def.var.name.tokens()));
    }
  }
  let mut result = expression(&var_def.expression, None, p)?;
  #[cfg(all(feature = "kind_annotation", feature = "convert"))]
  if let Some(knd_anntn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_anntn.kind,p)?;
    let mut state_brrw = &mut p.state.borrow_mut();
    let target_knd = knd.to_value_kind(&mut state_brrw.kinds)?;
    // Do kind checking
    match (&result, &target_knd) {
      // Atom is a variant of an enum
      #[cfg(all(feature = "atom", feature = "enum"))]
      (Value::Atom(atom_variant), ValueKind::Enum(enum_id, enum_variant_name)) => {
        let atom_variant_brrw = atom_variant.borrow();
        let enums = &state_brrw.enums;
        let my_enum = match enums.get(enum_id) {
          Some(my_enum) => my_enum,
          None => todo!(),
        };
        let dictionary = state_brrw.dictionary.clone();
        let atom_id = atom_variant_brrw.id();
        let atom_name = atom_variant_brrw.name();
        // Given atom isn't a variant of the enum
        if !my_enum.variants.iter().any(|(enum_variant, inner_value)| atom_id == *enum_variant) {
          return Err(MechError2::new(
            UnableToConvertAtomToEnumVariantError { atom_name: atom_name, target_enum_variant_name: enum_variant_name.clone() },
            None
          ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
        }
      }
      // Atoms can't convert into anything else.
      #[cfg(feature = "atom")]
      (Value::Atom(given_variant_id), target_kind) => {
        return Err(MechError2::new(
          UnableToConvertAtomError { atom_id: given_variant_id.borrow().0.0},
          None
        ).with_compiler_loc().with_tokens(var_def.expression.tokens()));
      }
      #[cfg(feature = "matrix")]
      (Value::MutableReference(v), ValueKind::Matrix(box target_matrix_knd,_)) => {
        let value = v.borrow().clone();
        if value.is_matrix() {
          let convert_fxn = ConvertMatToMat{}.compile(&vec![result.clone(), Value::Kind(target_knd.clone())])?;
          convert_fxn.solve();
          let converted_result = convert_fxn.out();
          state_brrw.add_plan_step(convert_fxn);
          result = converted_result;
        } else {
          let value_kind = value.kind();
          if value_kind.deref_kind() != target_matrix_knd.clone() && value_kind != *target_matrix_knd {
            let convert_fxn = ConvertKind{}.compile(&vec![result.clone(), Value::Kind(target_matrix_knd.clone())])?;
            convert_fxn.solve();
            let converted_result = convert_fxn.out();
            state_brrw.add_plan_step(convert_fxn);
            result = converted_result;
          };
          let convert_fxn = ConvertScalarToMat{}.compile(&vec![result.clone(), Value::Kind(target_knd.clone())])?;
          convert_fxn.solve();
          let converted_result = convert_fxn.out();
          state_brrw.add_plan_step(convert_fxn);
          result = converted_result;          
        }
      }
      #[cfg(feature = "matrix")]
      (value, ValueKind::Matrix(box target_matrix_knd,_)) => {
        if value.is_matrix() {
          let convert_fxn = ConvertMatToMat{}.compile(&vec![result.clone(), Value::Kind(target_knd.clone())])?;
          convert_fxn.solve();
          let converted_result = convert_fxn.out();
          state_brrw.add_plan_step(convert_fxn);
          result = converted_result;
        } else {
          let value_kind = value.kind();
          if value_kind.deref_kind() != target_matrix_knd.clone() && value_kind != *target_matrix_knd {
            let convert_fxn = ConvertKind{}.compile(&vec![result.clone(), Value::Kind(target_matrix_knd.clone())])?;
            convert_fxn.solve();
            let converted_result = convert_fxn.out();
            state_brrw.add_plan_step(convert_fxn);
            result = converted_result;
          };
          let convert_fxn = ConvertScalarToMat{}.compile(&vec![result.clone(), Value::Kind(target_knd.clone())])?;
          convert_fxn.solve();
          let converted_result = convert_fxn.out();
          state_brrw.add_plan_step(convert_fxn);
          result = converted_result;
        }
      }
      // Kind isn't checked
      x => {
        let convert_fxn = ConvertKind{}.compile(&vec![result.clone(), Value::Kind(target_knd)])?;
        convert_fxn.solve();
        let converted_result = convert_fxn.out();
        state_brrw.add_plan_step(convert_fxn);
        result = converted_result;
      },
    };
    // Save symbol to interpreter
    let val_ref = state_brrw.save_symbol(var_id, var_name.clone(), result.clone(), var_def.mutable);
    // Add variable define step to plan
    let var_def_fxn = VarDefine{}.compile(&vec![result.clone(), Value::String(Ref::new(var_name.clone())), Value::Bool(Ref::new(var_def.mutable))])?;
    state_brrw.add_plan_step(var_def_fxn);
    return Ok(result);
  } 
  let mut state_brrw = p.state.borrow_mut();
  // Save symbol to interpreter
  let val_ref = state_brrw.save_symbol(var_id,var_name.clone(),result.clone(),var_def.mutable);
  // Add variable define step to plan
  let var_def_fxn = VarDefine{}.compile(&vec![result.clone(), Value::String(Ref::new(var_name.clone())), Value::Bool(Ref::new(var_def.mutable))])?;
  state_brrw.add_plan_step(var_def_fxn);
  return Ok(result);
}

macro_rules! op_assign {
  ($fxn_name:ident, $op:tt) => {
    paste!{
      pub fn $fxn_name(sbscrpt: &Subscript, sink: &Value, source: &Value, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
        let plan = p.plan();
        match sbscrpt {
          Subscript::Dot(x) => {
            todo!()
          },
          Subscript::DotInt(x) => {
            todo!()
          },
          Subscript::Swizzle(x) => {
            todo!()
          },
          Subscript::Bracket(subs) => {
            let mut fxn_input = vec![sink.clone()];
            match &subs[..] {
              [Subscript::Formula(ix)] => {
                fxn_input.push(source.clone());
                let ixes = subscript_formula(&subs[0], env, p)?;
                let shape = ixes.shape();
                fxn_input.push(ixes);
                match shape[..] {
                  [1,1] => plan.borrow_mut().push(MatrixAssignScalar{}.compile(&fxn_input)?),
                  [1,n] => plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?),
                  [n,1] => plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?),
                  _ => todo!(),
                }
              },
              [Subscript::Formula(ix1),Subscript::All] => {
                fxn_input.push(source.clone());
                let ix = subscript_formula(&subs[0], env, p)?;
                let shape = ix.shape();
                fxn_input.push(ix);
                fxn_input.push(Value::IndexAll);
                match shape[..] {
                  [1,1] => plan.borrow_mut().push(MatrixAssignScalarAll{}.compile(&fxn_input)?),
                  [1,n] => plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?),
                  [n,1] => plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?),
                  _ => todo!(),
                }
              },
              [Subscript::Range(ix)] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], env, p)?;
                fxn_input.push(ixes);
                plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?);
              },
              [Subscript::Range(ix), Subscript::All] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], env, p)?;
                fxn_input.push(ixes);
                fxn_input.push(Value::IndexAll);
                plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?);
              },
              x => todo!("{:?}", x),
            };
            let plan_brrw = plan.borrow();
            let mut new_fxn = &plan_brrw.last().unwrap();
            new_fxn.solve();
            let res = new_fxn.out();
            return Ok(res);
          },
          Subscript::Brace(x) => todo!(),
          x => todo!("{:?}", x),
        }
      }
    }}}

#[cfg(feature = "math_add_assign")]
op_assign!(add_assign, Add);
#[cfg(feature = "math_sub_assign")]
op_assign!(sub_assign, Sub);
#[cfg(feature = "math_div_assign")]
op_assign!(mul_assign, Mul);
#[cfg(feature = "math_mul_assign")]
op_assign!(div_assign, Div);
//#[cfg(feature = "math_pow")]
//op_assign!(pow_assign, Pow);

#[cfg(all(feature = "subscript", feature = "assign"))]
pub fn subscript_ref(sbscrpt: &Subscript, sink: &Value, source: &Value, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let symbols = p.symbols();
  let functions = p.functions();
  match sbscrpt {
    Subscript::Dot(x) => {
      let key = x.hash();
      let fxn_input: Vec<Value> = vec![sink.clone(), source.clone(), Value::Id(key)];
      let new_fxn = AssignColumn{}.compile(&fxn_input)?;
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
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_formula(&subs[0], env, p)?;
          let shape = ixes.shape();
          fxn_input.push(ixes);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAssignScalar{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range", feature = "assign"))]
            [1,n] => plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range", feature = "assign"))]
            [n,1] => plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_range(&subs[0], env, p)?;
          fxn_input.push(ixes);
          plan.borrow_mut().push(MatrixAssignRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::All] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAssignAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result1 = subscript_formula(&subs[0], env, p)?;
          let result2 = subscript_formula(&subs[1], env, p)?;
          let shape1 = result1.shape();
          let shape2 = result2.shape();
          fxn_input.push(result1);
          fxn_input.push(result2);
          match ((shape1[0],shape1[1]),(shape2[0],shape2[1])) {
            #[cfg(feature = "matrix")]
            ((1,1),(1,1)) => plan.borrow_mut().push(MatrixAssignScalarScalar{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((1,1),(m,1)) => plan.borrow_mut().push(MatrixAssignScalarRange{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((n,1),(1,1)) => plan.borrow_mut().push(MatrixAssignRangeScalar{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((n,1),(m,1)) => plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?),
            _ => unreachable!(),
          }          
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_formula"))]
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let ix = subscript_formula(&subs[1], env, p)?;
          let shape = ix.shape();
          fxn_input.push(ix);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAssignAllScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixAssignAllRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixAssignAllRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        }
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let ix = subscript_formula(&subs[0], env, p)?;
          let shape = ix.shape();
          fxn_input.push(ix);
          fxn_input.push(Value::IndexAll);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAssignScalarAll{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [1,n] => plan.borrow_mut().push(MatrixAssignRangeAll{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [n,1] => plan.borrow_mut().push(MatrixAssignRangeAll{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAssignRangeScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "subscript_formula", feature = "subscript_range"))]
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_formula(&subs[0], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAssignScalarRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixAssignRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::All,Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAssignAllRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAssignRangeAll{}.compile(&fxn_input)?);
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

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomToEnumVariantError {
  pub atom_name: String,
  pub target_enum_variant_name: String,
}
impl MechErrorKind2 for UnableToConvertAtomToEnumVariantError {
  fn name(&self) -> &str {
    "UnableToConvertAtomToEnumVariant"
  }
  fn message(&self) -> String {
    format!("Unable to convert atom variant `{} to enum <{}>", self.atom_name, self.target_enum_variant_name)
  }
}

#[derive(Debug, Clone)]
pub struct UnableToConvertAtomError {
  pub atom_id: u64,
}
impl MechErrorKind2 for UnableToConvertAtomError {
  fn name(&self) -> &str {
    "UnableToConvertAtom"
  }
  fn message(&self) -> String {
    format!("Unable to atom  {}", self.atom_id)
  }
}

#[derive(Debug, Clone)]
pub struct VariableAlreadyDefinedError {
  pub id: u64,
}
impl MechErrorKind2 for VariableAlreadyDefinedError {
  fn name(&self) -> &str { "VariableAlreadyDefined" }
  fn message(&self) -> String {
    format!("Variable already defined: {}", self.id)
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
  pub id: u64,
}
impl MechErrorKind2 for UndefinedVariableError {
  fn name(&self) -> &str { "UndefinedVariable" }

  fn message(&self) -> String {
    format!("Undefined variable: {}", self.id)
  }
}

#[derive(Debug, Clone)]
pub struct NotMutableError {
  pub id: u64,
}
impl MechErrorKind2 for NotMutableError {
  fn name(&self) -> &str { "NotMutable" }
  fn message(&self) -> String {
    format!("Variable is not mutable: {}", self.id)
  }
}


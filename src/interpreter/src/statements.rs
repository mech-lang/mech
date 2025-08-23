use crate::*;
use paste::paste;

// Statements
// ----------------------------------------------------------------------------

pub fn statement(stmt: &Statement, p: &Interpreter) -> MResult<Value> {
  match stmt {
    #[cfg(feature = "tuple")]
    Statement::TupleDestructure(tpl_dstrct) => tuple_destructure(&tpl_dstrct, p),
    #[cfg(feature = "variable_define")]
    Statement::VariableDefine(var_def) => variable_define(&var_def, p),
    #[cfg(feature = "variable_assign")]
    Statement::VariableAssign(var_assgn) => variable_assign(&var_assgn, p),
    #[cfg(feature = "kind_define")]
    Statement::KindDefine(knd_def) => kind_define(&knd_def, p),
    #[cfg(feature = "enum")]
    Statement::EnumDefine(enm_def) => enum_define(&enm_def, p),
    #[cfg(feature = "math")]
    Statement::OpAssign(op_assgn) => op_assign(&op_assgn, p),
    //Statement::FsmDeclare(_) => todo!(),
    //Statement::SplitTable => todo!(),
    //Statement::FlattenTable => todo!(),
    x => return Err(MechError{file:file!().to_string(),tokens:x.tokens(),msg: format!("Feature not enabled {:?}", x), id:line!(),kind:MechErrorKind::None}),
  }
}

#[cfg(feature = "tuple")]
pub fn tuple_destructure(tpl_dstrct: &TupleDestructure, p: &Interpreter) -> MResult<Value> {
  let source = expression(&tpl_dstrct.expression, p)?;
  let tpl = match &source {
    Value::Tuple(tpl) => tpl,
    Value::MutableReference(ref r) => {
      let r_brrw = r.borrow();
      &match &*r_brrw {
        Value::Tuple(tpl) => tpl.clone(),
        _ => return Err(MechError{file:file!().to_string(),tokens:tpl_dstrct.vars[0].tokens(),msg:"Expected a tuple.".to_string(),id:line!(),kind:MechErrorKind::GenericError("Expected a tuple.".to_string())}),
      }
    },
    _ => return Err(MechError{file:file!().to_string(),tokens:tpl_dstrct.vars[0].tokens(),msg:format!("Expected a tuple, found: {}", source.kind()),id:line!(),kind:MechErrorKind::GenericError(format!("Expected a tuple, found: {}", source.kind()))}),
  };
  let symbols = p.symbols();
  let mut symbols_brrw = symbols.borrow_mut();
  for (i, var) in tpl_dstrct.vars.iter().enumerate() {
    let id = var.hash();
    if symbols_brrw.contains(id) {
      return Err(MechError{file:file!().to_string(),tokens:var.tokens(),msg:"Note: Variables are defined with the := operator.".to_string(),id:line!(),kind:MechErrorKind::VariableRedefined(id)});
    }
    if let Some(element) = tpl.get(i) {
      symbols_brrw.insert(id, element.clone(), true);
      symbols_brrw.dictionary.borrow_mut().insert(id, var.name.to_string());
    } else {
      return Err(MechError{file:file!().to_string(),tokens:var.tokens(),msg:"Tuple destructure has more variables than elements in the tuple.".to_string(),id:line!(),kind:MechErrorKind::IndexOutOfBounds});
    }
  }
  Ok(source)
}

#[cfg(feature = "math")]
pub fn op_assign(op_assgn: &OpAssign, p: &Interpreter) -> MResult<Value> {
  let mut source = expression(&op_assgn.expression, p)?;
  let slc = &op_assgn.target;
  let name = slc.name.hash();
  let sink = { 
    let mut state_brrw = p.state.borrow_mut();
    match state_brrw.get_symbol(name) {
      Some(val) => val.borrow().clone(),
      None => {return Err(MechError{file: file!().to_string(), tokens: slc.name.tokens(), msg: "Note: Variables are defined with the := operator.".to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});}
    }
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      // todo: this only works for the first subscript, it needs to work for multiple subscripts
      for s in sbscrpt {
        let fxn = match op_assgn.op {
          #[cfg(feature = "math_add_assign")]
          OpAssignOp::Add => add_assign(&s, &sink, &source, p)?,
          #[cfg(feature = "math_sub_assign")]
          OpAssignOp::Sub => sub_assign(&s, &sink, &source, p)?,
          #[cfg(feature = "math_div_assign")]
          OpAssignOp::Div => div_assign(&s, &sink, &source, p)?,
          #[cfg(feature = "math_mul_assign")]
          OpAssignOp::Mul => mul_assign(&s, &sink, &source, p)?,
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
pub fn variable_assign(var_assgn: &VariableAssign, p: &Interpreter) -> MResult<Value> {
  let mut source = expression(&var_assgn.expression, p)?;
  let slc = &var_assgn.target;
  let name = slc.name.hash();
  let sink = {
    let symbols = p.symbols();
    let symbols_brrw = symbols.borrow();
    match symbols_brrw.get_mutable(name) {
      Some(val) => val.borrow().clone(),
      None => {
        if !symbols_brrw.contains(name) {
          return Err(MechError{file: file!().to_string(), tokens: slc.name.tokens(), msg: "Note: Variables are defined with the := operator.".to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});
        } else { 
          return Err(MechError{file: file!().to_string(), tokens: slc.name.tokens(), msg: "Note: Variables are defined with the := operator.".to_string(), id: line!(), kind: MechErrorKind::NotMutable(name)});
        }
      }
    }
  };
  match &slc.subscript {
    Some(sbscrpt) => {
      for s in sbscrpt {
        let s_result = subscript_ref(&s, &sink, &source, p)?;
        return Ok(s_result);
      }
    }
    None => {
      let args = vec![sink,source];
      let fxn = AssignValue{}.compile(&args)?;
      fxn.solve();
      let res = fxn.out();
      p.state.borrow_mut().add_plan_step(fxn);
      return Ok(res);
    }
  }
  unreachable!(); // subscript should have thrown an error if we can't access an element
}

#[cfg(feature = "enum")]
pub fn enum_define(enm_def: &EnumDefine, p: &Interpreter) -> MResult<Value> {
  let id = enm_def.name.hash();
  let variants = enm_def.variants.iter().map(|v| (v.name.hash(),None)).collect::<Vec<(u64, Option<Value>)>>();
  let state = &p.state;
  let mut state_brrw = state.borrow_mut();
  let enm = MechEnum{id, variants};
  let val = Value::Enum(Box::new(enm.clone()));
  state_brrw.enums.insert(id, enm.clone());
  state_brrw.kinds.insert(id, val.kind());
  Ok(val)
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
#[derive(Debug, Clone)]
pub struct VariableDefineFxn {
  id: u64,
  name: String,
  mutable: bool,
  var: Ref<Value>,
}
#[cfg(feature = "variable_define")]
impl MechFunctionImpl for VariableDefineFxn {
  fn solve(&self) {}
  fn out(&self) -> Value { self.var.borrow().clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "variable_define", feature = "compiler"))]
impl MechFunctionCompiler for VariableDefineFxn {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    // Define the variable in the symbol table
    let addr = self.var.addr();
    let reg = ctx.alloc_register_for_ptr(addr);
    ctx.define_symbol(addr, reg, self.name.as_str());
    // Load the variable's constant value into constant blob
    let val_brrw = self.var.borrow();
    let const_id = val_brrw.compile_const(ctx)?;
    // Load constant into register
    ctx.emit_const_load(reg, const_id);
    // Set features
    ctx.features.insert(FeatureFlag::Builtin(val_brrw.kind().to_feature_kind()));
    Ok(reg)
  }
}

#[cfg(feature = "variable_define")]
pub fn variable_define(var_def: &VariableDefine, p: &Interpreter) -> MResult<Value> {
  let var_id = var_def.var.name.hash();
  let var_name = var_def.var.name.to_string();
  {
    let symbols = p.symbols();
    if symbols.borrow().contains(var_id) {
      return Err(MechError{file: file!().to_string(), tokens: var_def.var.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::VariableRedefined(var_id)}); 
    }
  }
  let mut result = expression(&var_def.expression, p)?;
  if let Some(knd_anntn) =  &var_def.var.kind {
    let knd = kind_annotation(&knd_anntn.kind,p)?;
    let mut state_brrw = &mut p.state.borrow_mut();
    let target_knd = knd.to_value_kind(&mut state_brrw.kinds)?;
    // Do kind checking
    match (&result, &target_knd) {
      // Atom is a variant of an enum
      #[cfg(all(feature = "atom", feature = "enum"))]
      (Value::Atom(given_variant_id), ValueKind::Enum(enum_id)) => {
        let enums = &state_brrw.enums;
        let my_enum = match enums.get(enum_id) {
          Some(my_enum) => my_enum,
          None => todo!(),
        };
        // Given atom isn't a variant of the enum
        if !my_enum.variants.iter().any(|(enum_variant, inner_value)| *given_variant_id == *enum_variant) {
          return Err(MechError{file: file!().to_string(), tokens: var_def.expression.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UnknownEnumVairant(*enum_id,*given_variant_id)}); 
        }
      }
      // Atoms can't convert into anything else.
      #[cfg(feature = "atom")]
      (Value::Atom(given_variant_id), target_kind) => {
        return Err(MechError{file: file!().to_string(), tokens: var_def.expression.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UnableToConvertValueKind}); 
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
    let var_def_fxn = VariableDefineFxn{id: var_id, name: var_name.clone(), mutable: var_def.mutable, var: val_ref.clone()};
    state_brrw.add_plan_step(Box::new(var_def_fxn.clone()));
    return Ok(result);
  } else { 
    let mut state_brrw = p.state.borrow_mut();
    // Save symbol to interpreter
    let val_ref = state_brrw.save_symbol(var_id,var_name.clone(),result.clone(),var_def.mutable);
    // Add variable define step to plan
    let var_def_fxn = VariableDefineFxn{id: var_id, name: var_name.clone(), mutable: var_def.mutable, var: val_ref.clone()};
    state_brrw.add_plan_step(Box::new(var_def_fxn.clone()));
    return Ok(result);
  }
}

macro_rules! op_assign {
  ($fxn_name:ident, $op:tt) => {
    paste!{
      pub fn $fxn_name(sbscrpt: &Subscript, sink: &Value, source: &Value, p: &Interpreter) -> MResult<Value> {
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
                let ixes = subscript_formula(&subs[0], p)?;
                let shape = ixes.shape();
                fxn_input.push(ixes);
                match shape[..] {
                  //[1,1] => plan.borrow_mut().push(MatrixSetScalar{}.compile(&fxn_input)?),
                  [1,n] => plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?),
                  [n,1] => plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?),
                  _ => todo!(),
                }
              },
              [Subscript::Formula(ix1),Subscript::All] => {
                fxn_input.push(source.clone());
                let ix = subscript_formula(&subs[0], p)?;
                let shape = ix.shape();
                fxn_input.push(ix);
                fxn_input.push(Value::IndexAll);
                match shape[..] {
                  //[1,1] => plan.borrow_mut().push(MatrixSetScalarAll{}.compile(&fxn_input)?),
                  [1,n] => plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?),
                  [n,1] => plan.borrow_mut().push([<$op AssignRangeAll>]{}.compile(&fxn_input)?),
                  _ => todo!(),
                }
              },
              [Subscript::Range(ix)] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], p)?;
                fxn_input.push(ixes);
                plan.borrow_mut().push([<$op AssignRange>]{}.compile(&fxn_input)?);
              },
              [Subscript::Range(ix), Subscript::All] => {
                fxn_input.push(source.clone());
                let ixes = subscript_range(&subs[0], p)?;
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
//#[cfg(feature = "math_exp")]
//op_assign!(exp_assign, Exp);

#[cfg(feature = "subscript")]
pub fn subscript_ref(sbscrpt: &Subscript, sink: &Value, source: &Value, p: &Interpreter) -> MResult<Value> {
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
        [Subscript::Formula(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_formula(&subs[0], p)?;
          let shape = ixes.shape();
          fxn_input.push(ixes);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixSetScalar{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [1,n] => plan.borrow_mut().push(MatrixSetRange{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [n,1] => plan.borrow_mut().push(MatrixSetRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix)] => {
          fxn_input.push(source.clone());
          let ixes = subscript_range(&subs[0], p)?;
          fxn_input.push(ixes);
          plan.borrow_mut().push(MatrixSetRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::All] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixSetAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_formula(&subs[0], p)?;
          let shape1 = result.shape();
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], p)?;
          let shape2 = result.shape();
          fxn_input.push(result);
          match ((shape1[0],shape1[1]),(shape2[0],shape2[1])) {
            #[cfg(feature = "matrix")]
            ((1,1),(1,1)) => plan.borrow_mut().push(MatrixSetScalarScalar{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((1,1),(m,1)) => plan.borrow_mut().push(MatrixSetScalarRange{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((n,1),(1,1)) => plan.borrow_mut().push(MatrixSetRangeScalar{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            ((n,1),(m,1)) => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            _ => unreachable!(),
          }          
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1],p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let ix = subscript_formula(&subs[1], p)?;
          let shape = ix.shape();
          fxn_input.push(ix);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixSetAllScalar{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [1,n] => plan.borrow_mut().push(MatrixSetAllRange{}.compile(&fxn_input)?),
            #[cfg(all(feature = "matrix", feature = "subscript_range"))]
            [n,1] => plan.borrow_mut().push(MatrixSetAllRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        }
        #[cfg(feature = "subscript_range")]
        [Subscript::Formula(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let ix = subscript_formula(&subs[0], p)?;
          let shape = ix.shape();
          fxn_input.push(ix);
          fxn_input.push(Value::IndexAll);
          match shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixSetScalarAll{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixSetRangeAll{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixSetRangeAll{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixSetRangeScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          let result = subscript_formula(&subs[0], p)?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1],p)?;
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixSetScalarRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixSetRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::All,Subscript::Range(ix2)] => {
          fxn_input.push(source.clone());
          fxn_input.push(Value::IndexAll);
          let result = subscript_range(&subs[1],p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixSetAllRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "matrix", feature = "subscript_range"))]
        [Subscript::Range(ix1),Subscript::All] => {
          fxn_input.push(source.clone());
          let result = subscript_range(&subs[0],p)?;
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
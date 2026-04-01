use crate::*;
use std::collections::HashMap;

// Expressions
// ----------------------------------------------------------------------------

pub type Environment = HashMap<u64, Value>;

pub fn expression(expr: &Expression, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match &expr {
    #[cfg(feature = "variables")]
    Expression::Var(v) => var(v, env, p),
    #[cfg(feature = "range")]
    Expression::Range(rng) => range(&rng, env, p),
    #[cfg(all(feature = "subscript_slice", feature = "access"))]
    Expression::Slice(slc) => slice(&slc, env, p),
    #[cfg(feature = "formulas")]
    Expression::Formula(fctr) => factor(fctr, env, p),
    Expression::Structure(strct) => structure(strct, env, p),
    Expression::Literal(ltrl) => literal(&ltrl, p),
    #[cfg(feature = "functions")]
    Expression::FunctionCall(fxn_call) => function_call(fxn_call, env, p),
    #[cfg(feature = "set_comprehensions")]
    Expression::SetComprehension(set_comp) => set_comprehension(set_comp, p),
    #[cfg(feature = "matrix_comprehensions")]
    Expression::MatrixComprehension(matrix_comp) => matrix_comprehension(matrix_comp, p),
    #[cfg(feature = "functions")]
    Expression::FsmPipe(fsm_pipe) => crate::functions::execute_fsm_pipe(fsm_pipe, env, p),
    x => Err(MechError::new(
      FeatureNotEnabledError,
      None
    ).with_compiler_loc().with_tokens(x.tokens())),
  }
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
pub fn pattern_match_value(pattern: &Pattern, value: &Value, env: &mut Environment) -> MResult<()> {
  match pattern {
    Pattern::Wildcard => Ok(()),
    Pattern::Expression(expr) => match expr {
      Expression::Var(var) => {
        let id = &var.name.hash();
        match env.get(id) {
          Some(existing) if existing == value => Ok(()),
          Some(existing) => {
            Err(MechError::new(
                PatternMatchError {
                    var: var.name.to_string(),
                    expected: existing.to_string(),
                    found: value.to_string(),
                },
                None
            ).with_compiler_loc())
          }
          None => {
            env.insert(id.clone(), value.clone());
            Ok(())
          }
        }
      },
      _ => todo!("Unsupported expression in pattern"),
    },
    #[cfg(feature = "tuple")]
    Pattern::Tuple(pat_tuple) => {
      match value {
        Value::Tuple(values) => {
          let values_brrw = values.borrow();
          if pat_tuple.0.len() != values_brrw.elements.len() {
            return Err(MechError::new(
              ArityMismatchError{
                expected: pat_tuple.0.len(),
                found: values_brrw.elements.len(),
              },
              None
            ).with_compiler_loc());
          }
          for (pttrn, val) in pat_tuple.0.iter().zip(values_brrw.elements.iter()) {
            pattern_match_value(pttrn, val, env)?;
          }
          Ok(())
        }
        _ => Err(MechError::new(
          PatternExpectedTupleError{
            found: value.kind(),
          },
          None
        ).with_compiler_loc()),
      }
    },
    Pattern::TupleStruct(pat_struct) => {
      todo!("Implement tuple struct pattern matching")
    },
    _ => {
      Err(MechError::new(
        FeatureNotEnabledError,
        None
      ).with_compiler_loc())
    } 
  }
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_environments(qualifiers: &[ComprehensionQualifier], comprehension_id: u64, p: &Interpreter) -> MResult<(Vec<Environment>, Interpreter)> {
  let mut envs: Vec<Environment> = vec![HashMap::new()];
  let mut new_p = p.clone();
  new_p.id = comprehension_id;
  new_p.clear_plan();
  for qual in qualifiers {
    envs = match qual {
      ComprehensionQualifier::Generator((pttrn, expr)) => {
        let mut new_envs = Vec::new();
        for env in &envs {
          let collection = expression(expr, Some(env), &new_p)?;
          for elmnt in comprehension_generator_values(&collection)? {
            let mut new_env = env.clone();
            if pattern_match_value(pttrn, &elmnt, &mut new_env).is_ok() {
              new_envs.push(new_env);
            }
          }
        }
        new_envs
      }
      ComprehensionQualifier::Filter(expr) => {
        envs
          .into_iter()
          .filter(|env| {
            let result = expression(expr, Some(env), &new_p);
            match result {
              Ok(Value::Bool(v)) => v.borrow().clone(),
              Ok(_) => false,
              Err(_) => false,
            }
          })
          .collect()
      }
      ComprehensionQualifier::Let(var_def) => {
        envs.into_iter()
            .map(|mut env| -> MResult<_> {
                let val = expression(&var_def.expression, Some(&env), &new_p)?;
                env.insert(var_def.var.name.hash(), val);
                Ok(env)
            })
            .collect::<MResult<Vec<_>>>()?
      }
    };
  }
  Ok((envs, new_p))
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn comprehension_generator_values(collection: &Value) -> MResult<Vec<Value>> {
  match collection {
    #[cfg(feature = "set")]
    Value::Set(mset) => Ok(mset.borrow().set.iter().cloned().collect()),
    #[cfg(feature = "matrix")]
    Value::MatrixIndex(matrix) => Ok(matrix.as_vec().into_iter().map(|value| Value::Index(Ref::new(value))).collect()),
    #[cfg(all(feature = "matrix", feature = "bool"))]
    Value::MatrixBool(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u8"))]
    Value::MatrixU8(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u16"))]
    Value::MatrixU16(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u32"))]
    Value::MatrixU32(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u64"))]
    Value::MatrixU64(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "u128"))]
    Value::MatrixU128(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i8"))]
    Value::MatrixI8(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i16"))]
    Value::MatrixI16(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i32"))]
    Value::MatrixI32(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i64"))]
    Value::MatrixI64(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "i128"))]
    Value::MatrixI128(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "f32"))]
    Value::MatrixF32(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "f64"))]
    Value::MatrixF64(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "string"))]
    Value::MatrixString(matrix) => Ok(matrix.as_vec().into_iter().map(Value::from).collect()),
    #[cfg(all(feature = "matrix", feature = "rational"))]
    Value::MatrixR64(matrix) => Ok(matrix.as_vec().into_iter().map(|value| value.to_value()).collect()),
    #[cfg(all(feature = "matrix", feature = "complex"))]
    Value::MatrixC64(matrix) => Ok(matrix.as_vec().into_iter().map(|value| value.to_value()).collect()),
    #[cfg(feature = "matrix")]
    Value::MatrixValue(matrix) => Ok(matrix.as_vec()),
    Value::MutableReference(reference) => comprehension_generator_values(&reference.borrow()),
    x => Err(MechError::new(
      ComprehensionGeneratorError{
        found: x.kind(),
      },
      None
    ).with_compiler_loc()),
  }
}

#[cfg(any(feature = "set_comprehensions", feature = "matrix_comprehensions"))]
fn detach_comprehension_value(value: &Value) -> Value {
  match value {
    Value::MutableReference(reference) => reference.borrow().clone(),
    _ => value.clone(),
  }
}

#[cfg(feature = "set_comprehensions")]
#[derive(Debug)]
pub struct ValueSetComprehension {
  pub arguments: Vec<Value>,
  pub out: Ref<MechSet>,
}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueSetComprehension {
  fn solve(&self) {
    let args = self.arguments.iter().map(detach_comprehension_value).collect::<Vec<Value>>();
    *self.out.borrow_mut() = MechSet::from_vec(args);
  }
  fn out(&self) -> Value { Value::Set(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueSetComprehension {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Nullary(out) => {
        let out: Ref<MechSet> = unsafe{ out.as_unchecked().clone() };
        Ok(Box::new(ValueSetComprehension { arguments: Vec::new(), out }))
      }
      _ => Err(MechError::new(
        IncorrectNumberOfArguments { expected: 0, found: args.len() },
        None
      ).with_compiler_loc()),
    }
  }
}
#[cfg(all(feature = "set_comprehensions", feature = "compiler"))]
impl MechFunctionCompiler for ValueSetComprehension {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    compile_nullop!("set/comprehension", self.out, ctx, FeatureFlag::Builtin(FeatureKind::SetComprehensions));
  }
}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
register_descriptor! {
  FunctionDescriptor {
    name: "set/comprehension",
    ptr: ValueSetComprehension::new,
  }
}
#[cfg(feature = "set_comprehensions")]
pub struct SetComprehensionDefine {}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
impl NativeFunctionCompiler for SetComprehensionDefine {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    Ok(Box::new(ValueSetComprehension {
      arguments: arguments.clone(),
      out: Ref::new(MechSet::from_vec(arguments.clone())),
    }))
  }
}
#[cfg(all(feature = "set_comprehensions", feature = "functions"))]
register_descriptor!{
  FunctionCompilerDescriptor {
    name: "set/comprehension",
    ptr: &SetComprehensionDefine{},
  }
}

#[cfg(feature = "matrix_comprehensions")]
#[derive(Debug)]
pub struct ValueMatrixComprehension {
  pub arguments: Vec<Value>,
  pub out: Ref<Value>,
}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionImpl for ValueMatrixComprehension {
  fn solve(&self) {
    let args = self.arguments.iter().map(detach_comprehension_value).collect::<Vec<Value>>();
    let out = if args.is_empty() {
      Value::MatrixValue(Matrix::from_vec(vec![], 0, 0))
    } else {
      let fxn = MatrixHorzCat{}.compile(&args).expect("matrix/comprehension input kinds changed to incompatible values");
      fxn.solve();
      fxn.out()
    };
    *self.out.borrow_mut() = out;
  }
  fn out(&self) -> Value { self.out.borrow().clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl MechFunctionFactory for ValueMatrixComprehension {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Nullary(out) => Ok(Box::new(ValueMatrixComprehension { arguments: Vec::new(), out: Ref::new(out) })),
      _ => Err(MechError::new(
        IncorrectNumberOfArguments { expected: 0, found: args.len() },
        None
      ).with_compiler_loc()),
    }
  }
}
#[cfg(all(feature = "matrix_comprehensions", feature = "compiler"))]
impl MechFunctionCompiler for ValueMatrixComprehension {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    compile_nullop!("matrix/comprehension", self.out, ctx, FeatureFlag::Builtin(FeatureKind::MatrixComprehensions));
  }
}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
register_descriptor! {
  FunctionDescriptor {
    name: "matrix/comprehension",
    ptr: ValueMatrixComprehension::new,
  }
}
#[cfg(feature = "matrix_comprehensions")]
pub struct MatrixComprehensionDefine {}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
impl NativeFunctionCompiler for MatrixComprehensionDefine {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    let out = if arguments.is_empty() {
      Value::MatrixValue(Matrix::from_vec(vec![], 0, 0))
    } else {
      let fxn = MatrixHorzCat {}.compile(arguments)?;
      fxn.solve();
      fxn.out()
    };
    Ok(Box::new(ValueMatrixComprehension { arguments: arguments.clone(), out: Ref::new(out) }))
  }
}
#[cfg(all(feature = "matrix_comprehensions", feature = "functions"))]
register_descriptor!{
  FunctionCompilerDescriptor {
    name: "matrix/comprehension",
    ptr: &MatrixComprehensionDefine{},
  }
}

#[cfg(feature = "set_comprehensions")]
pub fn set_comprehension(set_comp: &SetComprehension,p: &Interpreter) -> MResult<Value> {
  let comprehension_id = hash_str(&format!("{:?}", set_comp));
  let (envs, new_p) = comprehension_environments(&set_comp.qualifiers, comprehension_id, p)?;
  let mut values = Vec::new();
  for env in envs {
    let val = expression(&set_comp.expression, Some(&env), &new_p)?;
    values.push(val);
  }
  let functions = p.functions();
  let set_define_id = hash_str("set/comprehension");
  let set_define = {
    functions
      .borrow()
      .function_compilers
      .get(&set_define_id)
      .copied()
  };
  match set_define {
    Some(compiler) => execute_native_function_compiler(compiler, &values, p),
    None => Err(MechError::new(
      MissingFunctionError { function_id: set_define_id },
      None,
    ).with_compiler_loc()),
  }
}

#[cfg(feature = "matrix_comprehensions")]
pub fn matrix_comprehension(matrix_comp: &MatrixComprehension,p: &Interpreter) -> MResult<Value> {
  let comprehension_id = hash_str(&format!("{:?}", matrix_comp));
  let (envs, new_p) = comprehension_environments(&matrix_comp.qualifiers, comprehension_id, p)?;
  let mut values = Vec::new();
  for env in envs {
    values.push(expression(&matrix_comp.expression, Some(&env), &new_p)?);
  }
  let functions = p.functions();
  let horzcat_id = hash_str("matrix/comprehension");
  let horzcat = {
    functions
      .borrow()
      .function_compilers
      .get(&horzcat_id)
      .copied()
  };
  match horzcat {
    Some(compiler) => execute_native_function_compiler(compiler, &values, p),
    None => Err(MechError::new(
      MissingFunctionError { function_id: horzcat_id },
      None,
    ).with_compiler_loc()),
  }
}


#[cfg(feature = "range")]
pub fn range(rng: &RangeExpression, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let start = factor(&rng.start, env, p)?;
  let terminal = factor(&rng.terminal, env, p)?;
  let new_fxn = match &rng.increment {
    Some((_,inc)) => {
      let step = factor(inc, env, p)?;
      match &rng.operator {
        #[cfg(feature = "range_exclusive")]
        RangeOp::Exclusive => RangeIncrementExclusive{}.compile(&vec![start, step, terminal])?,
        #[cfg(feature = "range_inclusive")]
        RangeOp::Inclusive => RangeIncrementInclusive{}.compile(&vec![start, step, terminal])?,
        x => unreachable!(),
      }
    }
    None => {
      match &rng.operator {
        #[cfg(feature = "range_exclusive")]
        RangeOp::Exclusive => RangeExclusive{}.compile(&vec![start,terminal])?,
        #[cfg(feature = "range_inclusive")]
        RangeOp::Inclusive => RangeInclusive{}.compile(&vec![start,terminal])?,
        x => unreachable!(),
      }
    }
  };
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.push(new_fxn);
  let step = plan_brrw.last().unwrap();
  step.solve();
  let res = step.out();
  Ok(res)
}

#[cfg(all(feature = "subscript_slice", feature = "access"))]
pub fn slice(slc: &Slice, env: Option<&Environment>,p: &Interpreter) -> MResult<Value> {
  let id = slc.name.hash();
  let val: Value = if let Some(env) = env {
    if let Some(val) = env.get(&id) {
      val.clone()
    } else {
      // fallback to global symbols
      match p.symbols().borrow().get(id) {
        Some(val) => Value::MutableReference(val.clone()),
        None => {
          return Err(MechError::new(
            UndefinedVariableError { id },
            None,
          )
          .with_compiler_loc()
          .with_tokens(slc.tokens()))
        }
      }
    }
  } else {
    match p.symbols().borrow().get(id) {
      Some(val) => Value::MutableReference(val.clone()),
      None => {
        return Err(MechError::new(
          UndefinedVariableError { id },
          None,
        )
        .with_compiler_loc()
        .with_tokens(slc.tokens()))
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
pub fn subscript_formula(sbscrpt: &Subscript, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match sbscrpt {
    Subscript::Formula(fctr) => {
      factor(fctr, env, p)
    }
    _ => unreachable!()
  }
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula_ix(sbscrpt: &Subscript, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match sbscrpt {
    Subscript::Formula(fctr) => {
      let result = factor(fctr, env, p)?;
      result.as_index()
    }
    _ => unreachable!()
  }
}

#[cfg(feature = "subscript_range")]
pub fn subscript_range(sbscrpt: &Subscript, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match sbscrpt {
    Subscript::Range(rng) => {
      let result = range(rng, env, p)?;
      match result.as_vecusize() {
        Ok(v) => Ok(v.to_value()),
        Err(_) => Err(MechError::new(
            InvalidIndexKindError { kind: result.kind() },
            None
          ).with_compiler_loc().with_tokens(rng.tokens())
        ),
      }
    }
    _ => unreachable!()
  }
}

#[cfg(all(feature = "subscript", feature = "access"))]
pub fn subscript(sbscrpt: &Subscript, val: &Value, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  match sbscrpt {
    #[cfg(feature = "table")]
    Subscript::Dot(x) => {
      let key = x.hash();
      let fxn_input: Vec<Value> = vec![val.clone(), Value::Id(key)];
      let new_fxn = AccessColumn{}.compile(&fxn_input)?;
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    Subscript::DotInt(x) => {
      let mut fxn_input = vec![val.clone()];
      let result = real(&x.clone(), p)?;
      fxn_input.push(result.as_index()?);
      match val.deref_kind() {
        #[cfg(feature = "matrix")]
        ValueKind::Matrix(..) => {
          let new_fxn = MatrixAccessScalar{}.compile(&fxn_input)?;
          new_fxn.solve();
          let res = new_fxn.out();
          plan.borrow_mut().push(new_fxn);
          return Ok(res);
        },
        #[cfg(feature = "tuple")]
        ValueKind::Tuple(..) => {
          let new_fxn = TupleAccess{}.compile(&fxn_input)?;
          new_fxn.solve();
          let res = new_fxn.out();
          plan.borrow_mut().push(new_fxn);
          return Ok(res);
        },
        /*ValueKind::Record(_) => {
          let new_fxn = RecordAccessScalar{}.compile(&fxn_input)?;
          new_fxn.solve();
          let res = new_fxn.out();
          plan.borrow_mut().push(new_fxn);
          return Ok(res);
        },*/
        _ => todo!("Implement access for dot int"),
      }
    },
    #[cfg(feature = "swizzle")]
    Subscript::Swizzle(x) => {
      let mut keys = x.iter().map(|x| Value::Id(x.hash())).collect::<Vec<Value>>();
      let mut fxn_input: Vec<Value> = vec![val.clone()];
      fxn_input.append(&mut keys);
      let new_fxn = AccessSwizzle{}.compile(&fxn_input)?;
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
    Subscript::Brace(subs) => {
      let mut fxn_input = vec![val.clone()];
      match &subs[..] {
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix)] => {
          let result = subscript_formula(&subs[0], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match shape[..] {
            [1, 1] => plan.borrow_mut().push(AccessScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "subscript_range")]
            [n,1] => plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?),
            #[cfg(feature = "subscript_range")]
            [1,n] => plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::Range(ix)] => {
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?);
        },
        /*[Subscript::All] => {
          fxn_input.push(Value::IndexAll);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MapAccessAll{}.compile(&fxn_input)?);
        },*/
        _ => {
          todo!("Implement brace subscript")
        }
      }
      let plan_brrw = plan.borrow();
      let mut new_fxn = &plan_brrw.last().unwrap();
      new_fxn.solve();
      let res = new_fxn.out();
      return Ok(res);
    }
    #[cfg(feature = "subscript_slice")]
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![val.clone()];
      match &subs[..] {
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix)] => {
          let result = subscript_formula_ix(&subs[0], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match shape[..] {
            [1,1] => plan.borrow_mut().push(AccessScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "subscript_range")]
            [1,n] => plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?),
            #[cfg(feature = "subscript_range")]
            [n,1] => plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::Range(ix)] => {
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?);
        },
        [Subscript::All] => {
          fxn_input.push(Value::IndexAll);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MatrixAccessAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_formula_ix(&subs[0], env, p)?;
          let shape1 = result.shape();
          fxn_input.push(result);
          let result = subscript_formula_ix(&subs[1], env, p)?;
          let shape2 = result.shape();
          fxn_input.push(result);
          match ((shape1[0],shape1[1]),(shape2[0],shape2[1])) {
            #[cfg(feature = "matrix")]
            ((1,1),(1,1)) => plan.borrow_mut().push(MatrixAccessScalarScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            ((1,1),(m,1)) => plan.borrow_mut().push(MatrixAccessScalarRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            ((n,1),(1,1)) => plan.borrow_mut().push(MatrixAccessRangeScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            ((n,1),(m,1)) => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => unreachable!(),
          }
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_formula_ix(&subs[1], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAccessAllScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
        [Subscript::Formula(ix1),Subscript::All] => {
          let result = subscript_formula_ix(&subs[0], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAccessScalarAll{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          let result = subscript_formula_ix(&subs[1], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAccessRangeScalar{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          let result = subscript_formula_ix(&subs[0], env, p)?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          match &shape[..] {
            #[cfg(feature = "matrix")]
            [1,1] => plan.borrow_mut().push(MatrixAccessScalarRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            #[cfg(feature = "matrix")]
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::All,Subscript::Range(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_range(&subs[1], env, p)?;
          fxn_input.push(result);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?);
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::Range(ix1),Subscript::All] => {
          let result = subscript_range(&subs[0], env, p)?;
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?);
        },
        _ => unreachable!(),
      };
      let plan_brrw = plan.borrow();
      let mut new_fxn = &plan_brrw.last().unwrap();
      new_fxn.solve();
      let res = new_fxn.out();
      return Ok(res);
    },
    _ => unreachable!(),
  }
}

#[cfg(feature = "symbol_table")]
pub fn var(v: &Var, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let state_brrw = p.state.borrow();
  let symbols_brrw = state_brrw.symbol_table.borrow();
  let id = v.name.hash();
  match env {
    Some(env) => {
      match env.get(&id) {
        Some(value) => {
          return Ok(value.clone())
        }
        None => {
          match symbols_brrw.get(id) {
            Some(value) => {
              return Ok(Value::MutableReference(value.clone()))
            }
            None => {
              return Err(MechError::new(
                  UndefinedVariableError { id },
                  None
                ).with_compiler_loc().with_tokens(v.tokens())
              )
            }
          }
        }
      }
    }
    None => {
      match symbols_brrw.get(id) {
        Some(value) => {
          return Ok(Value::MutableReference(value.clone()))
        }
        None => {
          return Err(MechError::new(
              UndefinedVariableError { id },
              None
            ).with_compiler_loc().with_tokens(v.tokens())
          )
        }
      }
    }
  }
}

#[cfg(feature = "formulas")]
pub fn factor(fctr: &Factor, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, env, p)?;
      Ok(result)
    },
    Factor::Parenthetical(paren) => factor(&*paren, env, p),
    Factor::Expression(expr) => expression(expr, env, p),
    #[cfg(feature = "math_neg")]
    Factor::Negate(neg) => {
      let value = factor(neg, env, p)?;
      let new_fxn = MathNegate{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      p.state.borrow_mut().add_plan_step(new_fxn);
      Ok(out)
    },
    #[cfg(feature = "logic_not")]
    Factor::Not(neg) => {
      let value = factor(neg, env, p)?;
      let new_fxn = LogicNot{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      p.state.borrow_mut().add_plan_step(new_fxn);
      Ok(out)
    },
    #[cfg(feature = "matrix_transpose")]
    Factor::Transpose(fctr) => {
      use mech_matrix::MatrixTranspose;
      let value = factor(fctr, env, p)?;
      let new_fxn = MatrixTranspose{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      p.state.borrow_mut().add_plan_step(new_fxn);
      Ok(out)
    }
    _ => todo!(),
  }
}

#[cfg(feature = "formulas")]
pub fn term(trm: &Term, env: Option<&Environment>, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let mut lhs = factor(&trm.lhs, env, p)?;
  let mut term_plan: Vec<Box<dyn MechFunction>> = vec![];
  for (op,rhs) in &trm.rhs {
    let rhs = factor(&rhs, env, p)?;
    let new_fxn: Box<dyn MechFunction> = match op {
      // Math
      FormulaOperator::AddSub(AddSubOp::Add) => {
        match (&lhs, &rhs) {
          #[cfg(feature = "string_concat")]
          (_, value) | (value, _) if value.is_string() => StringConcat{}.compile(&vec![lhs, rhs])?,
          #[cfg(feature = "math_add")]
          _ => MathAdd{}.compile(&vec![lhs,rhs])?,
        }
      }
      #[cfg(feature = "math_sub")]
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_mul")]
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_div")]
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_mod")]
      FormulaOperator::MulDiv(MulDivOp::Mod) => MathMod{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_pow")]
      FormulaOperator::Power(PowerOp::Pow) => MathPow{}.compile(&vec![lhs,rhs])?,

      // Matrix
      #[cfg(feature = "matrix_matmul")]
      FormulaOperator::Vec(VecOp::MatMul) => {
        use mech_matrix::MatrixMatMul;
        MatrixMatMul{}.compile(&vec![lhs,rhs])?
      }
      #[cfg(feature = "matrix_solve")]
      FormulaOperator::Vec(VecOp::Solve) => MatrixSolve{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "matrix_cross")]
      FormulaOperator::Vec(VecOp::Cross) => todo!(),
      #[cfg(feature = "matrix_dot")]
      FormulaOperator::Vec(VecOp::Dot) => MatrixDot{}.compile(&vec![lhs,rhs])?,

      // Compare
      #[cfg(feature = "compare_eq")]
      FormulaOperator::Comparison(ComparisonOp::Equal) => CompareEqual{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "compare_seq")]
      FormulaOperator::Comparison(ComparisonOp::StrictEqual) => todo!(), //CompareStrictEqual{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "compare_neq")]
      FormulaOperator::Comparison(ComparisonOp::NotEqual) => CompareNotEqual{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "compare_sneq")]
      FormulaOperator::Comparison(ComparisonOp::StrictNotEqual) => todo!(), //CompareStrictNotEqual{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "compare_lte")]
      FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => CompareLessThanEqual{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "compare_gte")]
      FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => CompareGreaterThanEqual{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "compare_lt")]
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "compare_gt")]
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan{}.compile(&vec![lhs,rhs])?,

      // Logic
      #[cfg(feature = "logic_and")]
      FormulaOperator::Logic(LogicOp::And) => LogicAnd{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "logic_or")]
      FormulaOperator::Logic(LogicOp::Or)  => LogicOr{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "logic_not")]
      FormulaOperator::Logic(LogicOp::Not) => LogicNot{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "logic_xor")]
      FormulaOperator::Logic(LogicOp::Xor) => LogicXor{}.compile(&vec![lhs,rhs])?,
      
      // Table
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::InnerJoin) => TableInnerJoin{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftOuterJoin) => TableLeftOuterJoin{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::RightOuterJoin) => TableRightOuterJoin{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::FullOuterJoin) => TableFullOuterJoin{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftSemiJoin) => TableLeftSemiJoin{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftAntiJoin) => TableLeftAntiJoin{}.compile(&vec![lhs,rhs])?,

      // Set
      #[cfg(feature = "set_union")]
      FormulaOperator::Set(SetOp::Union) => SetUnion{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_intersection")]
      FormulaOperator::Set(SetOp::Intersection) => SetIntersection{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_difference")]
      FormulaOperator::Set(SetOp::Difference) => SetDifference{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_symmetric_difference")]
      FormulaOperator::Set(SetOp::SymmetricDifference) => SetSymmetricDifference{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_complement")]
      FormulaOperator::Set(SetOp::Complement) => todo!(),
      #[cfg(feature = "set_subset")]
      FormulaOperator::Set(SetOp::Subset) => SetSubset{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_superset")]
      FormulaOperator::Set(SetOp::Superset) => SetSuperset{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_proper_subset")]
      FormulaOperator::Set(SetOp::ProperSubset) => SetProperSubset{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_proper_superset")]
      FormulaOperator::Set(SetOp::ProperSuperset) => SetProperSuperset{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_element_of")]
      FormulaOperator::Set(SetOp::ElementOf) => SetElementOf{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "set_not_element_of")]
      FormulaOperator::Set(SetOp::NotElementOf) => SetNotElementOf{}.compile(&vec![lhs,rhs])?,
      x => return Err(MechError::new(
          UnhandledFormulaOperatorError { operator: x.clone() },
          None
        ).with_compiler_loc().with_tokens(trm.tokens())
      ),
    };
    new_fxn.solve();
    let res = new_fxn.out();
    term_plan.push(new_fxn);
    lhs = res;
  }
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.append(&mut term_plan);
  return Ok(lhs);
}

#[derive(Debug, Clone)]
pub struct UnhandledFormulaOperatorError {
  pub operator: FormulaOperator,
}
impl MechErrorKind for UnhandledFormulaOperatorError {
  fn name(&self) -> &str { "UnhandledFormulaOperator" }
  fn message(&self) -> String {
    format!("Unhandled formula operator: {:#?}", self.operator)
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
  pub id: u64, 
}
impl MechErrorKind for UndefinedVariableError {
  fn name(&self) -> &str { "UndefinedVariable" }

  fn message(&self) -> String {
    format!("Undefined variable: {}", self.id)
  }
}
#[derive(Debug, Clone)]
pub struct InvalidIndexKindError {
  kind: ValueKind,
}
impl MechErrorKind for InvalidIndexKindError {
  fn name(&self) -> &str {
    "InvalidIndexKind"
  }
  fn message(&self) -> String {
    "Invalid index kind".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct ComprehensionGeneratorError{
  found: ValueKind,
}

impl MechErrorKind for ComprehensionGeneratorError {
  fn name(&self) -> &str {
    "ComprehensionGenerator"
  }
  fn message(&self) -> String {
    format!("Comprehension generator must produce a set or matrix, found kind: {:?}", self.found)
  }
}

#[derive(Debug, Clone)]
pub struct PatternExpectedTupleError{
  found: ValueKind,
}
impl MechErrorKind for PatternExpectedTupleError {
  fn name(&self) -> &str {
    "PatternExpectedTuple"
  }
  fn message(&self) -> String {
    format!("Pattern expected a tuple, found kind: {:?}", self.found)
  }
}

#[derive(Debug, Clone)]
pub struct ArityMismatchError {
  expected: usize,
  found: usize,
}
impl MechErrorKind for ArityMismatchError {
  fn name(&self) -> &str {
    "ArityMismatch"
  }
  fn message(&self) -> String {
    format!("Arity mismatch: expected {}, found {}", self.expected, self.found)
  }
}

#[derive(Debug, Clone)]
pub struct PatternMatchError {
  pub var: String,
  pub expected: String,
  pub found: String,
}

impl MechErrorKind for PatternMatchError {
  fn name(&self) -> &str {
    "PatternMatchError"
  }
  fn message(&self) -> String {
    format!(
      "Pattern match error for variable '{}': expected value {}, found value {}",
      self.var, self.expected, self.found
    )
  }
}

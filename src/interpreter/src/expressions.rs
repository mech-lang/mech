use crate::*;

// Expressions
// ----------------------------------------------------------------------------

pub fn expression(expr: &Expression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match &expr {
    Expression::Var(v) => var(&v, symbols.clone()),
    Expression::Range(rng) => range(&rng, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Slice(slc) => slice(&slc, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Formula(fctr) => factor(fctr, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Structure(strct) => structure(strct, plan.clone(), symbols.clone(), functions.clone()),
    Expression::Literal(ltrl) => literal(&ltrl, functions.clone()),
    Expression::FunctionCall(fxn_call) => function_call(fxn_call, plan.clone(), symbols.clone(), functions.clone()),
    Expression::FsmPipe(_) => todo!(),
  }
}

pub fn range(rng: &RangeExpression, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let start = factor(&rng.start, plan.clone(),symbols.clone(), functions.clone())?;
  let terminal = factor(&rng.terminal, plan.clone(),symbols.clone(), functions.clone())?;
  let new_fxn = match &rng.operator {
    RangeOp::Exclusive => RangeExclusive{}.compile(&vec![start,terminal])?,
    RangeOp::Inclusive => RangeInclusive{}.compile(&vec![start,terminal])?,
    x => unreachable!(),
  };
  let mut plan_brrw = plan.borrow_mut();
  plan_brrw.push(new_fxn);
  let step = plan_brrw.last().unwrap();
  step.solve();
  let res = step.out();
  Ok(res)
}

pub fn slice(slc: &Slice, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let name = slc.name.hash();
  let symbols_brrw = symbols.borrow();
  let val: Value = match symbols_brrw.get(name) {
    Some(val) => Value::MutableReference(val.clone()),
    None => {return Err(MechError{file: file!().to_string(), tokens: slc.name.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});}
  };
  for s in &slc.subscript {
    let s_result = subscript(&s, &val, plan.clone(), symbols.clone(), functions.clone())?;
    return Ok(s_result);
  }
  unreachable!() // subscript should have thrown an error if we can't access an element
}

pub fn subscript_formula(sbscrpt: &Subscript, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Formula(fctr) => {
      let result = factor(fctr,plan.clone(), symbols.clone(), functions.clone())?;
      result.as_index()
    }
    _ => unreachable!()
  }
}

pub fn subscript_range(sbscrpt: &Subscript, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
    Subscript::Range(rng) => {
      let result = range(rng,plan.clone(), symbols.clone(), functions.clone())?;
      match result.as_vecusize() {
        Some(v) => Ok(v.to_value()),
        None => Err(MechError{file: file!().to_string(), tokens: rng.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledIndexKind}),
      }
    }
    _ => unreachable!()
  }
}

pub fn subscript(sbscrpt: &Subscript, val: &Value, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match sbscrpt {
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
      let result = real(&x.clone());
      fxn_input.push(result.as_index()?);
      let new_fxn = MatrixAccessScalar{}.compile(&fxn_input)?; // This presumes the thing is a matrix...
      new_fxn.solve();
      let res = new_fxn.out();
      plan.borrow_mut().push(new_fxn);
      return Ok(res);
    },
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
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![val.clone()];
      match &subs[..] {
        [Subscript::Formula(ix)] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          match shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix)] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessRange{}.compile(&fxn_input)?);
        },
        [Subscript::All] => {
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAccessAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape1 = result.shape();
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape2 = result.shape();
          fxn_input.push(result);
          match ((shape1[0],shape1[1]),(shape2[0],shape2[1])) {
            ((1,1),(1,1)) => plan.borrow_mut().push(MatrixAccessScalarScalar{}.compile(&fxn_input)?),
            ((1,1),(m,1)) => plan.borrow_mut().push(MatrixAccessScalarRange{}.compile(&fxn_input)?),
            ((n,1),(1,1)) => plan.borrow_mut().push(MatrixAccessRangeScalar{}.compile(&fxn_input)?),
            ((n,1),(m,1)) => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => unreachable!(),
          }
        },
        [Subscript::Range(ix1),Subscript::Range(ix2)] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessAllScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Formula(ix1),Subscript::All] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessScalarAll{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeAll{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessRangeScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Formula(ix1),Subscript::Range(ix2)] => {
          let result = subscript_formula(&subs[0], plan.clone(), symbols.clone(), functions.clone())?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          match &shape[..] {
            [1,1] => plan.borrow_mut().push(MatrixAccessScalarRange{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::All,Subscript::Range(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_range(&subs[1],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?);
        },
        [Subscript::Range(ix1),Subscript::All] => {
          let result = subscript_range(&subs[0],plan.clone(), symbols.clone(), functions.clone())?;
          fxn_input.push(result);
          fxn_input.push(Value::IndexAll);
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
    Subscript::Brace(x) => todo!(),
    _ => unreachable!(),
  }
}

pub fn var(v: &Var, symbols: SymbolTableRef) -> MResult<Value> {
  let id = v.name.hash();
  let symbols_brrw = symbols.borrow();
  match symbols_brrw.get(id) {
    Some(value) => {
      return Ok(Value::MutableReference(value.clone()))
    }
    None => {
      return Err(MechError{file: file!().to_string(), tokens: v.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(id)});
    }
  }
}

pub fn factor(fctr: &Factor, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, plan.clone(), symbols.clone(), functions.clone())?;
      Ok(result)
    },
    Factor::Parenthetical(paren) => factor(&*paren, plan.clone(), symbols.clone(), functions.clone()),
    Factor::Expression(expr) => expression(expr, plan.clone(), symbols.clone(), functions.clone()),
    Factor::Negate(neg) => {
      let value = factor(neg, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = MathNegate{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
    Factor::Not(neg) => {
      let value = factor(neg, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = LogicNot{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
    Factor::Transpose(fctr) => {
      let value = factor(fctr, plan.clone(), symbols.clone(), functions.clone())?;
      let new_fxn = MatrixTranspose{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    }
  }
}

pub fn term(trm: &Term, plan: Plan, symbols: SymbolTableRef, functions: FunctionsRef) -> MResult<Value> {
  let mut lhs = factor(&trm.lhs, plan.clone(), symbols.clone(), functions.clone())?;
  let mut term_plan: Vec<Box<dyn MechFunction>> = vec![];
  for (op,rhs) in &trm.rhs {
    let rhs = factor(&rhs, plan.clone(), symbols.clone(), functions.clone())?;
    let new_fxn = match op {
      FormulaOperator::AddSub(AddSubOp::Add) => MathAdd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Exponent(ExponentOp::Exp) => MathExp{}.compile(&vec![lhs,rhs])?,

      FormulaOperator::Vec(VecOp::MatMul) => MatrixMatMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Vec(VecOp::Solve) => todo!(),
      FormulaOperator::Vec(VecOp::Cross) => todo!(),
      FormulaOperator::Vec(VecOp::Dot) => todo!(),

      FormulaOperator::Comparison(ComparisonOp::Equal) => CompareEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::NotEqual) => CompareNotEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => CompareLessThanEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => CompareGreaterThanEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan{}.compile(&vec![lhs,rhs])?,

      FormulaOperator::Logic(LogicOp::And) => LogicAnd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Or)  => LogicOr{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Not) => LogicNot{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Xor) => LogicXor{}.compile(&vec![lhs,rhs])?,
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
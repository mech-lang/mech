use crate::*;

// Expressions
// ----------------------------------------------------------------------------

pub fn expression(expr: &Expression, p: &Interpreter) -> MResult<Value> {
  match &expr {
    Expression::Var(v) => var(&v, p),
    Expression::Range(rng) => range(&rng, p),
    Expression::Slice(slc) => slice(&slc, p),
    Expression::Formula(fctr) => factor(fctr, p),
    Expression::Structure(strct) => structure(strct, p),
    Expression::Literal(ltrl) => literal(&ltrl, p),
    Expression::FunctionCall(fxn_call) => function_call(fxn_call, p),
    Expression::FsmPipe(_) => todo!(),
  }
}

pub fn range(rng: &RangeExpression, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let start = factor(&rng.start, p)?;
  let terminal = factor(&rng.terminal, p)?;
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

pub fn slice(slc: &Slice, p: &Interpreter) -> MResult<Value> {
  let symbols = p.symbols();
  let plan = p.plan();
  let functions = p.functions();
  let name = slc.name.hash();
  let symbols_brrw = symbols.borrow();
  let val: Value = match symbols_brrw.get(name) {
    Some(val) => Value::MutableReference(val.clone()),
    None => {return Err(MechError{file: file!().to_string(), tokens: slc.name.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedVariable(name)});}
  };
  let mut v = val;
  for s in &slc.subscript {
    let s_result = subscript(&s, &v, p)?;
    v = s_result;
  }
  return Ok(v);
}

pub fn subscript_formula(sbscrpt: &Subscript, p: &Interpreter) -> MResult<Value> {
  match sbscrpt {
    Subscript::Formula(fctr) => {
      let result = factor(fctr,p)?;
      result.as_index()
    }
    _ => unreachable!()
  }
}

pub fn subscript_range(sbscrpt: &Subscript, p: &Interpreter) -> MResult<Value> {
  match sbscrpt {
    Subscript::Range(rng) => {
      let result = range(rng,p)?;
      match result.as_vecusize() {
        Some(v) => Ok(v.to_value()),
        None => Err(MechError{file: file!().to_string(), tokens: rng.tokens(), msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledIndexKind}),
      }
    }
    _ => unreachable!()
  }
}

pub fn subscript(sbscrpt: &Subscript, val: &Value, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
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
      match val.deref_kind() {
        ValueKind::Matrix(..) => {
          let new_fxn = MatrixAccessScalar{}.compile(&fxn_input)?;
          new_fxn.solve();
          let res = new_fxn.out();
          plan.borrow_mut().push(new_fxn);
          return Ok(res);
        },
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
    Subscript::Brace(subs) |
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![val.clone()];
      match &subs[..] {
        [Subscript::Formula(ix)] => {
          let result = subscript_formula(&subs[0], p)?;
          let shape = result.shape();
          fxn_input.push(result);
          match shape[..] {
            [1,1] => plan.borrow_mut().push(AccessScalar{}.compile(&fxn_input)?),
            [1,n] => plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?),
            [n,1] => plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?),
            _ => todo!(),
          }
        },
        [Subscript::Range(ix)] => {
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?);
        },
        [Subscript::All] => {
          fxn_input.push(Value::IndexAll);
          plan.borrow_mut().push(MatrixAccessAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_formula(&subs[0], p)?;
          let shape1 = result.shape();
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], p)?;
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
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1],p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_formula(&subs[1], p)?;
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
          let result = subscript_formula(&subs[0], p)?;
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
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], p)?;
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
          let result = subscript_formula(&subs[0], p)?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1],p)?;
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
          let result = subscript_range(&subs[1],p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?);
        },
        [Subscript::Range(ix1),Subscript::All] => {
          let result = subscript_range(&subs[0],p)?;
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
    _ => unreachable!(),
  }
}

pub fn var(v: &Var, p: &Interpreter) -> MResult<Value> {
  let symbols = p.symbols();
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

pub fn factor(fctr: &Factor, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, p)?;
      Ok(result)
    },
    Factor::Parenthetical(paren) => factor(&*paren, p),
    Factor::Expression(expr) => expression(expr, p),
    Factor::Negate(neg) => {
      let value = factor(neg, p)?;
      let new_fxn = MathNegate{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
    Factor::Not(neg) => {
      let value = factor(neg, p)?;
      let new_fxn = LogicNot{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    },
    Factor::Transpose(fctr) => {
      let value = factor(fctr, p)?;
      let new_fxn = MatrixTranspose{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      let mut plan_brrw = plan.borrow_mut();
      plan_brrw.push(new_fxn);
      Ok(out)
    }
  }
}

pub fn term(trm: &Term, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let mut lhs = factor(&trm.lhs, p)?;
  let mut term_plan: Vec<Box<dyn MechFunction>> = vec![];
  for (op,rhs) in &trm.rhs {
    let rhs = factor(&rhs, p)?;
    let new_fxn = match op {
      FormulaOperator::AddSub(AddSubOp::Add) => MathAdd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::MulDiv(MulDivOp::Mod) => MathMod{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Exponent(ExponentOp::Exp) => MathExp{}.compile(&vec![lhs,rhs])?,

      FormulaOperator::Vec(VecOp::MatMul) => MatrixMatMul{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Vec(VecOp::Solve) => todo!(),
      FormulaOperator::Vec(VecOp::Cross) => todo!(),
      FormulaOperator::Vec(VecOp::Dot) => todo!(),

      FormulaOperator::Comparison(ComparisonOp::Equal) => CompareEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::StrictEqual) => todo!(), //CompareStrictEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::NotEqual) => CompareNotEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::StrictNotEqual) => todo!(), //CompareStrictNotEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => CompareLessThanEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => CompareGreaterThanEqual{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan{}.compile(&vec![lhs,rhs])?,

      FormulaOperator::Logic(LogicOp::And) => LogicAnd{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Or)  => LogicOr{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Not) => LogicNot{}.compile(&vec![lhs,rhs])?,
      FormulaOperator::Logic(LogicOp::Xor) => LogicXor{}.compile(&vec![lhs,rhs])?,
      
      FormulaOperator::Table(TableOp::InnerJoin) => todo!(),
      FormulaOperator::Table(TableOp::LeftOuterJoin) => todo!(),
      FormulaOperator::Table(TableOp::RightOuterJoin) => todo!(),
      FormulaOperator::Table(TableOp::FullOuterJoin) => todo!(),
      FormulaOperator::Table(TableOp::LeftSemiJoin) => todo!(),
      FormulaOperator::Table(TableOp::LeftAntiJoin) => todo!(),

      FormulaOperator::Set(SetOp::Union) => todo!(),
      FormulaOperator::Set(SetOp::Intersection) => todo!(),
      FormulaOperator::Set(SetOp::Difference) => todo!(),
      FormulaOperator::Set(SetOp::Complement) => todo!(),
      FormulaOperator::Set(SetOp::Subset) => todo!(),
      FormulaOperator::Set(SetOp::Superset) => todo!(),
      FormulaOperator::Set(SetOp::ProperSubset) => todo!(),
      FormulaOperator::Set(SetOp::ProperSuperset) => todo!(),
      FormulaOperator::Set(SetOp::ElementOf) => todo!(),
      FormulaOperator::Set(SetOp::NotElementOf) => todo!(),
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
use crate::*;

// Expressions
// ----------------------------------------------------------------------------

pub fn expression(expr: &Expression, p: &Interpreter) -> MResult<Value> {
  match &expr {
    #[cfg(feature = "variables")]
    Expression::Var(v) => var(&v, p),
    #[cfg(feature = "range")]
    Expression::Range(rng) => range(&rng, p),
    #[cfg(all(feature = "subscript_slice", feature = "access"))]
    Expression::Slice(slc) => slice(&slc, p),
    #[cfg(feature = "formulas")]
    Expression::Formula(fctr) => factor(fctr, p),
    Expression::Structure(strct) => structure(strct, p),
    Expression::Literal(ltrl) => literal(&ltrl, p),
    #[cfg(feature = "functions")]
    Expression::FunctionCall(fxn_call) => function_call(fxn_call, p),
    //Expression::FsmPipe(_) => todo!(),
    x => Err(MechError2::new(
      FeatureNotEnabledError,
      None
    ).with_compiler_loc().with_tokens(x.tokens())),
  }
}

#[cfg(feature = "range")]
pub fn range(rng: &RangeExpression, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let start = factor(&rng.start, p)?;
  let terminal = factor(&rng.terminal, p)?;
  let new_fxn = match &rng.operator {
    #[cfg(feature = "range_exclusive")]
    RangeOp::Exclusive => RangeExclusive{}.compile(&vec![start,terminal])?,
    #[cfg(feature = "range_inclusive")]
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

#[cfg(all(feature = "subscript_slice", feature = "access"))]
pub fn slice(slc: &Slice, p: &Interpreter) -> MResult<Value> {
  let symbols = p.symbols();
  let plan = p.plan();
  let functions = p.functions();
  let id = slc.name.hash();
  let symbols_brrw = symbols.borrow();
  let val: Value = match symbols_brrw.get(id) {
    Some(val) => Value::MutableReference(val.clone()),
    None => {return Err(MechError2::new(
        UndefinedVariableError { id },
        None
      ).with_compiler_loc().with_tokens(slc.tokens())
    )},
  };
  let mut v = val;
  for s in &slc.subscript {
    let s_result = subscript(&s, &v, p)?;
    v = s_result;
  }
  return Ok(v);
}

#[cfg(feature = "subscript_formula")]
pub fn subscript_formula(sbscrpt: &Subscript, p: &Interpreter) -> MResult<Value> {
  match sbscrpt {
    Subscript::Formula(fctr) => {
      let result = factor(fctr,p)?;
      result.as_index()
    }
    _ => unreachable!()
  }
}

#[cfg(feature = "subscript_range")]
pub fn subscript_range(sbscrpt: &Subscript, p: &Interpreter) -> MResult<Value> {
  match sbscrpt {
    Subscript::Range(rng) => {
      let result = range(rng,p)?;
      match result.as_vecusize() {
        Ok(v) => Ok(v.to_value()),
        Err(_) => Err(MechError2::new(
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
pub fn subscript(sbscrpt: &Subscript, val: &Value, p: &Interpreter) -> MResult<Value> {
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
      let result = real(&x.clone());
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
    #[cfg(feature = "subscript_slice")]
    Subscript::Brace(subs) |
    Subscript::Bracket(subs) => {
      let mut fxn_input = vec![val.clone()];
      match &subs[..] {
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix)] => {
          let result = subscript_formula(&subs[0], p)?;
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
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          plan.borrow_mut().push(AccessRange{}.compile(&fxn_input)?);
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::All] => {
          fxn_input.push(Value::IndexAll);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MatrixAccessAll{}.compile(&fxn_input)?);
        },
        [Subscript::All,Subscript::All] => todo!(),
        #[cfg(feature = "subscript_formula")]
        [Subscript::Formula(ix1),Subscript::Formula(ix2)] => {
          let result = subscript_formula(&subs[0], p)?;
          let shape1 = result.shape();
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], p)?;
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
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          let result = subscript_range(&subs[1],p)?;
          fxn_input.push(result);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MatrixAccessRangeRange{}.compile(&fxn_input)?);
        },
        #[cfg(all(feature = "subscript_range", feature = "subscript_formula"))]
        [Subscript::All,Subscript::Formula(ix2)] => {
          fxn_input.push(Value::IndexAll);
          let result = subscript_formula(&subs[1], p)?;
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
          let result = subscript_formula(&subs[0], p)?;
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
          let result = subscript_range(&subs[0],p)?;
          fxn_input.push(result);
          let result = subscript_formula(&subs[1], p)?;
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
          let result = subscript_formula(&subs[0], p)?;
          let shape = result.shape();
          fxn_input.push(result);
          let result = subscript_range(&subs[1],p)?;
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
          let result = subscript_range(&subs[1],p)?;
          fxn_input.push(result);
          #[cfg(feature = "matrix")]
          plan.borrow_mut().push(MatrixAccessAllRange{}.compile(&fxn_input)?);
        },
        #[cfg(feature = "subscript_range")]
        [Subscript::Range(ix1),Subscript::All] => {
          let result = subscript_range(&subs[0],p)?;
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
pub fn var(v: &Var, p: &Interpreter) -> MResult<Value> {
  let state_brrw = p.state.borrow();
  let symbols_brrw = state_brrw.symbol_table.borrow();
  let id = v.name.hash();
  match symbols_brrw.get(id) {
    Some(value) => {
      return Ok(Value::MutableReference(value.clone()))
    }
    None => {
      return Err(MechError2::new(
          UndefinedVariableError { id },
          None
        ).with_compiler_loc().with_tokens(v.tokens())
      )
    }
  }
}

#[cfg(feature = "formulas")]
pub fn factor(fctr: &Factor, p: &Interpreter) -> MResult<Value> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, p)?;
      Ok(result)
    },
    Factor::Parenthetical(paren) => factor(&*paren, p),
    Factor::Expression(expr) => expression(expr, p),
    #[cfg(feature = "math_neg")]
    Factor::Negate(neg) => {
      let value = factor(neg, p)?;
      let new_fxn = MathNegate{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      p.state.borrow_mut().add_plan_step(new_fxn);
      Ok(out)
    },
    #[cfg(feature = "logic_not")]
    Factor::Not(neg) => {
      let value = factor(neg, p)?;
      let new_fxn = LogicNot{}.compile(&vec![value])?;
      new_fxn.solve();
      let out = new_fxn.out();
      p.state.borrow_mut().add_plan_step(new_fxn);
      Ok(out)
    },
    #[cfg(feature = "matrix_transpose")]
    Factor::Transpose(fctr) => {
      use mech_matrix::MatrixTranspose;
      let value = factor(fctr, p)?;
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
pub fn term(trm: &Term, p: &Interpreter) -> MResult<Value> {
  let plan = p.plan();
  let mut lhs = factor(&trm.lhs, p)?;
  let mut term_plan: Vec<Box<dyn MechFunction>> = vec![];
  for (op,rhs) in &trm.rhs {
    let rhs = factor(&rhs, p)?;
    let new_fxn: Box<dyn MechFunction> = match op {
      // Math
      #[cfg(feature = "math_add")]
      FormulaOperator::AddSub(AddSubOp::Add) => MathAdd{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_sub")]
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_mul")]
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_div")]
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_mod")]
      FormulaOperator::MulDiv(MulDivOp::Mod) => MathMod{}.compile(&vec![lhs,rhs])?,
      #[cfg(feature = "math_exp")]
      FormulaOperator::Exponent(ExponentOp::Exp) => MathExp{}.compile(&vec![lhs,rhs])?,

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
      #[cfg(feature = "table_inner_join")]
      FormulaOperator::Table(TableOp::InnerJoin) => todo!(),
      #[cfg(feature = "table_left_outer_join")]
      FormulaOperator::Table(TableOp::LeftOuterJoin) => todo!(),
      #[cfg(feature = "table_right_outer_join")]
      FormulaOperator::Table(TableOp::RightOuterJoin) => todo!(),
      #[cfg(feature = "table_full_outer_join")]
      FormulaOperator::Table(TableOp::FullOuterJoin) => todo!(),
      #[cfg(feature = "table_left_semi_join")]
      FormulaOperator::Table(TableOp::LeftSemiJoin) => todo!(),
      #[cfg(feature = "table_left_anti_join")]
      FormulaOperator::Table(TableOp::LeftAntiJoin) => todo!(),

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
      x => return Err(MechError2::new(
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
impl MechErrorKind2 for UnhandledFormulaOperatorError {
  fn name(&self) -> &str { "UnhandledFormulaOperator" }
  fn message(&self) -> String {
    format!("Unhandled formula operator: {:#?}", self.operator)
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
pub struct InvalidIndexKindError {
  kind: ValueKind,
}
impl MechErrorKind2 for InvalidIndexKindError {
  fn name(&self) -> &str {
    "InvalidIndexKind"
  }
  fn message(&self) -> String {
    "Invalid index kind".to_string()
  }
}
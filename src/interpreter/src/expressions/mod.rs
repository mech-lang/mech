#![forbid(unsafe_code)]

use crate::*;

use std::collections::HashMap;
#[cfg(feature = "enum")]
use std::collections::HashSet;

mod environment;
mod comprehensions;
mod ranges;
mod registration;
mod subscripts;
mod variables;

#[cfg(feature = "matrix_comprehensions")]
pub use comprehensions::{
  MatrixComprehensionDefine, ValueMatrixComprehension, matrix_comprehension,
};
#[cfg(feature = "set_comprehensions")]
pub use comprehensions::{SetComprehensionDefine, ValueSetComprehension, set_comprehension};
pub(crate) use environment::DeferredExpressionSolveScope;
use environment::expression_solves_deferred;
pub use ranges::range;
use registration::{
  register_expression_function_batch, register_initialized_expression_function,
};
#[cfg(all(feature = "subscript", feature = "access"))]
pub use subscripts::subscript;
#[cfg(feature = "subscript_formula")]
pub use subscripts::{subscript_formula, subscript_formula_ix};
#[cfg(feature = "subscript_range")]
pub use subscripts::subscript_range;
#[cfg(all(feature = "subscript_slice", feature = "access"))]
pub use subscripts::slice;
#[cfg(feature = "subscript_formula")]
pub(crate) use subscripts::{
  current_string_access_expression_live, mark_current_string_access_expression_live,
  mark_string_access_value_live, reset_current_string_access_expression_live,
  string_access_input_is_live, string_access_value_is_marked_live,
  take_current_string_access_expression_live,
};
use variables::{addressed_identifier_hash, addressed_identifier_name};
pub use variables::var;

#[cfg(test)]
mod tests;

// Expressions
// ----------------------------------------------------------------------------

pub type Environment = HashMap<u64, Value>;

pub fn expression(expr: &Expression, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
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
        Expression::Match(match_expr) => match_expression(match_expr, env, p),
        #[cfg(feature = "state_machines")]
        Expression::FsmPipe(fsm_pipe) => crate::state_machines::execute_fsm_pipe(fsm_pipe, env, p),
        x => Err(MechError::new(FeatureNotEnabledError, None)
            .with_compiler_loc()
            .with_tokens(x.tokens())),
    }
}

pub fn match_expression(
    match_expr: &MatchExpression,
    env: Option<&Environment>,
    p: &InterpreterExecution<'_>,
) -> MResult<Value> {
    let source = expression(&match_expr.source, env, p)?;
    let detached_source = match &source {
        Value::MutableReference(reference) => reference.borrow().clone(),
        _ => source.clone(),
    };
    let mut base_env = env.cloned().unwrap_or_default();
    if let Expression::Var(var) = &match_expr.source {
        base_env.insert(var.name.hash(), detached_source.clone());
    }
    if !match_expr
        .arms
        .iter()
        .any(|arm| matches!(arm.pattern, Pattern::Wildcard))
    {
        #[cfg(feature = "enum")]
        if let Some((enum_name, missing_patterns)) =
            infer_missing_enum_match_patterns(match_expr, &detached_source, p)
        {
            if missing_patterns.is_empty() {
                // Exhaustive enum matches do not require a wildcard arm.
                validate_match_arm_output_kinds(match_expr, &base_env, p)?;
            } else {
                return Err(MechError::new(
                    MatchNonExhaustiveVariantsError {
                        enum_name,
                        missing_patterns,
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(match_expr.source.tokens()));
            }
        } else {
            return Err(MechError::new(MatchNonExhaustiveError, None)
                .with_compiler_loc()
                .with_tokens(match_expr.source.tokens()));
        }
    }
    if value_contains_empty(&detached_source) && !has_identity_wildcard_coalesce_arms(match_expr) {
        if let Some(arm) = match_expr
            .arms
            .iter()
            .find(|arm| matches!(arm.pattern, Pattern::Wildcard))
        {
            let passed_guard = match &arm.guard {
                Some(guard) => guard_expression_true(guard, &base_env, p)?,
                None => true,
            };
            if passed_guard {
                return expression(&arm.expression, Some(&base_env), p);
            }
        }
    }

    for (arm_ix, arm) in match_expr.arms.iter().enumerate() {
        let mut guard_env = base_env.clone();
        let matched = match &arm.pattern {
            Pattern::Wildcard => true,
            _ => crate::patterns::pattern_matches_value(
                &arm.pattern,
                &detached_source,
                &mut guard_env,
                p,
            )?,
        };
        if !matched {
            continue;
        }
        let passed_guard = match &arm.guard {
            Some(guard) => guard_expression_true(guard, &guard_env, p)?,
            None => true,
        };
        if passed_guard {
            #[cfg(feature = "matrix")]
            if value_contains_empty(&detached_source) && is_identity_option_matrix_arm(arm) {
                if let Some(wildcard_arm) = match_expr
                    .arms
                    .iter()
                    .find(|arm| matches!(arm.pattern, Pattern::Wildcard))
                {
                    let wildcard_passed = match &wildcard_arm.guard {
                        Some(guard) => guard_expression_true(guard, &base_env, p)?,
                        None => true,
                    };
                    if wildcard_passed {
                        let fallback = expression(&wildcard_arm.expression, Some(&base_env), p)?;
                        let coalesced =
                            coalesce_option_matrix_with_fallback(&detached_source, &fallback)?;
                        return Ok(coalesced);
                    }
                }
            }
            let output = expression(&arm.expression, Some(&guard_env), p)?;
            match_validate_arm_kinds(
                match_expr,
                arm_ix,
                &output.kind(),
                &detached_source,
                &base_env,
                p,
            )?;
            return Ok(output);
        }
    }

    Err(MechError::new(MatchNoArmMatchedError, None)
        .with_compiler_loc()
        .with_tokens(match_expr.source.tokens()))
}

#[cfg(feature = "enum")]
fn infer_missing_enum_match_patterns(
    match_expr: &MatchExpression,
    source: &Value,
    p: &InterpreterExecution<'_>,
) -> Option<(String, Vec<String>)> {
    let (source_enum_id, source_tag) = match source {
        Value::Enum(enum_value) => {
            let enum_brrw = enum_value.borrow();
            if enum_brrw.variants.len() != 1 {
                (Some(enum_brrw.id), None)
            } else {
                (Some(enum_brrw.id), Some(enum_brrw.variants[0].0))
            }
        }
        Value::Atom(atom) => (None, Some(atom.borrow().id())),
        #[cfg(feature = "tuple")]
        Value::Tuple(tuple_val) => {
            let tuple_brrw = tuple_val.borrow();
            match tuple_brrw.elements.first() {
                Some(tag) => match tag.as_ref() {
                    Value::Atom(atom) => (None, Some(atom.borrow().id())),
                    _ => (None, None),
                },
                None => (None, None),
            }
        }
        _ => (None, None),
    };
    let source_tag = source_tag?;

    let mut arm_tags: HashSet<u64> = HashSet::new();
    for arm in &match_expr.arms {
        match &arm.pattern {
            Pattern::Expression(Expression::Literal(Literal::Atom(atom))) => {
                arm_tags.insert(atom.name.hash());
            }
            #[cfg(feature = "atom")]
            Pattern::TupleStruct(pattern_tuple_struct) => {
                arm_tags.insert(pattern_tuple_struct.name.hash());
            }
            _ => {}
        }
    }
    if arm_tags.is_empty() {
        return None;
    }

    let state_brrw = p.state.borrow();
    let enum_def = if let Some(enum_id) = source_enum_id {
        state_brrw.enums.get(&enum_id)?
    } else {
        let candidates: Vec<&MechEnum> = state_brrw
            .enums
            .values()
            .filter(|enm| {
                let variant_ids: HashSet<u64> = enm.variants.iter().map(|(id, _)| *id).collect();
                variant_ids.contains(&source_tag) && arm_tags.is_subset(&variant_ids)
            })
            .collect();
        if candidates.len() != 1 {
            return None;
        }
        candidates[0]
    };
    let variant_ids: HashSet<u64> = enum_def.variants.iter().map(|(id, _)| *id).collect();
    let missing_ids: Vec<u64> = variant_ids.difference(&arm_tags).cloned().collect();
    let names_brrw = enum_def.names.borrow();
    let missing_patterns = enum_def
        .variants
        .iter()
        .filter(|(id, _)| missing_ids.contains(id))
        .map(|(id, payload_kind)| {
            let variant_name = names_brrw
                .get(id)
                .cloned()
                .unwrap_or_else(|| id.to_string());
            if payload_kind.is_some() {
                format!(":{}(…)", variant_name)
            } else {
                format!(":{}", variant_name)
            }
        })
        .collect::<Vec<String>>();
    Some((enum_def.name(), missing_patterns))
}

fn match_validate_arm_kinds(
    match_expr: &MatchExpression,
    matched_arm_ix: usize,
    matched_kind: &ValueKind,
    source: &Value,
    base_env: &Environment,
    p: &InterpreterExecution<'_>,
) -> MResult<()> {
    for (ix, arm) in match_expr.arms.iter().enumerate() {
        if ix == matched_arm_ix {
            continue;
        }
        if matches!(arm.pattern, Pattern::Wildcard) {
            continue;
        }
        let mut arm_env = base_env.clone();
        let applicable = match arm.pattern {
            Pattern::Wildcard => true,
            _ => crate::patterns::pattern_matches_value(
                &arm.pattern,
                source,
                &mut arm_env,
                p,
            )?,
        };
        if !applicable {
            continue;
        }
        let passed_guard = match &arm.guard {
            Some(guard) => guard_expression_true(guard, &arm_env, p)?,
            None => true,
        };
        if !passed_guard {
            continue;
        }
        let arm_value = expression(&arm.expression, Some(&arm_env), p)?;
        let arm_kind = arm_value.kind();
        if arm_kind != *matched_kind {
            return Err(MechError::new(
                MatchArmKindMismatchError {
                    expected: matched_kind.clone(),
                    found: arm_kind,
                },
                None,
            )
            .with_compiler_loc()
            .with_tokens(arm.expression.tokens()));
        }
    }
    Ok(())
}

fn validate_match_arm_output_kinds(
    match_expr: &MatchExpression,
    env: &Environment,
    p: &InterpreterExecution<'_>,
) -> MResult<()> {
    let mut expected: Option<ValueKind> = None;
    for arm in &match_expr.arms {
        let arm_kind = match expression(&arm.expression, Some(env), p) {
            Ok(value) => value.kind(),
            Err(_) => continue,
        };
        if let Some(expected_kind) = &expected {
            if *expected_kind != arm_kind {
                return Err(MechError::new(
                    MatchArmKindMismatchError {
                        expected: expected_kind.clone(),
                        found: arm_kind,
                    },
                    None,
                )
                .with_compiler_loc()
                .with_tokens(arm.expression.tokens()));
            }
        } else {
            expected = Some(arm_kind);
        }
    }
    Ok(())
}

fn guard_expression_true(guard: &Expression, env: &Environment, p: &InterpreterExecution<'_>) -> MResult<bool> {
  let guard_result = expression(guard, Some(env), p)?;
  let flag = validate_guard_expression_result(guard_result, guard.tokens())?;
  let result = *flag.borrow();
  Ok(result)
}

pub(crate) fn validate_guard_expression_result(
  guard_result: Value,
  tokens: Vec<Token>,
) -> MResult<Ref<bool>> {
  match guard_result {
    #[cfg(feature = "bool")]
    Value::Bool(flag) => Ok(flag),
    _ => Err(MechError::new(
      InvalidGuardExpressionError {
        found: guard_result.kind(),
      },
      None,
    )
    .with_compiler_loc()
    .with_tokens(tokens)),
  }
}

fn is_identity_option_matrix_arm(arm: &MatchArm) -> bool {
  match (&arm.pattern, &arm.expression) {
      (Pattern::Expression(Expression::Var(pattern_var)), Expression::Var(expr_var)) => {
        pattern_var.name.hash() == expr_var.name.hash()
      }
      _ => false,
  }
}

fn has_identity_wildcard_coalesce_arms(match_expr: &MatchExpression) -> bool {
  let has_identity = match_expr.arms.iter().any(is_identity_option_matrix_arm);
  let has_wildcard = match_expr
      .arms
      .iter()
      .any(|arm| matches!(arm.pattern, Pattern::Wildcard));
  has_identity && has_wildcard
}

#[cfg(feature = "matrix")]
fn coalesce_option_matrix_with_fallback(source: &Value, fallback: &Value) -> MResult<Value> {
  let source_kind = source.kind();
  if let ValueKind::Option(inner_kind) = source_kind.clone() {
    let raw = match source {
        Value::Typed(inner, _) => inner.as_ref().clone(),
        value => value.clone(),
    };
    let candidate = match raw {
        Value::Empty | Value::EmptyKind(_) => fallback.clone(),
        value => value,
    };
    return candidate.convert_to(inner_kind.as_ref()).ok_or_else(|| {
        MechError::new(
            CannotConvertToTypeError {
                target_type: "requested type",
            },
            None,
        )
        .with_compiler_loc()
    });
  }
  let (inner_kind, shape) = match source_kind {
    ValueKind::Matrix(element_kind, shape) => match *element_kind {
        ValueKind::Option(inner) => (*inner, shape),
        _ => return Ok(source.clone()),
    },
    _ => return Ok(source.clone()),
  };
  let values = match crate::patterns::matrix_like_values(source) {
    Some(values) => values,
    None => return Ok(source.clone()),
  };
  let fill_value = fallback
    .convert_to(&inner_kind)
    .ok_or_else(|| {
        MechError::new(
            CannotConvertToTypeError {
                target_type: "requested type",
            },
            None,
        )
        .with_compiler_loc()
    })?;
  let converted_values = values
    .into_iter()
    .map(|value| {
        let raw = match value {
            Value::Empty | Value::EmptyKind(_) => fill_value.clone(),
            other => other,
        };
        raw.convert_to(&inner_kind).ok_or_else(|| {
            MechError::new(
                CannotConvertToTypeError {
                    target_type: "requested type",
                },
                None,
            )
            .with_compiler_loc()
        })
    })
    .collect::<MResult<Vec<Value>>>()?;
  Ok(Value::MatrixValue(Matrix::from_vec(
    converted_values,
    shape[0],
    shape[1],
  )))
}

fn value_contains_empty(value: &Value) -> bool {
  match value {
    Value::Empty | Value::EmptyKind(_) => true,
    #[cfg(feature = "matrix")]
    Value::MatrixValue(matrix) => matrix
        .as_vec()
        .iter()
        .any(|value| value_contains_empty(value)),
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple) => tuple
        .borrow()
        .elements
        .iter()
        .any(|value| value_contains_empty(value.as_ref())),
    Value::Typed(value, _) => value_contains_empty(value),
    Value::MutableReference(reference) => value_contains_empty(&reference.borrow()),
    _ => false,
  }
}

#[cfg(feature = "formulas")]
pub fn factor(fctr: &Factor, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
  match fctr {
    Factor::Term(trm) => {
      let result = term(trm, env, p)?;
      Ok(result)
    }
    Factor::Parenthetical(paren) => factor(&*paren, env, p),
    Factor::Expression(expr) => expression(expr, env, p),
    #[cfg(feature = "math_neg")]
    Factor::Negate(neg) => {
      let value = factor(neg, env, p)?;
      #[cfg(feature = "subscript_formula")]
      let value_is_live = current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
      let arguments = vec![value];
      let function = MathNegate {}.compile(&arguments)?;
      let plan = p.plan();
      let out = register_initialized_expression_function(&plan, function, &arguments)?;
      #[cfg(feature = "subscript_formula")]
      if value_is_live {
        mark_current_string_access_expression_live(p);
        mark_string_access_value_live(p, &out);
      }
      Ok(out)
    }
    #[cfg(feature = "logic_not")]
    Factor::Not(neg) => {
      let value = factor(neg, env, p)?;
      #[cfg(feature = "subscript_formula")]
      let value_is_live = current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
      let arguments = vec![value];
      let function = LogicNot {}.compile(&arguments)?;
      let plan = p.plan();
      let out = register_initialized_expression_function(&plan, function, &arguments)?;
      #[cfg(feature = "subscript_formula")]
      if value_is_live {
        mark_current_string_access_expression_live(p);
        mark_string_access_value_live(p, &out);
      }
      Ok(out)
    }
    #[cfg(feature = "matrix_transpose")]
    Factor::Transpose(fctr) => {
      use mech_matrix::MatrixTranspose;
      let value = factor(fctr, env, p)?;
      #[cfg(feature = "subscript_formula")]
      let value_is_live = current_string_access_expression_live(p) || string_access_input_is_live(&value, p);
      let arguments = vec![value];
      let function = MatrixTranspose {}.compile(&arguments)?;
      let plan = p.plan();
      let out = register_initialized_expression_function(&plan, function, &arguments)?;
      #[cfg(feature = "subscript_formula")]
      if value_is_live {
        mark_current_string_access_expression_live(p);
        mark_string_access_value_live(p, &out);
      }
      Ok(out)
    }
    _ => todo!(),
  }
}

#[cfg(feature = "formulas")]
pub fn term(trm: &Term, env: Option<&Environment>, p: &InterpreterExecution<'_>) -> MResult<Value> {
  let plan = p.plan();
  let mut lhs = factor(&trm.lhs, env, p)?;
  let mut term_plan: Vec<(Box<dyn MechFunction>, Vec<Value>)> = Vec::new();
  for (op, rhs) in &trm.rhs {
    let rhs = factor(&rhs, env, p)?;
    let dependency_arguments = vec![lhs.clone(), rhs.clone()];
    #[cfg(feature = "subscript_formula")]
    let new_fxn_is_live = current_string_access_expression_live(p)
      || string_access_input_is_live(&lhs, p)
      || string_access_input_is_live(&rhs, p);
    let new_fxn: Box<dyn MechFunction> = match op {
      // Math
      FormulaOperator::AddSub(AddSubOp::Add) => match (&lhs, &rhs) {
        #[cfg(feature = "string_concat")]
        (_, value) | (value, _) if value.is_string() => {
          StringConcat {}.compile(&vec![lhs, rhs])?
        }
        #[cfg(feature = "math_add")]
        _ => MathAdd {}.compile(&vec![lhs, rhs])?,
      },
      #[cfg(feature = "math_sub")]
      FormulaOperator::AddSub(AddSubOp::Sub) => MathSub {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_mul")]
      FormulaOperator::MulDiv(MulDivOp::Mul) => MathMul {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_div")]
      FormulaOperator::MulDiv(MulDivOp::Div) => MathDiv {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_mod")]
      FormulaOperator::MulDiv(MulDivOp::Mod) => MathMod {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "math_pow")]
      FormulaOperator::Power(PowerOp::Pow) => MathPow {}.compile(&vec![lhs, rhs])?,

      // Matrix
      #[cfg(feature = "matrix_matmul")]
      FormulaOperator::Vec(VecOp::MatMul) => MatrixMatMul {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "matrix_solve")]
      FormulaOperator::Vec(VecOp::Solve) => MatrixSolve {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "matrix_cross")]
      FormulaOperator::Vec(VecOp::Cross) => todo!(),
      #[cfg(feature = "matrix_dot")]
      FormulaOperator::Vec(VecOp::Dot) => MatrixDot {}.compile(&vec![lhs, rhs])?,

      // Compare
      #[cfg(feature = "compare_eq")]
      FormulaOperator::Comparison(ComparisonOp::Equal) => CompareEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_seq")]
      FormulaOperator::Comparison(ComparisonOp::StrictEqual) => CompareStrictEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_neq")]
      FormulaOperator::Comparison(ComparisonOp::NotEqual) => CompareNotEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_sneq")]
      FormulaOperator::Comparison(ComparisonOp::StrictNotEqual) => CompareStrictNotEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_lte")]
      FormulaOperator::Comparison(ComparisonOp::LessThanEqual) => CompareLessThanEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_gte")]
      FormulaOperator::Comparison(ComparisonOp::GreaterThanEqual) => CompareGreaterThanEqual {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_lt")]
      FormulaOperator::Comparison(ComparisonOp::LessThan) => CompareLessThan {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "compare_gt")]
      FormulaOperator::Comparison(ComparisonOp::GreaterThan) => CompareGreaterThan {}.compile(&vec![lhs, rhs])?,

      // Logic
      #[cfg(feature = "logic_and")]
      FormulaOperator::Logic(LogicOp::And) => LogicAnd {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "logic_or")]
      FormulaOperator::Logic(LogicOp::Or) => LogicOr {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "logic_not")]
      FormulaOperator::Logic(LogicOp::Not) => LogicNot {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "logic_xor")]
      FormulaOperator::Logic(LogicOp::Xor) => LogicXor {}.compile(&vec![lhs, rhs])?,

      // Table
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::InnerJoin) => TableInnerJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftOuterJoin) => TableLeftOuterJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::RightOuterJoin) => TableRightOuterJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::FullOuterJoin) => TableFullOuterJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftSemiJoin) => TableLeftSemiJoin {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "table")]
      FormulaOperator::Table(TableOp::LeftAntiJoin) => TableLeftAntiJoin {}.compile(&vec![lhs, rhs])?,

      // Set
      #[cfg(feature = "set_union")]
      FormulaOperator::Set(SetOp::Union) => SetUnion {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_intersection")]
      FormulaOperator::Set(SetOp::Intersection) => SetIntersection {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_difference")]
      FormulaOperator::Set(SetOp::Difference) => SetDifference {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_symmetric_difference")]
      FormulaOperator::Set(SetOp::SymmetricDifference) => SetSymmetricDifference {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_complement")]
      FormulaOperator::Set(SetOp::Complement) => todo!(),
      #[cfg(feature = "set_subset")]
      FormulaOperator::Set(SetOp::Subset) => SetSubset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_superset")]
      FormulaOperator::Set(SetOp::Superset) => SetSuperset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_proper_subset")]
      FormulaOperator::Set(SetOp::ProperSubset) => SetProperSubset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_proper_superset")]
      FormulaOperator::Set(SetOp::ProperSuperset) => SetProperSuperset {}.compile(&vec![lhs, rhs])?,
      #[cfg(feature = "set_element_of")]
      FormulaOperator::Set(SetOp::ElementOf) => {
        #[cfg(feature = "kind_annotation")]
        if let Value::Kind(kind) = &rhs {
          lhs = Value::Bool(Ref::new(value_in_kind(&lhs, kind, p)));
          continue;
        }
        SetElementOf {}.compile(&vec![lhs, rhs])?
      }
      #[cfg(feature = "set_not_element_of")]
      FormulaOperator::Set(SetOp::NotElementOf) => {
        #[cfg(feature = "kind_annotation")]
        if let Value::Kind(kind) = &rhs {
          lhs = Value::Bool(Ref::new(!value_in_kind(&lhs, kind, p)));
          continue;
        }
        SetNotElementOf {}.compile(&vec![lhs, rhs])?
      }
      x => {
        return Err(MechError::new(
          UnhandledFormulaOperatorError {
            operator: x.clone(),
          },
          None,
        )
        .with_compiler_loc()
        .with_tokens(trm.tokens()));
      }
    };
    if !expression_solves_deferred(p) {
      new_fxn.solve();
    }
    let res = new_fxn.out();
    #[cfg(feature = "subscript_formula")]
    if new_fxn_is_live {
      mark_current_string_access_expression_live(p);
      mark_string_access_value_live(p, &res);
    }
    term_plan.push((new_fxn, dependency_arguments));
    lhs = res;
  }
  register_expression_function_batch(&plan, term_plan)?;
  Ok(lhs)
}

#[cfg(all(feature = "kind_annotation", feature = "enum", feature = "atom"))]
  fn enum_value_matches_kind(value: &Value, enum_id: u64, state: &ProgramState) -> bool {
  let enum_def = match state.enums.get(&enum_id) {
    Some(enm) => enm,
    None => return false,
  };
  let names_brrw = enum_def.names.borrow();
  let atom_matches_variant = |variant_id: u64, atom_id: u64, atom_name: &str| {
    if variant_id == atom_id {
      return true;
    }
    let variant_name = match names_brrw.get(&variant_id) {
      Some(name) => name.as_str(),
      None => return false,
    };
    let short_variant = variant_name.rsplit('/').next().unwrap_or(variant_name);
    let short_atom = atom_name.rsplit('/').next().unwrap_or(atom_name);
    short_variant == short_atom
  };
  match value {
    Value::Enum(enum_value) => {
      let enum_value_brrw = enum_value.borrow();
      if enum_value_brrw.id != enum_id {
        return false;
      }
      if enum_value_brrw.variants.len() != 1 {
        return false;
      }
      let (variant_id, payload) = &enum_value_brrw.variants[0];
      let (_, declared_payload_kind) = match enum_def
        .variants
        .iter()
        .find(|(known_variant, _)| *known_variant == *variant_id)
      {
        Some(entry) => entry,
        None => return false,
      };
      match (payload, declared_payload_kind) {
        (None, None) => true,
        (Some(payload_value), Some(Value::Kind(expected_kind))) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => {
            enum_value_matches_kind(payload_value, *inner_enum_id, state)
          }
          _ => payload_value.kind() == expected_kind.clone() || payload_value.convert_to(expected_kind).is_some(),
        },
        _ => false,
      }
    }
    Value::Atom(atom) => {
      let atom_brrw = atom.borrow();
      let variant_id = atom_brrw.id();
      let atom_name = atom_brrw.name();
      enum_def
        .variants
        .iter()
        .any(|(known_variant, payload_kind)| atom_matches_variant(*known_variant, variant_id, &atom_name) && payload_kind.is_none())
    }
    #[cfg(feature = "tuple")]
    Value::Tuple(tuple_val) => {
      let tuple_brrw = tuple_val.borrow();
      if tuple_brrw.elements.len() != 2 {
        return false;
      }
      let (tag, tag_name) = match tuple_brrw.elements[0].as_ref() {
        Value::Atom(atom) => {
          let atom_brrw = atom.borrow();
          (atom_brrw.id(), atom_brrw.name())
        }
        _ => return false,
      };
      let payload = tuple_brrw.elements[1].as_ref();
      let (_, declared_payload_kind) = match enum_def
        .variants
        .iter()
        .find(|(known_variant, _)| atom_matches_variant(*known_variant, tag, &tag_name))
      {
        Some(entry) => entry,
        None => return false,
      };
      match declared_payload_kind {
        Some(Value::Kind(expected_kind)) => match expected_kind {
          ValueKind::Enum(inner_enum_id, _) => {
            enum_value_matches_kind(payload, *inner_enum_id, state)
          }
          _ => payload.kind() == expected_kind.clone() || payload.convert_to(expected_kind).is_some(),
        },
        _ => false,
      }
    }
    _ => false,
  }
}

#[cfg(feature = "kind_annotation")]
fn value_in_kind(value: &Value, kind: &ValueKind, p: &InterpreterExecution<'_>) -> bool {
  let detached = detach_value(value);
  #[cfg(all(feature = "enum", feature = "atom"))]
  if let ValueKind::Enum(enum_id, _) = kind {
    let state_brrw = p.state.borrow();
    return enum_value_matches_kind(&detached, *enum_id, &state_brrw);
  }
  detached.convert_to(kind).is_some()
}

// Errors
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UnhandledFormulaOperatorError {
  pub operator: FormulaOperator,
}
impl MechErrorKind for UnhandledFormulaOperatorError {
  fn name(&self) -> &str {
    "UnhandledFormulaOperator"
  }
  fn message(&self) -> String {
    format!("Unhandled formula operator: {:#?}", self.operator)
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedVariableError {
  pub id: u64,
  pub name: String,
}
impl MechErrorKind for UndefinedVariableError {
  fn name(&self) -> &str {
    "UndefinedVariable"
  }

  fn message(&self) -> String {
    format!("Undefined variable `{}` (id: {})", self.name, self.id)
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
pub struct ComprehensionGeneratorError {
  found: ValueKind,
}

impl MechErrorKind for ComprehensionGeneratorError {
  fn name(&self) -> &str {
    "ComprehensionGenerator"
  }
  fn message(&self) -> String {
      format!(
        "Comprehension generator must produce a set or matrix, found kind: {:?}",
        self.found
      )
  }
}

#[derive(Debug, Clone)]
pub(crate) struct SetComprehensionOutputKindMismatchError {
  found: ValueKind,
}

impl MechErrorKind for SetComprehensionOutputKindMismatchError {
  fn name(&self) -> &str {
    "SetComprehensionOutputKindMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Set comprehension bytecode output must be a set, but found {:?}.",
      self.found
    )
  }
}

#[derive(Debug, Clone)]
pub struct PatternExpectedTupleError {
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
    format!(
      "Arity mismatch: expected {}, found {}",
      self.expected, self.found
    )
  }
}

#[derive(Debug, Clone)]
pub struct PatternMatchError {
  pub var: String,
  pub expected: String,
  pub found: String,
}

#[derive(Debug, Clone)]
pub struct MatchNoArmMatchedError;
impl MechErrorKind for MatchNoArmMatchedError {
  fn name(&self) -> &str {
    "MatchNoArmMatched"
  }
  fn message(&self) -> String {
    format!("No match arm matched the provided value.")
  }
}

#[derive(Debug, Clone)]
pub struct MatchArmKindMismatchError {
  expected: ValueKind,
  found: ValueKind,
}
impl MechErrorKind for MatchArmKindMismatchError {
  fn name(&self) -> &str {
    "MatchArmKindMismatch"
  }
  fn message(&self) -> String {
    format!(
      "Expected {:?}, found {:?}",
      self.expected, self.found
    )
  }
}

#[derive(Debug, Clone)]
pub struct MatchNonExhaustiveError;
impl MechErrorKind for MatchNonExhaustiveError {
  fn name(&self) -> &str {
    "MatchNonExhaustive"
  }
  fn message(&self) -> String {
    "Match expression must include a wildcard (`*`) arm.".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct MatchNonExhaustiveVariantsError {
  pub enum_name: String,
  pub missing_patterns: Vec<String>,
}
impl MechErrorKind for MatchNonExhaustiveVariantsError {
  fn name(&self) -> &str {
    "MatchNonExhaustive"
  }
  fn message(&self) -> String {
    format!(
      "Match over enum '{}' is non-exhaustive. Missing variants: {}. Handle the missing variants or add a wildcard (`*`) arm to catch all cases.",
      self.enum_name,
      self.missing_patterns.join(", ")
    )
  }
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

#[derive(Debug, Clone)]
pub struct InvalidGuardExpressionError {
  found: ValueKind,
}

impl MechErrorKind for InvalidGuardExpressionError {
  fn name(&self) -> &str {
    "InvalidGuardExpression"
  }
  fn message(&self) -> String {
    format!(
      "Guard expressions must evaluate to a boolean value. Found kind: {:?}",
      self.found
    )
  }
}

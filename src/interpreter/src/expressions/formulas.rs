use super::environment::expression_solves_deferred;
use super::registration::{
  register_expression_function_batch, register_initialized_expression_function,
};
#[cfg(feature = "subscript_formula")]
use super::{
  current_string_access_expression_live, mark_current_string_access_expression_live,
  mark_string_access_value_live, string_access_input_is_live,
};
use super::{Environment, UnhandledFormulaOperatorError, expression};
#[cfg(feature = "compare_eq")]
use crate::CompareEqual;
#[cfg(feature = "compare_gt")]
use crate::CompareGreaterThan;
#[cfg(feature = "compare_gte")]
use crate::CompareGreaterThanEqual;
#[cfg(feature = "compare_lt")]
use crate::CompareLessThan;
#[cfg(feature = "compare_lte")]
use crate::CompareLessThanEqual;
#[cfg(feature = "compare_neq")]
use crate::CompareNotEqual;
#[cfg(feature = "compare_seq")]
use crate::CompareStrictEqual;
#[cfg(feature = "compare_sneq")]
use crate::CompareStrictNotEqual;
#[cfg(feature = "logic_and")]
use crate::LogicAnd;
#[cfg(feature = "logic_not")]
use crate::LogicNot;
#[cfg(feature = "logic_or")]
use crate::LogicOr;
#[cfg(feature = "logic_xor")]
use crate::LogicXor;
#[cfg(feature = "math_add")]
use crate::MathAdd;
#[cfg(feature = "math_div")]
use crate::MathDiv;
#[cfg(feature = "math_mod")]
use crate::MathMod;
#[cfg(feature = "math_mul")]
use crate::MathMul;
#[cfg(feature = "math_neg")]
use crate::MathNegate;
#[cfg(feature = "math_pow")]
use crate::MathPow;
#[cfg(feature = "math_sub")]
use crate::MathSub;
#[cfg(feature = "matrix_dot")]
use crate::MatrixDot;
#[cfg(feature = "matrix_matmul")]
use crate::MatrixMatMul;
#[cfg(feature = "matrix_solve")]
use crate::MatrixSolve;
#[cfg(feature = "set_difference")]
use crate::SetDifference;
#[cfg(feature = "set_element_of")]
use crate::SetElementOf;
#[cfg(feature = "set_intersection")]
use crate::SetIntersection;
#[cfg(feature = "set_not_element_of")]
use crate::SetNotElementOf;
#[cfg(feature = "set_proper_subset")]
use crate::SetProperSubset;
#[cfg(feature = "set_proper_superset")]
use crate::SetProperSuperset;
#[cfg(feature = "set_subset")]
use crate::SetSubset;
#[cfg(feature = "set_superset")]
use crate::SetSuperset;
#[cfg(feature = "set_symmetric_difference")]
use crate::SetSymmetricDifference;
#[cfg(feature = "set_union")]
use crate::SetUnion;
#[cfg(feature = "string_concat")]
use crate::StringConcat;
#[cfg(feature = "table")]
use crate::{
  TableFullOuterJoin, TableInnerJoin, TableLeftAntiJoin, TableLeftOuterJoin, TableLeftSemiJoin,
  TableRightOuterJoin,
};
use crate::{
  AddSubOp, ComparisonOp, Factor, FormulaOperator, InterpreterExecution, LogicOp, MResult,
  MechError, MechFunction, MulDivOp, NativeFunctionCompiler, PowerOp, ProgramState, Ref, SetOp,
  TableOp, Term, Value, ValueKind, VecOp, detach_value,
};

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

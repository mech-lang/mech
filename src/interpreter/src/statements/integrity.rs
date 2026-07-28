#[cfg(feature = "invariant_define")]
use super::{VariableAlreadyDefinedError, detach_variable_value};
#[cfg(feature = "invariant_define")]
use crate::{
  ComparisonOp, Expression, Factor, FormulaOperator, IntegrityConstraint, InterpreterExecution,
  InvariantDefine, Literal, MResult, MechError, NativeFunctionCompiler, Ref, Token, ValRef, Value,
  expression, literal,
};
#[cfg(feature = "invariant_define")]
use crate::stdlib::define::VarDefine;

#[cfg(feature = "invariant_define")]
pub fn invariant_define(inv_def: &InvariantDefine, p: &InterpreterExecution<'_>) -> MResult<Value> {
  let invariant_id = inv_def.name.hash();
  let invariant_name = inv_def.name.to_string();
  let invariant_expression = tokens_to_string(&inv_def.expression.tokens());
  {
    let symbols = p.symbols();
    if symbols.borrow().contains(invariant_id) {
      return Err(MechError::new(
        VariableAlreadyDefinedError { id: invariant_id },
        None
      ).with_compiler_loc().with_tokens(inv_def.name.tokens()));
    }
  }
  let plan = p.plan();
  let result = expression(&inv_def.expression, None, p)?;
  let detached_result = detach_variable_value(&result);
  let result_ref = {
    let state = p.state.borrow();
    state.save_symbol(
      invariant_id,
      invariant_name.clone(),
      detached_result.clone(),
      false,
    )
  };

  let var_define_arguments = vec![
    detached_result,
    Value::String(Ref::new(invariant_name.clone())),
    Value::Bool(Ref::new(false)),
  ];
  let var_def_fxn = VarDefine{}.compile(&var_define_arguments)?;
  plan.register_function(var_def_fxn, &[])?;

  let (lhs, operator, rhs) = integrity_constraint_operands(inv_def, p);
  p.state.borrow_mut().integrity_constraints.insert(
    invariant_id,
    IntegrityConstraint {
      id: invariant_id,
      name: invariant_name,
      expression: invariant_expression,
      result: result_ref,
      lhs,
      operator,
      rhs,
      tokens: inv_def.expression.tokens(),
    },
  );
  Ok(result)
}


#[cfg(feature = "invariant_define")]
fn tokens_to_string(tokens: &[Token]) -> String {
  tokens.iter().flat_map(|t| t.chars.clone()).collect::<String>()
}


#[cfg(feature = "invariant_define")]
fn value_to_ref(value: Value) -> ValRef {
  match value {
    Value::MutableReference(r) => r.clone(),
    other => Ref::new(other),
  }
}


#[cfg(feature = "invariant_define")]
fn integrity_constraint_operands(
  inv_def: &InvariantDefine,
  p: &InterpreterExecution<'_>,
) -> (Option<ValRef>, Option<FormulaOperator>, Option<ValRef>) {
  let factor = match &inv_def.expression {
    Expression::Formula(factor) => factor,
    _ => return (None, None, None),
  };
  let term = match transparent_factor(factor) {
    Factor::Term(term) => term,
    _ => return (None, None, None),
  };
  if term.rhs.len() != 1 {
    return (None, None, None);
  }
  let (operator, rhs_factor) = &term.rhs[0];
  if !matches!(
    operator,
    FormulaOperator::Comparison(
      ComparisonOp::Equal
        | ComparisonOp::NotEqual
        | ComparisonOp::LessThan
        | ComparisonOp::LessThanEqual
        | ComparisonOp::GreaterThan
        | ComparisonOp::GreaterThanEqual
    )
  ) {
    return (None, None, None);
  }
  (
    integrity_constraint_operand(&term.lhs, p),
    Some(operator.clone()),
    integrity_constraint_operand(rhs_factor, p),
  )
}


#[cfg(feature = "invariant_define")]
fn transparent_factor(factor: &Factor) -> &Factor {
  match factor {
    Factor::Parenthetical(inner) => transparent_factor(inner),
    Factor::Term(term) if term.rhs.is_empty() => transparent_factor(&term.lhs),
    other => other,
  }
}


#[cfg(feature = "invariant_define")]
fn integrity_constraint_operand(
  factor: &Factor,
  p: &InterpreterExecution<'_>,
) -> Option<ValRef> {
  let expression = match transparent_factor(factor) {
    Factor::Expression(expression) => expression.as_ref(),
    _ => return None,
  };
  match expression {
    Expression::Var(var) if var.context.is_none() && var.kind.is_none() => {
      p.state.borrow().get_symbol(var.name.hash())
    }
    Expression::Literal(
      literal_node @ (
        Literal::Atom(_)
        | Literal::Boolean(_)
        | Literal::Number(_)
        | Literal::String(_)
      ),
    ) => literal(literal_node, p).ok().map(value_to_ref),
    _ => None,
  }
}


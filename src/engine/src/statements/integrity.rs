#[cfg(feature = "invariant_define")]
use super::VariableAlreadyDefinedError;
#[cfg(feature = "invariant_define")]
use super::variable_define::detach_variable_value;
#[cfg(feature = "invariant_define")]
use crate::{
    ComparisonOp, Expression, Factor, FormulaOperator, IntegrityConstraint, InterpreterExecution,
    InvariantDefine, Literal, MResult, MechError, OperationId, SpecializationInvocation, Token,
    ValueCell, expression_cell, literal,
};

#[cfg(feature = "invariant_define")]
pub fn invariant_define(
    definition: &InvariantDefine,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ValueCell> {
    let invariant_id = definition.name.hash();
    let invariant_name = definition.name.to_string();
    let invariant_expression = tokens_to_string(&definition.expression.tokens());
    {
        let symbols = interpreter.symbols();
        if symbols.borrow().contains(invariant_id) {
            return Err(
                MechError::new(VariableAlreadyDefinedError { id: invariant_id }, None)
                    .with_compiler_loc()
                    .with_tokens(definition.name.tokens()),
            );
        }
    }

    let result = expression_cell(&definition.expression, None, interpreter)?;
    let detached_result = detach_variable_value(&result);
    let result_cell = interpreter.state.borrow().save_symbol(
        invariant_id,
        invariant_name.clone(),
        detached_result.clone(),
        false,
    );

    let arguments = vec![
        detached_result,
        ValueCell::from_exact(invariant_name.clone())?,
        ValueCell::from_exact(false)?,
        ValueCell::from_exact(!interpreter.in_user_function_scope())?,
    ];
    let invocation = SpecializationInvocation::from_cells(arguments.into_boxed_slice());
    let specialized = interpreter.specialize_visible_invocation_named(
        OperationId::from_name("var/define"),
        Some("var/define"),
        &invocation,
    )?;
    interpreter.plan().register_specialized(specialized)?;

    let (lhs, operator, rhs) = integrity_constraint_operands(definition, interpreter);
    interpreter.state.borrow_mut().integrity_constraints.insert(
        invariant_id,
        IntegrityConstraint {
            id: invariant_id,
            name: invariant_name,
            expression: invariant_expression,
            result: result_cell,
            lhs,
            operator,
            rhs,
            tokens: definition.expression.tokens(),
        },
    );
    Ok(result)
}

#[cfg(feature = "invariant_define")]
fn tokens_to_string(tokens: &[Token]) -> String {
    tokens
        .iter()
        .flat_map(|token| token.chars.clone())
        .collect::<String>()
}

#[cfg(feature = "invariant_define")]
fn integrity_constraint_operands(
    definition: &InvariantDefine,
    interpreter: &InterpreterExecution<'_>,
) -> (
    Option<ValueCell>,
    Option<FormulaOperator>,
    Option<ValueCell>,
) {
    let factor = match &definition.expression {
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
        integrity_constraint_operand(&term.lhs, interpreter),
        Some(operator.clone()),
        integrity_constraint_operand(rhs_factor, interpreter),
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
    interpreter: &InterpreterExecution<'_>,
) -> Option<ValueCell> {
    let expression = match transparent_factor(factor) {
        Factor::Expression(expression) => expression.as_ref(),
        _ => return None,
    };
    match expression {
        Expression::Var(variable) if variable.context.is_none() && variable.kind.is_none() => {
            interpreter.state.borrow().get_symbol(variable.name.hash())
        }
        Expression::Literal(
            literal_node @ (Literal::Atom(_)
            | Literal::Boolean(_)
            | Literal::Number(_)
            | Literal::String(_)),
        ) => literal(literal_node, interpreter)
            .ok()
            .and_then(|input| input.cell().ok().cloned()),
        _ => None,
    }
}

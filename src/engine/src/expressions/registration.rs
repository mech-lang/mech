use crate::{InitialSolvePolicy, MResult, MechFunction, Plan, Value};

#[cfg(feature = "functions")]
pub(super) fn register_initialized_expression_function(
    plan: &Plan,
    function: Box<dyn MechFunction>,
    arguments: &[Value],
) -> MResult<Value> {
    if !plan.activation_registration_active()
        && function.initial_solve_policy() == InitialSolvePolicy::Solve
    {
        function.solve_result()?;
    }
    let output = function.out();
    plan.register_function(function, arguments)?;
    Ok(output)
}

#[cfg(feature = "functions")]
pub(super) fn register_expression_function_batch(
    plan: &Plan,
    functions: Vec<(Box<dyn MechFunction>, Vec<Value>)>,
) -> MResult<()> {
    for (function, arguments) in functions {
        plan.register_function(function, &arguments)?;
    }
    Ok(())
}

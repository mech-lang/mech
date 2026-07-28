use crate::{MResult, MechFunction, Plan, Value};

#[cfg(feature = "functions")]
pub(super) fn register_initialized_expression_function(
    plan: &Plan,
    function: Box<dyn MechFunction>,
    arguments: &[Value],
) -> MResult<Value> {
    let node_id = plan.register_function(function, arguments)?;
    let plan_borrow = plan.borrow();
    let function = &plan_borrow[node_id];
    if !plan.activation_registration_active() {
        function.solve();
    }
    Ok(function.out())
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

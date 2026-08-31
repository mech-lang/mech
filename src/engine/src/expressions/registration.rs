use crate::{MResult, Plan, SpecializedFunction};

#[cfg(all(
    feature = "functions",
    any(
        test,
        feature = "math_neg",
        feature = "logic_not",
        feature = "matrix_transpose",
        feature = "range_inclusive",
        feature = "range_exclusive",
        feature = "range_inclusive_increment",
        feature = "range_exclusive_increment",
        feature = "subscript_range",
        all(feature = "subscript", feature = "access")
    )
))]
pub(super) fn register_initialized_expression_function(
    plan: &Plan,
    specialized: SpecializedFunction,
) -> MResult<crate::ValueCell> {
    let instance = specialized.into_instance();
    if !plan.activation_registration_active()
        && instance.implementation().initial_solve_policy() == crate::InitialSolvePolicy::Solve
    {
        instance.solve_result()?;
    }
    let output = instance.output().clone();
    plan.register_instance(instance)?;
    Ok(output)
}

#[cfg(all(
    feature = "functions",
    any(
        test,
        feature = "string_concat",
        feature = "math_add",
        feature = "math_sub",
        feature = "math_mul",
        feature = "math_div",
        feature = "math_mod",
        feature = "math_pow",
        feature = "matrix_matmul",
        feature = "matrix_solve",
        feature = "matrix_dot",
        feature = "compare_eq",
        feature = "compare_seq",
        feature = "compare_neq",
        feature = "compare_sneq",
        feature = "compare_lte",
        feature = "compare_gte",
        feature = "compare_lt",
        feature = "compare_gt",
        feature = "logic_and",
        feature = "logic_or",
        feature = "logic_not",
        feature = "logic_xor",
        feature = "table",
        feature = "set_union",
        feature = "set_intersection",
        feature = "set_difference",
        feature = "set_symmetric_difference",
        feature = "set_subset",
        feature = "set_superset",
        feature = "set_proper_subset",
        feature = "set_proper_superset",
        feature = "set_element_of",
        feature = "set_not_element_of"
    )
))]
pub(super) fn register_expression_function_batch(
    plan: &Plan,
    functions: Vec<SpecializedFunction>,
) -> MResult<()> {
    for specialized in functions {
        plan.register_instance(specialized.into_instance())?;
    }
    Ok(())
}

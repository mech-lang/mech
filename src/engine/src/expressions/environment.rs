use crate::{Interpreter, Ref};

pub(crate) struct DeferredExpressionSolveScope {
    depth: Ref<usize>,
}

impl DeferredExpressionSolveScope {
    pub(crate) fn enter(interpreter: &Interpreter) -> Self {
        let depth = interpreter.deferred_expression_solve_depth.clone();
        *depth.borrow_mut() += 1;
        Self { depth }
    }
}

impl Drop for DeferredExpressionSolveScope {
    fn drop(&mut self) {
        let mut depth = self.depth.borrow_mut();
        debug_assert!(*depth > 0);
        *depth -= 1;
    }
}

#[cfg(any(
    all(feature = "subscript", feature = "access"),
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
))]
pub(super) fn expression_solves_deferred(interpreter: &Interpreter) -> bool {
    *interpreter.deferred_expression_solve_depth.borrow() > 0
}

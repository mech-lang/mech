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

pub(super) fn expression_solves_deferred(interpreter: &Interpreter) -> bool {
    *interpreter.deferred_expression_solve_depth.borrow() > 0
}

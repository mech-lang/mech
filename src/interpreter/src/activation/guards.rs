use super::captures::transaction_bool_state;
use super::{
    ActivationPatternCapture, ActivationPatternGuardDependencyInvariant,
    ActivationPatternGuardMustBePure, activation_scope_entry_cells,
};
use crate::{
    Expression, InterpreterExecution, MResult, MechError, MechFunctionImpl, ReactiveNodeId,
    ReactiveNodeKind, ReactiveSolveStatus, Ref, Value,
};

pub(super) struct GuardFinalize {
    pub(super) guard: Ref<bool>,
    pub(super) eligible: Ref<bool>,
    pub(super) out: Ref<usize>,
}
impl MechFunctionImpl for GuardFinalize {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.eligible.borrow_mut() = *self.guard.borrow();
        *self.out.borrow_mut() += 1;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        let mut values = self.reactive_output_values();
        values.push(transaction_bool_state(&self.eligible)?);
        Ok(values)
    }
    fn to_string(&self) -> String {
        "ActivationPatternGuardFinalize".into()
    }
}

pub(super) struct ElaboratedPatternGuard {
    pub(super) finalizer_node: ReactiveNodeId,
    pub(super) node_start: usize,
    pub(super) node_end: usize,
}

pub(super) fn elaborate_patterned_arm_guard(
    guard: &Expression,
    captures: &[ActivationPatternCapture],
    pulse: &Value,
    eligible: &Ref<bool>,
    completion: Ref<usize>,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<ElaboratedPatternGuard> {
    let symbols = interpreter.symbols();
    let symbol_snapshot = symbols.borrow().snapshot();
    let plan = interpreter.plan();
    let original_scope_depth = plan.activation_registration_depth();
    {
        let mut symbols = symbols.borrow_mut();
        for capture in captures {
            symbols.mutable_variables.remove(&capture.id);
            symbols.insert(capture.id, capture.proposed.clone(), false);
            symbols
                .dictionary
                .borrow_mut()
                .insert(capture.id, capture.name.clone());
        }
    }
    let node_start = plan.len();
    let pulse_cells = pulse.reactive_root_cell_ids();
    plan.push_activation_registration_scope_with_sampled_cells(
        pulse_cells.clone(),
        activation_scope_entry_cells(interpreter),
    );
    let result = (|| -> MResult<ElaboratedPatternGuard> {
        let _deferred_expression_solves =
            crate::expressions::DeferredExpressionSolveScope::enter(interpreter);
        let _persistent_user_function_plan =
            crate::functions::PersistentUserFunctionPlanScope::enter(interpreter);
        let guard_value = crate::expression(guard, None, interpreter)?;
        let guard_ref = crate::expressions::validate_guard_expression_result(
            guard_value.clone(),
            guard.tokens(),
        )?;
        let finalizer_node = plan.register_function(
            Box::new(GuardFinalize {
                guard: guard_ref,
                eligible: eligible.clone(),
                out: completion,
            }),
            &[guard_value],
        )?;
        let node_end = plan.len();
        {
            let plan_borrow = plan.borrow();
            if plan_borrow.nodes[node_start..node_end]
                .iter()
                .any(|node| node.kind != ReactiveNodeKind::Combinational)
            {
                return Err(
                    MechError::new(ActivationPatternGuardMustBePure, None)
                        .with_tokens(guard.tokens()),
                );
            }
        }
        {
            let Some(pulse_cell) = pulse_cells.first().copied() else {
                return Err(
                    MechError::new(ActivationPatternGuardDependencyInvariant, None)
                        .with_tokens(guard.tokens()),
                );
            };
            let mut plan_borrow = plan.borrow_mut();
            for node in node_start..node_end {
                if !plan_borrow.add_reactive_dependency(node, pulse_cell) {
                    return Err(
                        MechError::new(ActivationPatternGuardDependencyInvariant, None)
                            .with_tokens(guard.tokens()),
                    );
                }
                for capture in captures {
                    let capture_cell = capture.proposed.reactive_root_cell_ids()[0];
                    if !plan_borrow.add_sampled_dependency(node, capture_cell) {
                        return Err(
                            MechError::new(ActivationPatternGuardDependencyInvariant, None)
                                .with_tokens(guard.tokens()),
                        );
                    }
                }
            }
        }
        Ok(ElaboratedPatternGuard {
            finalizer_node,
            node_start,
            node_end,
        })
    })();
    while plan.activation_registration_depth() > original_scope_depth {
        plan.pop_activation_registration_scope();
    }
    symbols.borrow_mut().restore(symbol_snapshot);
    result
}

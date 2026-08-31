use super::captures::{increment, read_bool, register_node, write_bool};
use super::{
    ActivationPatternCapture, ActivationPatternGuardDependencyInvariant,
    ActivationPatternGuardMustBePure, activation_scope_entry_cells,
};
use crate::{
    Expression, FunctionStatePort, InterpreterExecution, MResult, MechError, MechFunctionImpl,
    ReactiveNodeId, ReactiveNodeKind, ReactiveSolveStatus, ValueCell,
};

pub(super) struct GuardFinalize {
    pub(super) guard: ValueCell,
    pub(super) eligible: ValueCell,
    pub(super) out: ValueCell,
}
impl MechFunctionImpl for GuardFinalize {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        write_bool(&self.eligible, read_bool(&self.guard)?)?;
        increment(&self.out)?;
        Ok(ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.out))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![
            FunctionStatePort::from_cell(&self.out),
            FunctionStatePort::from_cell(&self.eligible),
        ]))
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
    pulse: &ValueCell,
    eligible: &ValueCell,
    completion: ValueCell,
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
            symbols.insert_cell(capture.id, capture.proposed.clone(), false);
            symbols
                .dictionary
                .borrow_mut()
                .insert(capture.id, capture.name.clone());
        }
    }
    let node_start = plan.len();
    let pulse_cells = vec![pulse.reactive_cell_id()];
    plan.push_activation_registration_scope_with_sampled_cells(
        pulse_cells.clone(),
        activation_scope_entry_cells(interpreter),
    );
    let result = (|| -> MResult<ElaboratedPatternGuard> {
        let _deferred_expression_solves =
            crate::expressions::DeferredExpressionSolveScope::enter(interpreter);
        let _persistent_user_function_plan =
            crate::function::PersistentUserFunctionPlanScope::enter(interpreter);
        let guard_value = crate::expression_cell(guard, None, interpreter)?;
        let guard_cell =
            crate::expressions::validate_guard_expression_result(guard_value, guard.tokens())?;
        let finalizer_node = register_node(
            &plan,
            Box::new(GuardFinalize {
                guard: guard_cell.clone(),
                eligible: eligible.clone(),
                out: completion.clone(),
            }),
            completion,
            vec![guard_cell],
        )?;
        let node_end = plan.len();
        {
            let plan_borrow = plan.borrow();
            if plan_borrow.nodes[node_start..node_end]
                .iter()
                .any(|node| node.kind != ReactiveNodeKind::Combinational)
            {
                return Err(MechError::new(ActivationPatternGuardMustBePure, None)
                    .with_tokens(guard.tokens()));
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
                    if !plan_borrow
                        .add_sampled_dependency(node, capture.proposed.reactive_cell_id())
                    {
                        return Err(MechError::new(
                            ActivationPatternGuardDependencyInvariant,
                            None,
                        )
                        .with_tokens(guard.tokens()));
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

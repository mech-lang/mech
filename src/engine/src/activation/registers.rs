use crate::{
    ActivationArm, ActivationArmBody, Expression, FunctionStatePort, Interpreter,
    InterpreterExecution, MResult, MechError, MechFunctionImpl, ReactiveCellId,
    ReactiveSolveStatus, SliceRef, Token, ValueCell,
};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, GenericError, MechFunctionCompiler, Register};

use super::{
    ActivationPatternBodyDependencyInvariant, ActivationPatternCapture,
    ActivationPatternContextEffectUnsupported, ActivationPatternRegisterWriteUnsupported,
    ActivationScopeTriggerWriteUnsupported, activation_scope_entry_cells,
    captures::commit_proposed_captures, validation::validate_patterned_expression,
};

pub(super) fn validate_patterned_register_write(
    target: &SliceRef,
    expression: &Expression,
    trigger_id: u64,
    trigger_cells: &[ReactiveCellId],
    interpreter: &Interpreter,
    tokens: Vec<Token>,
) -> MResult<()> {
    if target.context.is_some() {
        return Err(
            MechError::new(ActivationPatternContextEffectUnsupported, None).with_tokens(tokens),
        );
    }
    let target_id = target.name.hash();
    let aliases_trigger = interpreter
        .symbols()
        .borrow()
        .get(target_id)
        .is_some_and(|value| trigger_cells.contains(&value.reactive_cell_id()));
    if target_id == trigger_id || aliases_trigger {
        return Err(
            MechError::new(ActivationScopeTriggerWriteUnsupported, None).with_tokens(tokens)
        );
    }
    // Indexed assignment implementations still mutate eagerly and do not
    // implement the reactive-register staging contract.
    if target.subscript.is_some() {
        return Err(
            MechError::new(ActivationPatternRegisterWriteUnsupported, None).with_tokens(tokens),
        );
    }
    validate_patterned_expression(expression)
}

pub(super) fn elaborate_patterned_arm_body(
    arm: &ActivationArm,
    captures: &[ActivationPatternCapture],
    pulse: &ValueCell,
    interpreter: &InterpreterExecution<'_>,
) -> MResult<(usize, usize)> {
    let symbols = interpreter.symbols();
    let symbol_snapshot = symbols.borrow().snapshot();
    let plan = interpreter.plan();
    let original_scope_depth = plan.activation_registration_depth();
    {
        let mut symbols = symbols.borrow_mut();
        for capture in captures {
            symbols.mutable_variables.remove(&capture.id);
            symbols.insert_cell(capture.id, capture.committed.clone(), false);
            symbols
                .dictionary
                .borrow_mut()
                .insert(capture.id, capture.name.clone());
        }
    }
    let body_node_start = plan.len();
    plan.push_activation_registration_scope_with_sampled_cells(
        vec![pulse.reactive_cell_id()],
        activation_scope_entry_cells(interpreter),
    );
    let body_result = (|| -> MResult<()> {
        match &arm.body {
            ActivationArmBody::Block(body) => {
                for (code, _) in body {
                    crate::mech_code(code, interpreter)?;
                }
                Ok(())
            }
            ActivationArmBody::Expression(expression) => {
                crate::expression(expression, None, interpreter)?;
                Ok(())
            }
        }
    })();
    while plan.activation_registration_depth() > original_scope_depth {
        plan.pop_activation_registration_scope();
    }
    symbols.borrow_mut().restore(symbol_snapshot);
    body_result?;
    let body_node_end = plan.len();
    {
        let mut plan = plan.borrow_mut();
        for node in body_node_start..body_node_end {
            for capture in captures {
                let cell = capture.committed.reactive_cell_id();
                if !plan.add_sampled_dependency(node, cell) {
                    return Err(MechError::new(
                        ActivationPatternBodyDependencyInvariant,
                        None,
                    ));
                }
            }
        }
    }
    Ok((body_node_start, body_node_end))
}

pub(super) struct Gate {
    pub(super) arm: usize,
    pub(super) selected: ValueCell,
    pub(super) captures: Vec<ActivationPatternCapture>,
    pub(super) out: ValueCell,
}

impl MechFunctionImpl for Gate {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if super::captures::read_selected_arm(&self.selected)? == self.arm {
            commit_proposed_captures(&self.captures)?;
            super::captures::increment(&self.out)?;
            Ok(ReactiveSolveStatus::Changed)
        } else {
            Ok(ReactiveSolveStatus::Unchanged)
        }
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.out))
    }

    fn reactive_output_state_ports(&self) -> Option<Vec<FunctionStatePort<'_>>> {
        Some(
            std::iter::once(FunctionStatePort::from_cell(&self.out))
                .chain(
                    self.captures
                        .iter()
                        .map(|capture| FunctionStatePort::from_cell(&capture.committed)),
                )
                .collect(),
        )
    }

    fn reactive_output_value_cells(&self) -> Vec<ValueCell> {
        std::iter::once(self.out.clone())
            .chain(
                self.captures
                    .iter()
                    .map(|capture| capture.committed.clone()),
            )
            .collect()
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(self.reactive_output_state_ports())
    }

    fn to_string(&self) -> String {
        "ActivationPatternArmGate".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for Gate {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: "Activation pattern dispatch is interpreter-only.".into(),
            },
            None,
        ))
    }
}

use crate::{
    ActivationArm, ActivationArmBody, Expression, Interpreter, InterpreterExecution, MResult,
    MechError, MechFunctionImpl, ReactiveCellId, ReactiveSolveStatus, Ref, SliceRef, Token, Value,
};
#[cfg(feature = "compiler")]
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
        .is_some_and(|value| {
            value
                .borrow()
                .reactive_root_cell_ids()
                .iter()
                .any(|cell| trigger_cells.contains(cell))
        });
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
    pulse: &Value,
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
            symbols.insert(capture.id, capture.committed.clone(), false);
            symbols
                .dictionary
                .borrow_mut()
                .insert(capture.id, capture.name.clone());
        }
    }
    let body_node_start = plan.len();
    plan.push_activation_registration_scope_with_sampled_cells(
        pulse.reactive_root_cell_ids(),
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
                let cell = capture.committed.reactive_root_cell_ids()[0];
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
    pub(super) selected: Ref<usize>,
    pub(super) captures: Vec<ActivationPatternCapture>,
    pub(super) out: Ref<usize>,
}

impl MechFunctionImpl for Gate {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        if *self.selected.borrow() == self.arm {
            commit_proposed_captures(&self.captures)?;
            *self.out.borrow_mut() += 1;
            Ok(ReactiveSolveStatus::Changed)
        } else {
            Ok(ReactiveSolveStatus::Unchanged)
        }
    }

    fn out(&self) -> Value {
        Value::Index(self.out.clone())
    }

    fn reactive_output_values(&self) -> Vec<Value> {
        let mut outputs = vec![self.out()];
        outputs.extend(
            self.captures
                .iter()
                .map(|capture| capture.committed.clone()),
        );
        outputs
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        "ActivationPatternArmGate".into()
    }
}

#[cfg(feature = "compiler")]
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

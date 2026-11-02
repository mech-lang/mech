#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{FunctionStatePort, MechFunctionImpl, Plan, TransactionStateUnsupportedError};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{CanonicalStateJournal, MResult, MechError, ValueCell};

struct TypedStateFunction {
    output: ValueCell,
    retained: ValueCell,
}

impl MechFunctionImpl for TypedStateFunction {
    fn solve_managed(
        &self,
        _frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<crate::ReactiveSolveStatus> {
        (|| -> MResult<()> { Ok(()) })()?;
        Ok(crate::ReactiveSolveStatus::Changed)
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.output))
    }

    fn reactive_output_state_ports(&self) -> Option<Vec<FunctionStatePort<'_>>> {
        let output = FunctionStatePort::from_cell(&self.output);
        Some(vec![output, output])
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![
            FunctionStatePort::from_cell(&self.output),
            FunctionStatePort::from_cell(&self.retained),
        ]))
    }

    fn to_string(&self) -> String {
        "typed-state".to_string()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TypedStateFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct UnsupportedStateFunction;

impl MechFunctionImpl for UnsupportedStateFunction {
    fn solve_managed(
        &self,
        _frame: &mut crate::KernelMemoryFrame<'_>,
        _services: &mut dyn crate::MechExecutionServices,
    ) -> MResult<crate::ReactiveSolveStatus> {
        (|| -> MResult<()> { Ok(()) })()?;
        Ok(crate::ReactiveSolveStatus::Changed)
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: "unsupported".into(),
                reason: "deliberate typed state error".into(),
            },
            None,
        ))
    }

    fn to_string(&self) -> String {
        "unsupported".to_string()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for UnsupportedStateFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn transaction_state_unsupported_error_is_structured() {
    let error = UnsupportedStateFunction.retained_state_ports().unwrap_err();
    assert_eq!(error.kind_name(), "TransactionStateUnsupported");
}

#[test]
fn typed_state_ports_report_their_logical_reactive_identity() {
    let output = ValueCell::from_exact(42usize).unwrap();
    let function = TypedStateFunction {
        output: output.clone(),
        retained: ValueCell::from_exact(7usize).unwrap(),
    };

    assert_eq!(
        function.reactive_output_cell_ids(),
        vec![output.reactive_cell_id()]
    );
}

#[test]
fn plan_checkpoint_restores_output_and_hidden_state() {
    let output = ValueCell::from_exact(1usize).unwrap();
    let output_alias = output.clone();
    let retained = ValueCell::from_exact(2usize).unwrap();
    let retained_alias = retained.clone();
    let plan = Plan::new();
    plan.add_function(crate::function::test_planned_instance(
        Box::new(TypedStateFunction {
            output: output.clone(),
            retained: retained.clone(),
        }),
        crate::FunctionInvocation::nullary(output.clone()),
    ))
    .unwrap();
    let mut journal = CanonicalStateJournal::new();

    plan.capture_transaction_state(&mut journal).unwrap();
    output
        .replace(&ValueCell::from_exact(10usize).unwrap().snapshot().unwrap())
        .unwrap();
    retained
        .replace(&ValueCell::from_exact(20usize).unwrap().snapshot().unwrap())
        .unwrap();
    journal.restore_before().unwrap();

    assert!(output.same_logical_cell(&output_alias));
    assert!(retained.same_logical_cell(&retained_alias));
    assert!(matches!(
        output_alias.snapshot().unwrap().data(),
        crate::ValueData::Index(1)
    ));
    assert!(matches!(
        retained_alias.snapshot().unwrap().data(),
        crate::ValueData::Index(2)
    ));
}

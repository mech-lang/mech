#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{FunctionStatePort, MechFunctionImpl, Plan, TransactionStateUnsupportedError};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{CanonicalStateJournal, MResult, MechError, Ref};

struct TypedStateFunction {
    output: Ref<usize>,
    retained: Ref<usize>,
}

impl MechFunctionImpl for TypedStateFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_ref(&self.output))
    }

    fn reactive_output_state_ports(&self) -> Option<Vec<FunctionStatePort<'_>>> {
        let output = FunctionStatePort::from_ref(&self.output);
        Some(vec![output, output])
    }

    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![
            FunctionStatePort::from_ref(&self.output),
            FunctionStatePort::from_ref(&self.retained),
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
    fn solve_result(&self) -> MResult<()> {
        Ok(())
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
    let output = Ref::new(42usize);
    let function = TypedStateFunction {
        output: output.clone(),
        retained: Ref::new(7),
    };

    assert_eq!(
        function.reactive_output_cell_ids(),
        vec![output.reactive_cell_id()]
    );
}

#[test]
fn plan_checkpoint_restores_output_and_hidden_state() {
    let output = Ref::new(1usize);
    let output_alias = output.clone();
    let retained = Ref::new(2usize);
    let retained_alias = retained.clone();
    let plan = Plan::new();
    plan.add_function(Box::new(TypedStateFunction {
        output: output.clone(),
        retained: retained.clone(),
    }));
    let mut journal = CanonicalStateJournal::new();

    plan.capture_transaction_state(&mut journal).unwrap();
    *output.borrow_mut() = 10;
    *retained.borrow_mut() = 20;
    journal.restore_before().unwrap();

    assert!(output.same_handle(&output_alias));
    assert!(retained.same_handle(&retained_alias));
    assert_eq!((*output.borrow(), *retained.borrow()), (1, 2));
}

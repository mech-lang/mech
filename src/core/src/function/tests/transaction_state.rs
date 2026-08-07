#[cfg(feature = "compiler")]
use super::super::MechFunctionCompiler;
use super::super::{MechFunctionImpl, Plan, TransactionStateUnsupportedError};
#[cfg(feature = "compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{MResult, MechError, Ref, ValRef, Value};

struct UnsupportedStateFunction;
impl MechFunctionImpl for UnsupportedStateFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> Value {
        Value::Empty
    }
    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: self.to_string(),
                reason: "test-only opaque state".to_string(),
            },
            None,
        ))
    }
    fn to_string(&self) -> String {
        "unsupported".to_string()
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for UnsupportedStateFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct MisleadingRuntimeHostNameFunction {
    output: ValRef,
}
impl MechFunctionImpl for MisleadingRuntimeHostNameFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> Value {
        Value::MutableReference(self.output.clone())
    }
    fn to_string(&self) -> String {
        "ExternalHostCallFunction::misleading-name".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for MisleadingRuntimeHostNameFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct UnschedulableOutputFunction {
    state: ValRef,
}
impl MechFunctionImpl for UnschedulableOutputFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> Value {
        Value::MutableReference(self.state.clone())
    }
    fn reactive_output_values(&self) -> Vec<Value> {
        Vec::new()
    }
    fn to_string(&self) -> String {
        "unschedulable-output".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for UnschedulableOutputFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn transaction_state_unsupported_error_is_structured() {
    let function = UnsupportedStateFunction;
    let error = function.transaction_state_values().unwrap_err();
    assert_eq!(error.kind_name(), "TransactionStateUnsupported");
}

#[test]
fn host_like_display_name_does_not_change_checkpoint_behavior() {
    let plan = Plan::new();
    let output = Ref::new(Value::Index(Ref::new(42)));
    plan.add_function(Box::new(MisleadingRuntimeHostNameFunction {
        output: output.clone(),
    }));

    let values = plan.transaction_state_values().unwrap();

    assert!(values.iter().any(|value| matches!(
      value,
      Value::MutableReference(cell) if cell.same_handle(&output)
    ),));
}

#[test]
fn plan_transaction_state_retains_outputs_excluded_from_scheduling() {
    let state = Ref::new(Value::Index(Ref::new(1)));
    let plan = Plan::new();
    plan.add_function(Box::new(UnschedulableOutputFunction {
        state: state.clone(),
    }));

    let values = plan.transaction_state_values().unwrap();

    assert!(values.iter().any(
        |value| matches!(value, Value::MutableReference(cell) if cell.addr() == state.addr())
    ));
}

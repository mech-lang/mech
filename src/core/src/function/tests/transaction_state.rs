#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{MechFunctionImpl, Plan, TransactionStateUnsupportedError};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{LegacyValue, MResult, MechError, Ref, ValueCell};

struct UnsupportedStateFunction;
impl MechFunctionImpl for UnsupportedStateFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Empty
    }
    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
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
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for UnsupportedStateFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct MisleadingRuntimeHostNameFunction {
    output: ValueCell,
}
impl MechFunctionImpl for MisleadingRuntimeHostNameFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::MutableReference(self.output.legacy_ref())
    }
    fn to_string(&self) -> String {
        "ExternalHostCallFunction::misleading-name".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for MisleadingRuntimeHostNameFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct UnschedulableOutputFunction {
    state: ValueCell,
}
impl MechFunctionImpl for UnschedulableOutputFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::MutableReference(self.state.legacy_ref())
    }
    fn reactive_output_values(&self) -> Vec<LegacyValue> {
        Vec::new()
    }
    fn to_string(&self) -> String {
        "unschedulable-output".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
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
    let output = ValueCell::new(LegacyValue::Index(Ref::new(42)));
    plan.add_function(Box::new(MisleadingRuntimeHostNameFunction {
        output: output.clone(),
    }));

    let values = plan.transaction_state_values().unwrap();

    let output_value = values
        .iter()
        .find_map(|value| match value {
            LegacyValue::MutableReference(cell) => Some(ValueCell::from_legacy_ref(cell.clone())),
            _ => None,
        })
        .expect("plan retains the function output");
    assert!(output_value.same_cell(&output));
}

#[test]
fn plan_transaction_state_retains_outputs_excluded_from_scheduling() {
    let state = ValueCell::new(LegacyValue::Index(Ref::new(1)));
    let plan = Plan::new();
    plan.add_function(Box::new(UnschedulableOutputFunction {
        state: state.clone(),
    }));

    let values = plan.transaction_state_values().unwrap();

    let retained_state = values
        .iter()
        .find_map(|value| match value {
            LegacyValue::MutableReference(cell) => Some(ValueCell::from_legacy_ref(cell.clone())),
            _ => None,
        })
        .expect("plan retains outputs excluded from scheduling");
    assert!(retained_state.same_cell(&state));
}

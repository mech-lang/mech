#[cfg(feature = "semantic-compiler")]
use super::super::MechFunctionCompiler;
use super::super::{FunctionStatePort, MechFunctionImpl, Plan, TransactionStateUnsupportedError};
#[cfg(feature = "semantic-compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{LegacyValue, MResult, MechError, Ref, ValueCell};
use std::{cell::Cell, rc::Rc};

struct TypedOutputFunction {
    output: Ref<usize>,
    legacy_reactive_calls: Rc<Cell<usize>>,
}

impl MechFunctionImpl for TypedOutputFunction {
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

    fn out(&self) -> LegacyValue {
        LegacyValue::Index(self.output.clone())
    }

    fn reactive_output_values(&self) -> Vec<LegacyValue> {
        self.legacy_reactive_calls
            .set(self.legacy_reactive_calls.get() + 1);
        vec![self.out()]
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(vec![self.out()])
    }

    fn to_string(&self) -> String {
        "typed-output".to_string()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for TypedOutputFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

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
fn reactive_output_identity_prefers_typed_ports_and_deduplicates_them() {
    let output = Ref::new(42usize);
    let legacy_reactive_calls = Rc::new(Cell::new(0));
    let function = TypedOutputFunction {
        output: output.clone(),
        legacy_reactive_calls: legacy_reactive_calls.clone(),
    };

    let expected = LegacyValue::Index(output).reactive_root_cell_ids();
    assert_eq!(function.reactive_output_cell_ids(), expected);
    assert_eq!(legacy_reactive_calls.get(), 0);
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

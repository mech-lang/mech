use super::snapshot_runtime_value;
use crate::runtime::RuntimeActivationEffectBarrierInvariantError;
use mech_core::{
  CompileCtx, MResult, MechError, MechFunctionCompiler, MechFunctionImpl,
  NativeFunctionCompiler, Register, Value,
};

// This name deliberately starts with a NUL byte.  It is an identifier we can
// construct in the lowered tree, but it cannot be produced by the Mech lexer.
// Keeping the compiler registered on the program is therefore not a public
// source-level API.
pub(in crate::runtime) const ACTIVATION_EFFECT_BARRIER_NAME: &str = "\0mech/runtime/activation-effect-barrier";
pub(in crate::runtime) const ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME: &str =
  "\0mech/runtime/activation-effect-payload-capture";

#[derive(Clone, Debug)]
pub(in crate::runtime) struct ActivationEffectBarrierCompiler;
impl NativeFunctionCompiler for ActivationEffectBarrierCompiler {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn mech_core::MechFunction>> {
    if !arguments.is_empty() {
      return Err(MechError::new(RuntimeActivationEffectBarrierInvariantError {
        reason: format!(
          "activation effect barrier expected no payloads, found {}",
          arguments.len()
        ),
      }, None));
    }
    Ok(Box::new(ActivationEffectBarrier))
  }
}
#[derive(Clone, Debug)]
struct ActivationEffectBarrier;
impl MechFunctionImpl for ActivationEffectBarrier {
  fn solve(&self) {}
  fn solve_reactive(&self) -> MResult<mech_core::ReactiveSolveStatus> { Ok(mech_core::ReactiveSolveStatus::Unchanged) }
  // The barrier is a scheduling node, not a value producer.  Its node id and
  // reactive execution record are the observable state used by send replay.
  fn out(&self) -> Value { Value::Empty }
  fn reactive_output_values(&self) -> Vec<Value> { Vec::new() }
  fn to_string(&self) -> String { ACTIVATION_EFFECT_BARRIER_NAME.to_string() }

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
  }
}
impl MechFunctionCompiler for ActivationEffectBarrier {
  fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> { Err(MechError::new(RuntimeActivationEffectBarrierInvariantError { reason: "activation effect barrier cannot be bytecode compiled".into() }, None)) }
}

#[derive(Clone, Debug)]
pub(in crate::runtime) struct ActivationEffectPayloadCaptureCompiler;
impl NativeFunctionCompiler for ActivationEffectPayloadCaptureCompiler {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn mech_core::MechFunction>> {
    let [payload] = arguments.as_slice() else {
      return Err(MechError::new(RuntimeActivationEffectBarrierInvariantError {
        reason: format!(
          "activation effect payload capture expected one payload, found {}",
          arguments.len()
        ),
      }, None));
    };
    Ok(Box::new(ActivationEffectPayloadCapture {
      payload: payload.clone(),
      snapshot: mech_core::Ref::new(snapshot_runtime_value(payload)),
    }))
  }
}

#[derive(Clone, Debug)]
struct ActivationEffectPayloadCapture {
  payload: Value,
  snapshot: mech_core::ValRef,
}

impl MechFunctionImpl for ActivationEffectPayloadCapture {
  fn solve(&self) {
    *self.snapshot.borrow_mut() = snapshot_runtime_value(&self.payload);
  }
  fn solve_reactive(&self) -> MResult<mech_core::ReactiveSolveStatus> {
    self.solve();
    Ok(mech_core::ReactiveSolveStatus::Unchanged)
  }
  fn out(&self) -> Value { Value::MutableReference(self.snapshot.clone()) }
  // Payloads are sampled only when the activation pulse schedules this node.
  // Their own changes must never dispatch an activation effect.
  fn reactive_dependency_scopes(&self, argument_count: usize) -> Option<Vec<mech_core::ReactiveDependencyScope>> {
    Some(vec![mech_core::ReactiveDependencyScope::None; argument_count])
  }
  fn reactive_output_values(&self) -> Vec<Value> { Vec::new() }
  fn to_string(&self) -> String { ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME.to_string() }

  fn transaction_state_values(&self) -> MResult<Vec<Value>> {
    Ok(self.reactive_output_values())
  }
}

impl MechFunctionCompiler for ActivationEffectPayloadCapture {
  fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
    Err(MechError::new(RuntimeActivationEffectBarrierInvariantError {
      reason: "activation effect payload capture cannot be bytecode compiled".into(),
    }, None))
  }
}

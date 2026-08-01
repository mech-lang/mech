use super::snapshot_runtime_value;
use crate::runtime::RuntimeActivationEffectBarrierInvariantError;
#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use mech_core::{
    FunctionSpecializer, GuardFunctionSafety, MResult, MechError, MechFunctionImpl, Value,
};

// This name deliberately starts with a NUL byte.  It is an identifier we can
// construct in the lowered tree, but it cannot be produced by the Mech lexer.
// Keeping the compiler registered on the program is therefore not a public
// source-level API.
pub(in crate::runtime) const ACTIVATION_EFFECT_BARRIER_NAME: &str =
    "\0mech/runtime/activation-effect-barrier";
pub(in crate::runtime) const ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME: &str =
    "\0mech/runtime/activation-effect-payload-capture";

#[derive(Clone, Debug)]
pub(in crate::runtime) struct ActivationEffectBarrierSpecializer;
impl FunctionSpecializer for ActivationEffectBarrierSpecializer {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn mech_core::MechFunction>> {
        if !arguments.is_empty() {
            return Err(MechError::new(
                RuntimeActivationEffectBarrierInvariantError {
                    reason: format!(
                        "activation effect barrier expected no payloads, found {}",
                        arguments.len()
                    ),
                },
                None,
            ));
        }
        Ok(Box::new(ActivationEffectBarrier))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}
#[derive(Clone, Debug)]
struct ActivationEffectBarrier;
impl MechFunctionImpl for ActivationEffectBarrier {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<mech_core::ReactiveSolveStatus> {
        Ok(mech_core::ReactiveSolveStatus::Unchanged)
    }
    // The barrier is a scheduling node, not a value producer.  Its node id and
    // reactive execution record are the observable state used by send replay.
    fn out(&self) -> Value {
        Value::Empty
    }
    fn reactive_output_values(&self) -> Vec<Value> {
        Vec::new()
    }
    fn to_string(&self) -> String {
        ACTIVATION_EFFECT_BARRIER_NAME.to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ActivationEffectBarrier {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            RuntimeActivationEffectBarrierInvariantError {
                reason: "activation effect barrier cannot be bytecode compiled".into(),
            },
            None,
        ))
    }
}

#[derive(Clone, Debug)]
pub(in crate::runtime) struct ActivationEffectPayloadCaptureSpecializer;
impl FunctionSpecializer for ActivationEffectPayloadCaptureSpecializer {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn mech_core::MechFunction>> {
        let [payload] = arguments else {
            return Err(MechError::new(
                RuntimeActivationEffectBarrierInvariantError {
                    reason: format!(
                        "activation effect payload capture expected one payload, found {}",
                        arguments.len()
                    ),
                },
                None,
            ));
        };
        Ok(Box::new(ActivationEffectPayloadCapture {
            payload: payload.clone(),
            snapshot: mech_core::Ref::new(snapshot_runtime_value(payload)?),
        }))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}

#[derive(Clone, Debug)]
struct ActivationEffectPayloadCapture {
    payload: Value,
    snapshot: mech_core::ValRef,
}

impl ActivationEffectPayloadCapture {
    fn update_snapshot(&self) -> MResult<()> {
        let snapshot = snapshot_runtime_value(&self.payload)?;
        *self.snapshot.borrow_mut() = snapshot;
        Ok(())
    }
}

impl MechFunctionImpl for ActivationEffectPayloadCapture {
    fn solve(&self) {
        if let Err(error) = self.update_snapshot() {
            eprintln!(
                "[Mech Runtime Activation Error] payload snapshot failed during solve; preserving previous payload: {:?}",
                error,
            );
        }
    }
    fn solve_result(&self) -> MResult<()> {
        self.update_snapshot()
    }
    fn solve_reactive(&self) -> MResult<mech_core::ReactiveSolveStatus> {
        self.update_snapshot()?;
        Ok(mech_core::ReactiveSolveStatus::Unchanged)
    }
    fn out(&self) -> Value {
        Value::MutableReference(self.snapshot.clone())
    }
    // Payloads are sampled only when the activation pulse schedules this node.
    // Their own changes must never dispatch an activation effect.
    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<mech_core::ReactiveDependencyScope>> {
        Some(vec![
            mech_core::ReactiveDependencyScope::None;
            argument_count
        ])
    }
    fn reactive_output_values(&self) -> Vec<Value> {
        Vec::new()
    }
    fn to_string(&self) -> String {
        ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME.to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ActivationEffectPayloadCapture {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            RuntimeActivationEffectBarrierInvariantError {
                reason: "activation effect payload capture cannot be bytecode compiled".into(),
            },
            None,
        ))
    }
}

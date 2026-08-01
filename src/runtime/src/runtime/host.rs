// Host Calls
// -----------------------------------------------------------------------------

// This file defines the logic for handling host calls in the Mech runtime. Host calls are a mechanism for Mech programs to interact with the host environment, allowing them to call functions that are implemented outside of the Mech program itself, typically in Rust. This is a crucial part of the runtime, as it enables Mech to be extended and embedded in other programming environments.

// The runtime provides the following host methods:

// - `MechRuntimeBuilder::host_function`: Registers a planned host function
//   before the runtime is built.
// - `call_host`: Executes a host call by name with the provided arguments. It emits events for the start, completion, and failure of the host call, allowing for observability of host interactions. It also checks the host policy to ensure that the call is allowed and charges the appropriate costs based on the function's estimated cost. A version of the function that accepts a MechRuntimeContext is also provided.

// Furthermore, this file defines two structs:

// `RuntimeHostFunctionSpecializer`, which installs host functions as
// program-local extensions that can be called directly from Mech code. The
// `RuntimeHostNativeFunction` struct represents a specialized host function
// that can be executed within the Mech program.

// For example, a function to compute an affine transformation could be registered as a host function, and then called from Mech code like this:
/*
  let runtime = MechRuntime::builder().host_function(DeterministicHostFunction::new(
    "demo/math/affine",
    |_context, _args| Ok(value_f64(0.0)),
    |_context, args| {
      host_call3(
        "demo/math/affine",
        &args,
        |x: f64, scale: f64, offset: f64| {
          (x * scale) + offset
        },
      )
    },
  ))?.build()?;
*/
// Then in Mech:
/*
  result := demo/math/affine(2.0, 3.0, 4.0);
*/

use super::execution::{
    ACTIVATION_EFFECT_BARRIER_NAME, ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
    ActivationEffectBarrierSpecializer, ActivationEffectPayloadCaptureSpecializer,
};
use super::extension::invoke_extension;
use super::{MechRuntime, RuntimeHostFunctionNotBytecodeCompilableError};
use crate::{
    HostCall, HostFunctionNotFoundError, RegisteredHostFunction, RuntimeCallContext,
    RuntimeContext, RuntimeEventKind, RuntimeValueSnapshot,
};
#[cfg(feature = "compiler")]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};
use mech_core::{
    FunctionSpecializer, GuardFunctionSafety, MResult, MechError, MechErrorKind,
    MechExecutionServices, MechFunctionImpl, Ref, Value, ValueKind,
};
use mech_engine::MechProgram;
use std::sync::Arc;

impl MechRuntime {
    fn install_runtime_program_host_extensions(
        program: &mut MechProgram,
        context: RuntimeCallContext,
        functions: Vec<RegisteredHostFunction>,
    ) -> MResult<()> {
        program.register_function_extension(
            ACTIVATION_EFFECT_BARRIER_NAME,
            Arc::new(ActivationEffectBarrierSpecializer),
        )?;
        program.register_function_extension(
            ACTIVATION_EFFECT_PAYLOAD_CAPTURE_NAME,
            Arc::new(ActivationEffectPayloadCaptureSpecializer),
        )?;
        for function in functions {
            let name = function.name().to_string();
            program.register_function_extension(
                name.clone(),
                Arc::new(RuntimeHostFunctionSpecializer::new(
                    name.clone(),
                    name,
                    context.clone(),
                    function,
                )),
            )?;
        }
        Ok(())
    }

    pub(super) fn register_retained_program_host_functions(
        &mut self,
        context: &RuntimeContext,
    ) -> MResult<()> {
        let functions = self.registered_host_functions()?;
        Self::install_runtime_program_host_extensions(
            &mut self.program,
            RuntimeCallContext::capture(context),
            functions,
        )
    }

    pub(super) fn register_runtime_program_host_functions(
        &mut self,
        context: &mut RuntimeContext,
        program: &mut MechProgram,
    ) -> MResult<()> {
        let functions = self.registered_host_functions()?;
        Self::install_runtime_program_host_extensions(
            program,
            RuntimeCallContext::capture(context),
            functions,
        )
    }

    fn registered_host_functions(&self) -> MResult<Vec<RegisteredHostFunction>> {
        self.host_registry
            .list_functions()?
            .into_iter()
            .map(|name| {
                self.host_registry
                    .get_function(&name)?
                    .ok_or_else(|| MechError::new(HostFunctionNotFoundError { name }, None))
            })
            .collect()
    }

    pub fn call_host(&mut self, call: HostCall) -> MResult<RuntimeValueSnapshot> {
        let mut context = self.runtime_context()?;
        self.call_host_with_context(&mut context, call)
    }

    pub fn call_host_with_context(
        &mut self,
        context: &mut RuntimeContext,
        call: HostCall,
    ) -> MResult<RuntimeValueSnapshot> {
        self.call_host_with_context_map(context, call, |value| {
            RuntimeValueSnapshot::try_capture(&value)
        })
    }

    pub(crate) fn call_host_value_with_context(
        &mut self,
        context: &mut RuntimeContext,
        call: HostCall,
    ) -> MResult<Value> {
        self.call_host_with_context_map(context, call, Ok)
    }

    fn call_host_with_context_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        call: HostCall,
        finish: impl FnOnce(Value) -> MResult<T>,
    ) -> MResult<T> {
        self.ensure_runtime_mutation_allowed("call_host_with_context")?;
        self.validate_context_for_runtime(context)?;
        call.validate()?;
        let implicit = context.transaction.is_none();
        let transaction_id = if implicit {
            Some(self.begin_transaction(context)?)
        } else {
            context.transaction
        };
        let result = self
            .with_runtime_execution_session(context, |session| {
                session.invoke_native(&call.name, &call.args)
            })
            .and_then(finish);
        if !implicit {
            return result;
        }
        // The implicit transaction owns the provisional host events. Preserve
        // only the failure audit after rollback; completed calls still publish
        // their events through the normal commit path.
        let failed_host_audit = if result.is_err() {
            // The context event vector is retention-bounded and can discard newly
            // emitted events from the front without changing its length. The
            // transaction journal retains the complete provisional host audit.
            transaction_id
                .and_then(|transaction_id| self.active_transactions.get(&transaction_id))
                .map(|transaction| {
                    transaction
                        .store
                        .staged_events()
                        .filter_map(|event| match &event.kind {
                            RuntimeEventKind::HostCallStarted { .. }
                            | RuntimeEventKind::HostCallFailed { .. } => Some(event.kind.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        match result {
            Ok(value) => {
                self.commit_runtime_transaction(context)?;
                Ok(value)
            }
            Err(error) => {
                let original = format!("{error:?}");
                match self.abort_runtime_transaction_with_recovered_events(
                    context,
                    format!("host call `{}` failed", call.name),
                    failed_host_audit,
                ) {
                    Ok(()) => Err(error),
                    Err(cleanup_error) => Err(self.poison_program_operation(
                        "call_host_with_context",
                        transaction_id,
                        original,
                        vec![format!(
                            "implicit host transaction cleanup failed: {cleanup_error:?}",
                        )],
                    )),
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeHostFunctionSpecializer {
    pub mech_name: String,
    pub host_name: String,
    pub context: RuntimeCallContext,
    pub function: RegisteredHostFunction,
}

impl RuntimeHostFunctionSpecializer {
    pub fn new(
        mech_name: impl Into<String>,
        host_name: impl Into<String>,
        context: RuntimeCallContext,
        function: RegisteredHostFunction,
    ) -> Self {
        Self {
            mech_name: mech_name.into(),
            host_name: host_name.into(),
            context,
            function,
        }
    }
}

impl FunctionSpecializer for RuntimeHostFunctionSpecializer {
    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }

    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn mech_core::MechFunction>> {
        let argument_snapshots = arguments
            .iter()
            .map(RuntimeValueSnapshot::try_capture)
            .collect::<MResult<Vec<_>>>()?;
        let planned = invoke_extension(
            format!("host function `{}`", self.host_name),
            "plan",
            || self.function.plan(&self.context, &argument_snapshots),
        )?;
        Ok(Box::new(RuntimeHostNativeFunction {
            name: self.mech_name.clone(),
            host_name: self.host_name.clone(),
            arguments: arguments.to_vec(),
            value: Ref::new(planned.into_value().try_deep_snapshot()?),
        }))
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeHostNativeFunction {
    pub name: String,
    pub host_name: String,
    pub arguments: Vec<Value>,
    pub value: Ref<Value>,
}

#[derive(Debug, Clone)]
pub struct RuntimeHostOutputUpdateError {
    pub function: String,
    pub expected: ValueKind,
    pub actual: ValueKind,
    pub reason: String,
}
impl MechErrorKind for RuntimeHostOutputUpdateError {
    fn name(&self) -> &str {
        "RuntimeHostOutputUpdateError"
    }
    fn message(&self) -> String {
        format!(
            "host function `{}` returned unsupported or incompatible output kind {:?}; expected {:?}: {}",
            self.function, self.actual, self.expected, self.reason,
        )
    }
}

impl RuntimeHostNativeFunction {
    fn update_output(&self, next: Value) -> MResult<()> {
        let expected = self.value.borrow().kind();
        if expected == ValueKind::Empty {
            *self.value.borrow_mut() = next.try_deep_snapshot()?;
            return Ok(());
        }
        let next = next.try_deep_snapshot()?;
        let actual = next.kind();
        mech_engine::apply_stable_value_update(self.value.clone(), next)
            .map(|_| ())
            .map_err(|error| {
                MechError::new(
                    RuntimeHostOutputUpdateError {
                        function: self.name.clone(),
                        expected,
                        actual,
                        reason: format!("{error:?}"),
                    },
                    None,
                )
            })
    }

    fn solve_inner(&self, services: &mut dyn mech_core::MechExecutionServices) -> MResult<()> {
        let next = services.invoke_native(&self.host_name, &self.arguments)?;
        self.update_output(next)
    }
}

impl MechFunctionImpl for RuntimeHostNativeFunction {
    fn solve(&self) {
        let mut services = mech_core::NoMechExecutionServices;
        if let Err(error) = self.solve_inner(&mut services) {
            eprintln!(
                "[Mech Runtime Host Error] function `{}` failed during solve; preserving previous output: {:?}",
                self.name, error,
            );
        }
    }

    fn solve_result(&self) -> MResult<()> {
        let mut services = mech_core::NoMechExecutionServices;
        self.solve_inner(&mut services)
    }

    fn solve_result_with(
        &self,
        services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<()> {
        self.solve_inner(services)
    }

    fn solve_reactive_with(
        &self,
        services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        self.solve_inner(services)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn out(&self) -> Value {
        self.value.borrow().clone()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::MutableReference(self.value.clone())])
    }

    fn to_string(&self) -> String {
        format!("RuntimeHostNativeFunction::{}", self.name)
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for RuntimeHostNativeFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            RuntimeHostFunctionNotBytecodeCompilableError {
                function: self.name.clone(),
            },
            None,
        ))
    }
}

#[cfg(test)]
#[path = "host/tests/transaction/mod.rs"]
mod transaction_tests;

#[cfg(test)]
#[path = "host/tests/checkpoint.rs"]
mod checkpoint_tests;

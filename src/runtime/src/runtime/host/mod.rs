// Host Calls
// -----------------------------------------------------------------------------

// This file defines the logic for handling host calls in the Mech runtime. Host calls are a mechanism for Mech programs to interact with the host environment, allowing them to call functions that are implemented outside of the Mech program itself, typically in Rust. This is a crucial part of the runtime, as it enables Mech to be extended and embedded in other programming environments.

// The runtime provides the following host methods:

// - `MechRuntimeBuilder::host_function`: Registers a planned host function
//   before the runtime is built.
// - `call_host`: Executes a host call by name with the provided arguments. It emits events for the start, completion, and failure of the host call, allowing for observability of host interactions. It also checks the host policy to ensure that the call is allowed and charges the appropriate costs based on the function's estimated cost. A version of the function that accepts a MechRuntimeContext is also provided.

// Furthermore, this file defines two structs:

// `RuntimeHostFunctionSpecializer`, which installs host functions as
// program-local extensions that can be called directly from Mech code.

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

use super::extension::invoke_extension;
use super::{MechRuntime, RuntimeExecutionMode};
use crate::{
    HostCall, HostFunctionNotFoundError, RegisteredHostFunction, RuntimeCallContext,
    RuntimeContext, RuntimeEventKind, RuntimeValueSnapshot,
};
use mech_core::{
    ExecutionHostFunctionRequest, FunctionSpecializer, GuardFunctionSafety, InitialSolvePolicy,
    MResult, MechError, MechExecutionServices, Ref, Value,
};
use mech_engine::{ExternalHostCallFunction, MechProgram};
use std::sync::Arc;

impl MechRuntime {
    fn install_runtime_program_host_extensions(
        program: &mut MechProgram,
        context: RuntimeCallContext,
        functions: Vec<RegisteredHostFunction>,
        execution_mode: RuntimeExecutionMode,
    ) -> MResult<()> {
        for function in functions {
            let name = function.name().to_string();
            program.register_function_extension(
                name.clone(),
                Arc::new(RuntimeHostFunctionSpecializer::new(
                    name,
                    context.clone(),
                    function,
                    execution_mode,
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
        let execution_mode = self.execution_mode;
        Self::install_runtime_program_host_extensions(
            &mut self.program,
            RuntimeCallContext::capture(context),
            functions,
            execution_mode,
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
            self.execution_mode,
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
                session.invoke_host_function(
                    &ExecutionHostFunctionRequest {
                        name: call.name.clone(),
                    },
                    &call.args,
                )
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
    pub host_name: String,
    pub context: RuntimeCallContext,
    pub function: RegisteredHostFunction,
    pub execution_mode: RuntimeExecutionMode,
}

impl RuntimeHostFunctionSpecializer {
    pub fn new(
        host_name: impl Into<String>,
        context: RuntimeCallContext,
        function: RegisteredHostFunction,
        execution_mode: RuntimeExecutionMode,
    ) -> Self {
        Self {
            host_name: host_name.into(),
            context,
            function,
            execution_mode,
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
        Ok(Box::new(ExternalHostCallFunction {
            request: ExecutionHostFunctionRequest {
                name: self.host_name.clone(),
            },
            arguments: arguments.to_vec(),
            output: Ref::new(planned.into_value().try_deep_snapshot()?),
            initial_solve_policy: match self.execution_mode {
                RuntimeExecutionMode::Execute => InitialSolvePolicy::Solve,
                RuntimeExecutionMode::Plan => InitialSolvePolicy::PreserveSpecializedOutput,
            },
        }))
    }
}

#[cfg(test)]
#[path = "tests/transaction/mod.rs"]
mod transaction_tests;

#[cfg(test)]
#[path = "tests/checkpoint.rs"]
mod checkpoint_tests;

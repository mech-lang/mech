// Host Calls
// -----------------------------------------------------------------------------

// This file defines the logic for handling host calls in the Mech runtime. Host calls are a mechanism for Mech programs to interact with the host environment, allowing them to call functions that are implemented outside of the Mech program itself, typically in Rust. This is a crucial part of the runtime, as it enables Mech to be extended and embedded in other programming environments.

// The runtime provides the following host methods:

// - `MechRuntimeBuilder::host_function`: Registers a planned host function
//   before the runtime is built.
// - `call_host`: Executes a host call by name with the provided arguments. It emits events for the start, completion, and failure of the host call, allowing for observability of host interactions. It also checks the host policy to ensure that the call is allowed and charges the appropriate costs based on the function's estimated cost. A version of the function that accepts a MechRuntimeContext is also provided.

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

use super::MechRuntime;
use crate::{HostCall, RuntimeContext, RuntimeEventKind, RuntimeValueSnapshot};
use mech_core::{ExecutionHostFunctionRequest, LegacyValue, MResult, MechExecutionServices};

impl MechRuntime {
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

    fn call_host_with_context_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        call: HostCall,
        finish: impl FnOnce(LegacyValue) -> MResult<T>,
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
                    Err(cleanup_error) => Err(self.poison_runtime_operation(
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

#[cfg(all(test, feature = "source"))]
#[path = "tests/transaction/mod.rs"]
mod transaction_tests;

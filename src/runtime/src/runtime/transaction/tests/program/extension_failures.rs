use super::support::invoke_host_callback;
use crate::capability::{BasicCapability, BasicOperation, BasicResource, BasicSubject};
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    CapabilityId, PlannedRuntimeManagedHostFunction, RuntimeInvalidOperationError,
    RuntimeValueSnapshot,
};
use mech_core::{MResult, MechError, MechSourceCode};
use std::sync::Arc;

#[test]
fn host_callback_failure_cannot_escape_execution_session() {
    let mut runtime = test_runtime_builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "demo/reenter",
            |_context, _args| Ok(RuntimeValueSnapshot::empty()),
            move |_services, _context, _args| {
                Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "demo/reenter",
                        reason: "deliberate execution-session failure".to_string(),
                    },
                    None,
                ))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(500),
            &BasicSubject::new(&subject),
            &BasicResource::new("host:demo/reenter"),
            [BasicOperation::new("call")],
        )))
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let operation: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "host_callback_failure_test",
        |runtime, context| {
            runtime.program.run_source(&MechSourceCode::String(
                "reentrant-result := 0.0".to_string(),
            ))?;
            invoke_host_callback(runtime, context, "demo/reenter")?;
            Ok(())
        },
    );
    let outer_error = operation.unwrap_err();

    assert_eq!(outer_error.kind_name(), "RuntimeInvalidOperation");
    assert!(
        runtime
            .program
            .root_symbol_value("reentrant-result")
            .is_err()
    );
    assert!(runtime.active_transactions.is_empty());
}

use crate::capability::{BasicCapability, BasicOperation, BasicResource, BasicSubject};
use crate::{
    CapabilityId, MechRuntime, PlannedRuntimeManagedHostFunction, RuntimeInvalidOperationError,
    RuntimeValueSnapshot,
};
use mech_core::MechError;
use std::sync::Arc;

#[test]
fn host_callback_failure_cannot_escape_execution_session() {
    let mut runtime = MechRuntime::builder()
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

    let outer_error = runtime
        .run_string_with_context(&mut context, "reentrant-result := demo/reenter()")
        .unwrap_err();

    assert_eq!(outer_error.kind_name(), "RuntimeInvalidOperation");
    assert!(
        runtime
            .program
            .root_symbol_value("reentrant-result")
            .is_err()
    );
    assert!(runtime.active_transactions.is_empty());
}

use crate::runtime::test_support::capabilities::grant_host_call;
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    CapabilityId, HostCall, MechRuntime, PlannedPureHostFunction,
    PlannedRuntimeManagedHostFunction, PlannedStagedHostFunction, RuntimeHealth,
    RuntimeInvalidOperationError, RuntimeValueSnapshot,
};
use mech_core::MechError;

fn scalar_snapshot(value: f64) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::from_value(
        crate::RuntimeHostInputValue::F64(value)
            .into_value()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn pure_host_panic_is_contained_without_poisoning_runtime() {
    let mut runtime = test_runtime_builder()
        .host_function(PlannedPureHostFunction::new(
            "sealed/pure-panic",
            |_context, _arguments| Ok(scalar_snapshot(1.0)),
            |_context, _arguments| {
                panic!("deliberate pure host panic");
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "sealed/pure-panic");

    let error = runtime
        .call_host(HostCall::new("sealed/pure-panic", Vec::new()))
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate pure host panic"));
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

#[test]
fn runtime_managed_host_panic_is_an_ordinary_rollback_failure() {
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "sealed/managed-panic",
            |_context, _arguments| Ok(scalar_snapshot(1.0)),
            |_services, _context, _arguments| {
                panic!("deliberate runtime-managed host panic");
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "sealed/managed-panic");

    let error = runtime
        .call_host(HostCall::new("sealed/managed-panic", Vec::new()))
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate runtime-managed host panic"));
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

#[test]
fn runtime_managed_host_error_stays_contained_and_cleans_transaction_state() {
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "sealed/managed-error",
            |_context, _arguments| Ok(scalar_snapshot(1.0)),
            |_services, _context, _arguments| {
                Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "sealed/managed-error",
                        reason: "deliberate execution-session failure".to_string(),
                    },
                    None,
                ))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "sealed/managed-error");

    let error = runtime
        .call_host(HostCall::new("sealed/managed-error", Vec::new()))
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeInvalidOperation");
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

#[test]
fn staged_host_prepare_panic_stages_no_effect() {
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedStagedHostFunction::new(
            "sealed/staged-panic",
            |_context, _arguments| Ok(scalar_snapshot(1.0)),
            |_context, _arguments| {
                panic!("deliberate staged host prepare panic");
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "sealed/staged-panic");

    let error = runtime
        .call_host(HostCall::new("sealed/staged-panic", Vec::new()))
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate staged host prepare panic"));
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

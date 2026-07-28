use std::sync::{Arc, Mutex};

use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    CapabilityId, MechRuntime, PlannedPureHostFunction, PlannedRuntimeManagedHostFunction,
    PlannedStagedHostFunction, PreparedRuntimeEffect, RuntimeCallContext, RuntimeHealth,
    RuntimePreparedHostCall, RuntimeValueSnapshot,
};
use mech_core::{Ref, Value};

use super::support::RecordingHostEffect;

#[test]
fn failed_later_operation_discards_only_its_staged_host_effect() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedStagedHostFunction::new(
            "demo/staged",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(Value::String(Ref::new("provisional".to_string())).into())
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                Ok(RuntimePreparedHostCall {
                    value: Value::String(Ref::new("provisional".to_string())).into(),
                    effect: PreparedRuntimeEffect::AfterCommit(Box::new(RecordingHostEffect {
                        log: effect_log.clone(),
                        entry: "delivered".to_string(),
                    })),
                })
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "demo/staged");
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_string_with_context(&mut context, "first := demo/staged()")
        .unwrap();
    let failed = runtime.run_string_with_context(
        &mut context,
        "discarded := demo/staged()\nbroken := missing + 1",
    );

    assert!(failed.is_err());
    assert!(runtime.program.root_symbol_value("first").is_ok());
    assert!(runtime.program.root_symbol_value("discarded").is_err());
    assert!(log.lock().unwrap().is_empty());

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(log.lock().unwrap().as_slice(), &["delivered".to_string()],);
}

#[test]
fn pure_host_panic_rolls_back_and_restores_program_and_guard() {
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedPureHostFunction::new(
            "sealed/pure-panic",
            |_context, _arguments| Ok(Value::F64(Ref::new(1.0)).into()),
            |_context, _arguments| {
                panic!("deliberate pure host panic");
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "sealed/pure-panic");
    runtime.run_string("panic-anchor := 1.0").unwrap();

    let error = runtime
        .run_string("discarded := sealed/pure-panic()")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate pure host panic"));
    assert!(runtime.program.root_symbol_value("panic-anchor").is_ok());
    assert!(runtime.program.root_symbol_value("discarded").is_err());
    assert!(runtime.active_program_operation.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

#[test]
fn runtime_managed_host_panic_is_an_ordinary_rollback_failure() {
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "sealed/managed-panic",
            |_context, _arguments| Ok(Value::F64(Ref::new(1.0)).into()),
            |_services, _context, _arguments| {
                panic!("deliberate runtime-managed host panic");
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "sealed/managed-panic");

    let error = runtime
        .run_string("discarded := sealed/managed-panic()")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate runtime-managed host panic"));
    assert!(runtime.active_program_operation.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

#[test]
fn staged_host_prepare_panic_stages_no_effect() {
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedStagedHostFunction::new(
            "sealed/staged-panic",
            |_context, _arguments| Ok(Value::F64(Ref::new(1.0)).into()),
            |_context, _arguments| {
                panic!("deliberate staged host prepare panic");
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "sealed/staged-panic");

    let error = runtime
        .run_string("discarded := sealed/staged-panic()")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate staged host prepare panic"));
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::runtime::test_support::capabilities::grant_host_call;
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    CapabilityId, HostCall, ObjectRecord, PlannedPureHostFunction,
    PlannedRuntimeManagedHostFunction, PlannedStagedHostFunction, PreparedRuntimeEffect,
    RuntimeCallContext, RuntimePreparedHostCall, RuntimeValueSnapshot,
};
use mech_core::{LegacyValue, Ref};

use super::support::CountingAfterCommitEffect;

fn snapshot(value: LegacyValue) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[test]
fn planning_never_invokes_a_host_callback() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let callback_invocations = invocations.clone();
    let mut runtime = test_runtime_builder()
        .planning()
        .host_function(PlannedPureHostFunction::new(
            "demo/plan-only",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(RuntimeValueSnapshot::empty())
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_invocations.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::empty())
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(699), "demo/plan-only");

    let value = runtime
        .call_host(HostCall::new("demo/plan-only", Vec::new()))
        .unwrap();

    assert_eq!(invocations.load(Ordering::SeqCst), 0);
    assert_eq!(value.to_value(), LegacyValue::Empty);
}

#[test]
fn planning_runtime_calls_host_plan_without_preparing_or_delivering_an_effect() {
    let plans = Arc::new(AtomicUsize::new(0));
    let prepares = Arc::new(AtomicUsize::new(0));
    let deliveries = Arc::new(AtomicUsize::new(0));
    let plan_count = Arc::clone(&plans);
    let prepare_count = Arc::clone(&prepares);
    let effect_deliveries = Arc::clone(&deliveries);
    let mut runtime = test_runtime_builder()
        .planning()
        .host_function(PlannedStagedHostFunction::new(
            "demo/planning-staged",
            move |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                plan_count.fetch_add(1, Ordering::SeqCst);
                Ok(snapshot(LegacyValue::F64(Ref::new(0.0))))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                prepare_count.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimePreparedHostCall {
                    value: snapshot(LegacyValue::F64(Ref::new(7.0))),
                    effect: PreparedRuntimeEffect::AfterCommit(Box::new(
                        CountingAfterCommitEffect {
                            deliveries: Arc::clone(&effect_deliveries),
                        },
                    )),
                })
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(701), "demo/planning-staged");

    let value = runtime
        .call_host(HostCall::new("demo/planning-staged", Vec::new()))
        .unwrap();

    assert_eq!(value.to_value(), LegacyValue::F64(Ref::new(0.0)));
    assert_eq!(plans.load(Ordering::SeqCst), 1);
    assert_eq!(prepares.load(Ordering::SeqCst), 0);
    assert_eq!(deliveries.load(Ordering::SeqCst), 0);
}

#[test]
fn runtime_managed_source_planning_does_not_stage_mutation() {
    let observed_ids = Arc::new(Mutex::new(Vec::new()));
    let callback_ids = observed_ids.clone();
    let mut runtime = test_runtime_builder()
        .planning()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "demo/runtime-managed",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(LegacyValue::String(Ref::new(
                    "planned".to_string(),
                ))))
            },
            move |services, _context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                let id = services.allocate_object_id()?;
                callback_ids.lock().unwrap().push(id);
                services.put_object(ObjectRecord::text(id, "preview-test", "value"))?;
                Ok(snapshot(LegacyValue::String(Ref::new(id.to_string()))))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "demo/runtime-managed");

    let result = runtime
        .call_host(HostCall::new("demo/runtime-managed", Vec::new()))
        .unwrap();

    assert!(observed_ids.lock().unwrap().is_empty());
    assert_eq!(
        result.to_value(),
        LegacyValue::String(Ref::new("planned".to_string())),
    );
}

#[test]
fn host_planning_panics_are_converted_without_invocation() {
    let plan_calls = Arc::new(AtomicUsize::new(0));
    let invoke_calls = Arc::new(AtomicUsize::new(0));
    let plan_count = plan_calls.clone();
    let invoke_count = invoke_calls.clone();
    let mut runtime = test_runtime_builder()
        .planning()
        .host_function(PlannedPureHostFunction::new(
            "sealed/plan-panic",
            move |_context, _arguments| {
                plan_count.fetch_add(1, Ordering::SeqCst);
                panic!("deliberate host plan panic");
            },
            move |_context, _arguments| {
                invoke_count.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::empty())
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(702), "sealed/plan-panic");

    let error = runtime
        .call_host(HostCall::new("sealed/plan-panic", Vec::new()))
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate host plan panic"));
    assert_eq!(plan_calls.load(Ordering::SeqCst), 1);
    assert_eq!(invoke_calls.load(Ordering::SeqCst), 0);
}

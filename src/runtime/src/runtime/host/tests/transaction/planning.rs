use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::runtime::host::RuntimeHostNativeFunctionCompiler;
use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    CapabilityId, MechRuntime, ObjectRecord, PlannedPureHostFunction,
    PlannedRuntimeManagedHostFunction, RuntimeCallContext, RuntimeValueSnapshot,
};
use mech_core::{NativeFunctionCompiler, Ref, Value};

fn snapshot(value: Value) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[test]
fn planned_pure_host_runs_inside_implicit_and_explicit_transactions() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let runtime = MechRuntime::builder()
        .host_function(PlannedPureHostFunction::new(
            "demo/pure",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(Value::F64(Ref::new(42.0))))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(snapshot(Value::F64(Ref::new(42.0))))
            },
        ))
        .unwrap();
    let mut runtime = runtime.build().unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "demo/pure");

    runtime.run_string("implicit := demo/pure()").unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .run_string_with_context(&mut context, "explicit := demo/pure()")
        .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn planning_never_invokes_a_host_callback() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let callback_invocations = invocations.clone();
    let runtime = MechRuntime::builder()
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

    assert_eq!(invocations.load(Ordering::SeqCst), 0);
    assert!(runtime.program.root_symbol_value("missing").is_err());
}

#[test]
fn runtime_managed_planning_does_not_duplicate_staged_mutation() {
    let observed_ids = Arc::new(Mutex::new(Vec::new()));
    let callback_ids = observed_ids.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "demo/runtime-managed",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(Value::String(Ref::new("planned".to_string()))))
            },
            move |services, _context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                let id = services.allocate_object_id()?;
                callback_ids.lock().unwrap().push(id);
                services.put_object(ObjectRecord::text(id, "preview-test", "value"))?;
                Ok(snapshot(Value::String(Ref::new(id.to_string()))))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(700), "demo/runtime-managed");

    runtime
        .run_string("result := demo/runtime-managed()")
        .unwrap();

    let ids = observed_ids.lock().unwrap().clone();
    assert_eq!(ids.len(), 1);
    assert!(runtime.store().get_object(ids[0]).unwrap().is_some());
}

#[test]
fn host_planning_panics_are_converted_without_invocation() {
    let plan_calls = Arc::new(AtomicUsize::new(0));
    let invoke_calls = Arc::new(AtomicUsize::new(0));
    let plan_count = plan_calls.clone();
    let invoke_count = invoke_calls.clone();
    let runtime = MechRuntime::builder().build().unwrap();
    let context = RuntimeCallContext::capture(&runtime.runtime_context().unwrap());
    let compiler = RuntimeHostNativeFunctionCompiler::new(
        "sealed/plan-panic",
        "sealed/plan-panic",
        context,
        PlannedPureHostFunction::new(
            "sealed/plan-panic",
            move |_context, _arguments| {
                plan_count.fetch_add(1, Ordering::SeqCst);
                panic!("deliberate host plan panic");
            },
            move |_context, _arguments| {
                invoke_count.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::empty())
            },
        )
        .into(),
    );

    let error = match compiler.compile(&Vec::new()) {
        Ok(_) => panic!("planning panic should be converted to an error"),
        Err(error) => error,
    };

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate host plan panic"));
    assert_eq!(plan_calls.load(Ordering::SeqCst), 1);
    assert_eq!(invoke_calls.load(Ordering::SeqCst), 0);
}

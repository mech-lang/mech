use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::runtime::test_support::capabilities::grant_host_call_with_limit;
use crate::{
    CapabilityId, HostCall, MechRuntime, PlannedPureHostFunction,
    PlannedRuntimeManagedHostFunction, PlannedStagedHostFunction, PreparedRuntimeEffect,
    RuntimeCallContext, RuntimePreparedHostCall, RuntimeValueSnapshot,
};
use mech_core::{Ref, Value};

use super::support::{CountingAfterCommitEffect, PreviewUnsupportedCapability};

#[test]
fn pure_host_planning_does_not_consume_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedPureHostFunction::new(
            "demo/pure-limited",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(Value::F64(Ref::new(1.0)).into())
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::F64(Ref::new(1.0)).into())
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call_with_limit(&mut runtime, CapabilityId(710), "demo/pure-limited", 1);

    runtime
        .run_string("pure-limited-result := demo/pure-limited()")
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(runtime
        .call_host(HostCall::new("demo/pure-limited", Vec::new()))
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn runtime_managed_planning_does_not_consume_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "demo/managed-limited",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(Value::F64(Ref::new(1.0)).into())
            },
            move |_services, _context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::F64(Ref::new(1.0)).into())
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call_with_limit(&mut runtime, CapabilityId(711), "demo/managed-limited", 1);

    runtime
        .run_string("managed-limited-result := demo/managed-limited()")
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(runtime
        .call_host(HostCall::new("demo/managed-limited", Vec::new()))
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn staged_planning_does_not_consume_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let deliveries = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let delivered = deliveries.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedStagedHostFunction::new(
            "demo/staged-limited",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(Value::F64(Ref::new(1.0)).into())
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                let delivered = delivered.clone();
                Ok(RuntimePreparedHostCall {
                    value: Value::F64(Ref::new(1.0)).into(),
                    effect: PreparedRuntimeEffect::AfterCommit(Box::new(
                        CountingAfterCommitEffect {
                            deliveries: delivered,
                        },
                    )),
                })
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call_with_limit(&mut runtime, CapabilityId(712), "demo/staged-limited", 1);

    runtime
        .run_string("staged-limited-result := demo/staged-limited()")
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(deliveries.load(Ordering::SeqCst), 1);
    assert!(runtime
        .call_host(HostCall::new("demo/staged-limited", Vec::new()))
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn custom_capability_without_preview_contract_fails_closed() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedPureHostFunction::new(
            "demo/unsupported-preview",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| Ok(Value::Empty.into()),
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Empty.into())
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(PreviewUnsupportedCapability {
            id: CapabilityId(713),
            subject,
            resource: "host:demo/unsupported-preview".to_string(),
        }))
        .unwrap();
    let error = runtime
        .run_string("unsupported-preview-result := demo/unsupported-preview()")
        .unwrap_err();

    assert_eq!(error.kind_name(), "TransactionStateUnsupported");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

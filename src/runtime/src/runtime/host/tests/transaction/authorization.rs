use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::support::{CountingAfterCommitEffect, PreviewUnsupportedCapability};
use crate::runtime::test_support::capabilities::grant_host_call_with_limit;
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    CapabilityId, HostCall, MechRuntime, PlannedPureHostFunction,
    PlannedRuntimeManagedHostFunction, PlannedStagedHostFunction, PreparedRuntimeEffect,
    RuntimeCallContext, RuntimePreparedHostCall, RuntimeValueSnapshot,
};

fn scalar_snapshot(value: f64) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::from_value(
        crate::RuntimeHostInputValue::F64(value)
            .into_value()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn pure_host_call_consumes_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = test_runtime_builder()
        .host_function(PlannedPureHostFunction::new(
            "demo/pure-limited",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(scalar_snapshot(1.0))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(scalar_snapshot(1.0))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call_with_limit(&mut runtime, CapabilityId(710), "demo/pure-limited", 1);

    runtime
        .call_host(HostCall::new("demo/pure-limited", Vec::new()))
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        runtime
            .call_host(HostCall::new("demo/pure-limited", Vec::new()))
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn runtime_managed_host_call_consumes_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = test_runtime_builder()
        .host_function(PlannedRuntimeManagedHostFunction::new(
            "demo/managed-limited",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(scalar_snapshot(1.0))
            },
            move |_services, _context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(scalar_snapshot(1.0))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call_with_limit(&mut runtime, CapabilityId(711), "demo/managed-limited", 1);

    runtime
        .call_host(HostCall::new("demo/managed-limited", Vec::new()))
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(
        runtime
            .call_host(HostCall::new("demo/managed-limited", Vec::new()))
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn staged_host_call_consumes_single_use_capability() {
    let calls = Arc::new(AtomicUsize::new(0));
    let deliveries = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let delivered = deliveries.clone();
    let mut runtime = test_runtime_builder()
        .host_function(PlannedStagedHostFunction::new(
            "demo/staged-limited",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(scalar_snapshot(1.0))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                let delivered = delivered.clone();
                Ok(RuntimePreparedHostCall {
                    value: scalar_snapshot(1.0),
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
        .call_host(HostCall::new("demo/staged-limited", Vec::new()))
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(deliveries.load(Ordering::SeqCst), 1);
    assert!(
        runtime
            .call_host(HostCall::new("demo/staged-limited", Vec::new()))
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn custom_capability_without_preview_contract_fails_closed() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedPureHostFunction::new(
            "demo/unsupported-preview",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(RuntimeValueSnapshot::empty())
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                callback_calls.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::empty())
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
        .call_host(HostCall::new("demo/unsupported-preview", Vec::new()))
        .unwrap_err();

    assert_eq!(error.kind_name(), "TransactionStateUnsupported");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

use std::sync::{Arc, Mutex};

use super::support::RecordingHostEffect;
use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    CapabilityId, HostCall, MechRuntime, PlannedStagedHostFunction, PreparedRuntimeEffect,
    RuntimeCallContext, RuntimePreparedHostCall, RuntimeValueSnapshot,
};

fn string_snapshot(value: &str) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::from_value(
        crate::RuntimeHostInputValue::String(value.to_owned())
            .into_value()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn staged_host_call_returns_value_before_effect_delivery() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedStagedHostFunction::new(
            "demo/staged",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(string_snapshot("provisional"))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                Ok(RuntimePreparedHostCall {
                    value: string_snapshot("provisional"),
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

    let value = runtime
        .call_host_with_context(&mut context, HostCall::new("demo/staged", Vec::new()))
        .unwrap();

    assert_eq!(value.format_canonical_inline(), "\"provisional\"");
    assert!(log.lock().unwrap().is_empty());

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(log.lock().unwrap().as_slice(), &["delivered".to_string()],);
}

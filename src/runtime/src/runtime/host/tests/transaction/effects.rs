use std::sync::{Arc, Mutex};

use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    CapabilityId, HostCall, MechRuntime, PlannedStagedHostFunction, PreparedRuntimeEffect,
    RuntimeCallContext, RuntimePreparedHostCall, RuntimeValueSnapshot,
};
use mech_core::{LegacyValue, Ref};

use super::support::RecordingHostEffect;

#[test]
fn staged_host_call_returns_value_before_effect_delivery() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let effect_log = log.clone();
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedStagedHostFunction::new(
            "demo/staged",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                RuntimeValueSnapshot::try_capture(&LegacyValue::String(Ref::new(
                    "provisional".to_string(),
                )))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                Ok(RuntimePreparedHostCall {
                    value: RuntimeValueSnapshot::try_capture(&LegacyValue::String(Ref::new(
                        "provisional".to_string(),
                    )))?,
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

    assert_eq!(
        value.to_value(),
        LegacyValue::String(Ref::new("provisional".to_string())),
    );
    assert!(log.lock().unwrap().is_empty());

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(log.lock().unwrap().as_slice(), &["delivered".to_string()],);
}

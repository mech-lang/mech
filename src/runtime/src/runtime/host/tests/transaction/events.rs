use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    CapabilityId, HostCall, MechRuntime, PlannedPureHostFunction, RuntimeConfig, RuntimeEventKind,
    RuntimeHealth, RuntimeValueSnapshot,
};
use mech_core::{Ref, Value};

fn snapshot(value: Value) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[test]
fn host_session_events_use_shared_monotonic_sequence() {
    let mut runtime = MechRuntime::builder()
        .host_function(PlannedPureHostFunction::new(
            "demo/event-sequence",
            |_context, _arguments| Ok(snapshot(Value::F64(Ref::new(1.0)))),
            |_context, _arguments| Ok(snapshot(Value::F64(Ref::new(1.0)))),
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(740), "demo/event-sequence");

    runtime
        .call_host(HostCall::new("demo/event-sequence", Vec::new()))
        .unwrap();

    let events = runtime.list_events(None).unwrap();
    assert!(events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::HostCallCompleted { ref name }
              if name == "demo/event-sequence"
        )
    }));
    for pair in events.windows(2) {
        assert!(
            pair[0].sequence < pair[1].sequence,
            "event sequences were not strictly increasing: {:?}",
            events
                .iter()
                .map(|event| (event.sequence, event.kind.name()))
                .collect::<Vec<_>>(),
        );
    }
}

#[test]
fn failed_host_audit_survives_full_context_retention() {
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(4);
    let mut runtime = MechRuntime::new(config).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    for _ in 0..4 {
        runtime
            .emit_event_to_context(&mut context, RuntimeEventKind::RuntimeTickStarted)
            .unwrap();
    }
    assert_eq!(context.events().len(), 4);

    let error = runtime
        .call_host_with_context(
            &mut context,
            HostCall::new("missing/retained-audit", Vec::new()),
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "HostFunctionNotFound");
    for events in [
        context.events().to_vec(),
        runtime.list_events(None).unwrap(),
    ] {
        let started = events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    RuntimeEventKind::HostCallStarted { ref name }
                      if name == "missing/retained-audit"
                )
            })
            .collect::<Vec<_>>();
        let failed = events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    RuntimeEventKind::HostCallFailed { ref name, .. }
                      if name == "missing/retained-audit"
                )
            })
            .collect::<Vec<_>>();
        let aborted = events
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }))
            .collect::<Vec<_>>();
        assert_eq!(started.len(), 1, "unexpected started audit: {events:?}");
        assert_eq!(failed.len(), 1, "unexpected failed audit: {events:?}");
        assert_eq!(aborted.len(), 1, "unexpected abort audit: {events:?}");
        assert!(
            started[0].sequence < failed[0].sequence && failed[0].sequence < aborted[0].sequence,
            "failed host audit did not precede abort: {:?}",
            events
                .iter()
                .map(|event| (event.sequence, event.kind.name()))
                .collect::<Vec<_>>(),
        );
        assert!(!events.iter().any(|event| {
            matches!(
                event.kind,
                RuntimeEventKind::HostCallCompleted { ref name }
                  if name == "missing/retained-audit"
            )
        }));
    }
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);
}

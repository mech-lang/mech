use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    CapabilityId, HostCall, MechRuntime, PlannedPureHostFunction, RuntimeEventKind,
    RuntimeValueSnapshot,
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
    grant_host_call(
        &mut runtime,
        CapabilityId(740),
        "demo/event-sequence",
    );

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

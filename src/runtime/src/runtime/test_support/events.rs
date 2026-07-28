use super::super::MechRuntime;
use crate::{RuntimeEvent, RuntimeEventKind};

pub(crate) fn events_since(runtime: &MechRuntime, start: usize) -> Vec<RuntimeEvent> {
    let events = runtime.list_events(None).unwrap();
    events
        .get(start..)
        .unwrap_or_else(|| {
            panic!(
                "event suffix starts at {start}, but only {} events exist",
                events.len(),
            )
        })
        .to_vec()
}

pub(crate) fn event_count(
    events: &[RuntimeEvent],
    predicate: impl Fn(&RuntimeEventKind) -> bool,
) -> usize {
    events.iter().filter(|event| predicate(&event.kind)).count()
}

pub(crate) fn event_position(
    events: &[RuntimeEvent],
    predicate: impl Fn(&RuntimeEventKind) -> bool,
) -> Option<usize> {
    events.iter().position(|event| predicate(&event.kind))
}

pub(crate) fn assert_event_before(
    events: &[RuntimeEvent],
    before: impl Fn(&RuntimeEventKind) -> bool,
    after: impl Fn(&RuntimeEventKind) -> bool,
) {
    let before = event_position(events, before).expect("expected earlier runtime event");
    let after = event_position(events, after).expect("expected later runtime event");
    assert!(
        before < after,
        "expected event at {before} before event at {after}",
    );
}

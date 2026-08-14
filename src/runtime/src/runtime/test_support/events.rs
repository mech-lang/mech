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

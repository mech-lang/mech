//! Coordinator-side phase attribution for the source-build experiment.

use serde::Serialize;
use std::{cell::RefCell, path::PathBuf, time::Instant};

#[derive(Serialize)]
struct Event {
    pass: &'static str,
    parent: Option<usize>,
    start_ns: u64,
    end_ns: u64,
}

struct Trace {
    epoch: Instant,
    events: Vec<Event>,
    active: Option<usize>,
}

thread_local! {
    static TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) };
}

pub(super) struct Session(PathBuf);

impl Session {
    pub(super) fn start() -> Option<Self> {
        let path = std::env::var_os("MECH_NATIVE_PASS_ATTRIBUTION")?;
        TRACE.with_borrow_mut(|slot| {
            assert!(slot.is_none());
            *slot = Some(Trace {
                epoch: Instant::now(),
                events: Vec::new(),
                active: None,
            });
        });
        Some(Self(path.into()))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let trace = TRACE.with_borrow_mut(|slot| slot.take()).expect("native attribution session");
        assert!(trace.active.is_none());
        serde_json::to_writer(std::fs::File::create(&self.0).unwrap(), &trace.events).unwrap();
    }
}

pub(super) struct Span(usize);

pub(super) fn span(pass: &'static str) -> Option<Span> {
    TRACE.with_borrow_mut(|slot| {
        let trace = slot.as_mut()?;
        let entry = trace.events.len();
        trace.events.push(Event {
            pass,
            parent: trace.active.replace(entry),
            start_ns: trace.epoch.elapsed().as_nanos() as u64,
            end_ns: 0,
        });
        Some(Span(entry))
    })
}

impl Drop for Span {
    fn drop(&mut self) {
        TRACE.with_borrow_mut(|slot| {
            let trace = slot.as_mut().expect("native attribution session");
            assert_eq!(trace.active, Some(self.0));
            let event = &mut trace.events[self.0];
            event.end_ns = trace.epoch.elapsed().as_nanos() as u64;
            trace.active = event.parent;
        });
    }
}

macro_rules! measure {
    ($pass:literal, $expr:expr) => {{
        let _span = crate::native_artifact::program::attribution::span($pass);
        $expr
    }};
}

pub(super) use measure;

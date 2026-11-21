//! Diagnostic-only records. No shipping runtime imports this module.
use std::time::Instant;

pub type Stamp = Option<Instant>;

#[inline]
pub fn stamp() -> Stamp {
    if cfg!(setup_probe_timing) { Some(Instant::now()) } else { None }
}

#[inline]
pub fn elapsed(start: Stamp) -> u64 {
    start.map_or(0, |s| s.elapsed().as_nanos().try_into().unwrap())
}

#[derive(Clone, Debug)]
pub struct Event {
    pub op: &'static str,
    pub pointer: usize,
    pub bytes: usize,
    pub stride: usize,
    pub zeroed: bool,
    pub mapped: bool,
    pub total_ns: u64,
    pub payload_ns: u64,
}

#[derive(Default)]
pub struct Probe {
    pub events: Vec<Event>,
}

impl Probe {
    pub fn reserve(&mut self) {
        self.events.reserve_exact(2048);
    }

    #[inline]
    pub fn record(&mut self, event: Event) {
        assert!(self.events.len() < self.events.capacity(), "probe must not allocate in the call");
        self.events.push(event);
    }
}

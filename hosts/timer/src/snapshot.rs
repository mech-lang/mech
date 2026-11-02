use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{
    RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue,
};

pub fn timer_input_base_uri(instance: &str) -> String {
    format!("timer://{instance}/tick")
}

pub fn timer_source_matches(instance: &str, source: &RuntimeHostInputSource) -> bool {
    source.base_uri() == timer_input_base_uri(instance) && TIMER_PATHS.contains(&source.path())
}

pub const TIMER_PATHS: [&str; 6] = [
    "tick",
    "elapsed-ms",
    "delta-ms",
    "elapsed-seconds",
    "delta-seconds",
    "skipped-steps",
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimerSnapshot {
    pub tick: u64,
    pub elapsed_ms: f64,
    pub delta_ms: f64,
    pub elapsed_seconds: f64,
    pub delta_seconds: f64,
    pub skipped_steps: u64,
}

impl TimerSnapshot {
    pub fn new(tick: u64, frequency_hz: u64, skipped_steps: u64) -> Self {
        let delta_seconds = 1.0 / frequency_hz as f64;
        let delta_ms = 1000.0 / frequency_hz as f64;
        Self {
            tick,
            elapsed_ms: tick as f64 * delta_ms,
            delta_ms,
            elapsed_seconds: tick as f64 * delta_seconds,
            delta_seconds,
            skipped_steps,
        }
    }

    pub fn into_host_input(self, instance: &str) -> MResult<RuntimeHostInput> {
        let base_uri = timer_input_base_uri(instance);
        let values = [
            self.tick as f64,
            self.elapsed_ms,
            self.delta_ms,
            self.elapsed_seconds,
            self.delta_seconds,
            self.skipped_steps as f64,
        ];
        let mut updates = Vec::with_capacity(TIMER_PATHS.len());
        for (path, value) in TIMER_PATHS.iter().zip(values) {
            updates.push(RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(base_uri.clone(), *path)?,
                value: RuntimeHostInputValue::F64(value),
            });
        }
        RuntimeHostInput::new(updates)
    }
}

pub trait MonotonicTimerBackend: Clone + std::fmt::Debug + 'static {
    fn now_ms(&self) -> MResult<f64>;
}

pub type SharedTimerSnapshot = Arc<Mutex<TimerSnapshot>>;

pub fn new_shared_snapshot(snapshot: TimerSnapshot) -> SharedTimerSnapshot {
    Arc::new(Mutex::new(snapshot))
}

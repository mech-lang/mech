use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue};

pub const CLOCK_PATHS: [&str; 5] = ["unix-ms", "hour", "minute", "second", "millisecond"];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TimeSnapshot {
  pub unix_ms: f64,
  pub hour: f64,
  pub minute: f64,
  pub second: f64,
  pub millisecond: f64,
}

impl TimeSnapshot {
  pub fn into_host_input(self, instance: &str) -> MResult<RuntimeHostInput> {
    let base_uri = format!("time://{instance}/clock");
    let values = [self.unix_ms, self.hour, self.minute, self.second, self.millisecond];
    let mut updates = Vec::with_capacity(CLOCK_PATHS.len());
    for (path, value) in CLOCK_PATHS.iter().zip(values) {
      updates.push(RuntimeHostInputUpdate {
        source: RuntimeHostInputSource::new(base_uri.clone(), *path)?,
        value: RuntimeHostInputValue::F64(value),
      });
    }
    RuntimeHostInput::new(updates)
  }
}

pub trait TimeBackend: Clone + std::fmt::Debug + 'static {
  fn snapshot(&self) -> MResult<TimeSnapshot>;
}

pub type SharedTimeSnapshot = Arc<Mutex<TimeSnapshot>>;

pub fn new_shared_snapshot(snapshot: TimeSnapshot) -> SharedTimeSnapshot {
  Arc::new(Mutex::new(snapshot))
}

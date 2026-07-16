use mech_core::{MResult, Ref, Value};
use mech_runtime::{RuntimeResourceProvider, RuntimeResourceReadRequest};

use crate::{SharedTimerSnapshot, TimerSnapshot, timer_error};

#[derive(Debug)]
pub struct TimerResourceProvider {
    instance: String,
    snapshot: SharedTimerSnapshot,
}

impl TimerResourceProvider {
    pub fn new(instance: impl Into<String>, snapshot: SharedTimerSnapshot) -> Self {
        Self {
            instance: instance.into(),
            snapshot,
        }
    }

    pub fn base_uri(&self) -> String {
        format!("timer://{}/tick", self.instance)
    }

    fn value_for(snapshot: TimerSnapshot, path: &str) -> MResult<Value> {
        let value = match path {
            "tick" => snapshot.tick as f64,
            "elapsed-ms" => snapshot.elapsed_ms,
            "delta-ms" => snapshot.delta_ms,
            "elapsed-seconds" => snapshot.elapsed_seconds,
            "delta-seconds" => snapshot.delta_seconds,
            "skipped-steps" => snapshot.skipped_steps as f64,
            other => {
                return Err(timer_error(
                    "TimerResourceProvider",
                    format!("unknown timer tick path `{other}`"),
                ));
            }
        };
        Ok(Value::F64(Ref::new(value)))
    }
}

impl RuntimeResourceProvider for TimerResourceProvider {
    fn scheme(&self) -> &str {
        "timer"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base_uri()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri != self.base_uri() {
            return Err(timer_error(
                "TimerResourceProvider",
                format!("unknown timer resource `{}`", request.base_uri),
            ));
        }
        let snapshot = *self
            .snapshot
            .lock()
            .map_err(|_| timer_error("TimerResourceProvider", "timer snapshot lock is poisoned"))?;
        Self::value_for(snapshot, &request.path)
    }
}

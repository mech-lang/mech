use mech_core::{MResult, Ref, Value};
use mech_runtime::{RuntimeResourceProvider, RuntimeResourceReadRequest};

use crate::{time_error, SharedTimeSnapshot, TimeSnapshot};

#[derive(Debug)]
pub struct TimeResourceProvider {
  instance: String,
  snapshot: SharedTimeSnapshot,
}

impl TimeResourceProvider {
  pub fn new(instance: impl Into<String>, snapshot: SharedTimeSnapshot) -> Self {
    Self { instance: instance.into(), snapshot }
  }

  pub fn base_uri(&self) -> String {
    format!("time://{}/clock", self.instance)
  }

  fn value_for(snapshot: TimeSnapshot, path: &str) -> MResult<Value> {
    let value = match path {
      "unix-ms" => snapshot.unix_ms,
      "hour" => snapshot.hour,
      "minute" => snapshot.minute,
      "second" => snapshot.second,
      "millisecond" => snapshot.millisecond,
      other => return Err(time_error("TimeResourceProvider", format!("unknown time clock path `{other}`"))),
    };
    Ok(Value::F64(Ref::new(value)))
  }
}

impl RuntimeResourceProvider for TimeResourceProvider {
  fn scheme(&self) -> &str { "time" }

  fn base_uris(&self) -> Vec<String> { vec![self.base_uri()] }

  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
    if request.base_uri != self.base_uri() {
      return Err(time_error("TimeResourceProvider", format!("unknown time resource `{}`", request.base_uri)));
    }
    let snapshot = *self.snapshot.lock().map_err(|_| time_error("TimeResourceProvider", "time snapshot lock is poisoned"))?;
    Self::value_for(snapshot, &request.path)
  }
}

use mech_core::MResult;
use mech_runtime::ConfigValue;

use crate::time_error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeHostSettings {
  pub interval_ms: u64,
}

impl Default for TimeHostSettings {
  fn default() -> Self { Self { interval_ms: 100 } }
}

pub fn time_settings_from_config(settings: &ConfigValue) -> MResult<TimeHostSettings> {
  let ConfigValue::Map(map) = settings else {
    return Err(time_error("TimeHostConfig", "time host settings must be a map"));
  };
  let mut parsed = TimeHostSettings::default();
  for (key, value) in map {
    match key.as_str() {
      "interval-ms" => {
        let ConfigValue::Integer(raw) = value else {
          return Err(time_error("TimeHostConfig", "time host interval-ms must be an integer"));
        };
        if *raw <= 0 {
          return Err(time_error("TimeHostConfig", "time host interval-ms must be positive"));
        }
        if *raw > 60_000 {
          return Err(time_error("TimeHostConfig", "time host interval-ms must be at most 60000"));
        }
        parsed.interval_ms = *raw as u64;
      }
      other => return Err(time_error("TimeHostConfig", format!("unknown time host setting `{other}`"))),
    }
  }
  Ok(parsed)
}

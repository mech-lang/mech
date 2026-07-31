use mech_core::MResult;
use mech_runtime::ConfigValue;

use crate::timer_error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimerHostSettings {
    pub frequency_hz: u64,
    pub max_catch_up_steps: u64,
}

impl Default for TimerHostSettings {
    fn default() -> Self {
        Self {
            frequency_hz: 60,
            max_catch_up_steps: 8,
        }
    }
}

pub fn timer_settings_from_config(settings: &ConfigValue) -> MResult<TimerHostSettings> {
    let ConfigValue::Map(map) = settings else {
        return Err(timer_error(
            "TimerHostConfig",
            "timer host settings must be a map",
        ));
    };
    let mut parsed = TimerHostSettings::default();
    for (key, value) in map {
        match key.as_str() {
            "frequency-hz" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(timer_error(
                        "TimerHostConfig",
                        "timer host frequency-hz must be an integer",
                    ));
                };
                if !(1..=1000).contains(raw) {
                    return Err(timer_error(
                        "TimerHostConfig",
                        "timer host frequency-hz must be between 1 and 1000",
                    ));
                }
                parsed.frequency_hz = *raw as u64;
            }
            "max-catch-up-steps" => {
                let ConfigValue::Integer(raw) = value else {
                    return Err(timer_error(
                        "TimerHostConfig",
                        "timer host max-catch-up-steps must be an integer",
                    ));
                };
                if !(1..=64).contains(raw) {
                    return Err(timer_error(
                        "TimerHostConfig",
                        "timer host max-catch-up-steps must be between 1 and 64",
                    ));
                }
                parsed.max_catch_up_steps = *raw as u64;
            }
            other => {
                return Err(timer_error(
                    "TimerHostConfig",
                    format!("unknown timer host setting `{other}`"),
                ));
            }
        }
    }
    Ok(parsed)
}

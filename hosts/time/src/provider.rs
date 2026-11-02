use mech_core::{MResult, OperationContractDeclaration, Value};
use mech_runtime::{RuntimeHostInputValue, RuntimeResourceProvider, RuntimeResourceReadRequest};

use crate::{SharedTimeSnapshot, TimeSnapshot, time_error, time_input_base_uri};

#[derive(Debug)]
pub struct TimeResourceProvider {
    instance: String,
    snapshot: SharedTimeSnapshot,
}

impl TimeResourceProvider {
    pub fn new(instance: impl Into<String>, snapshot: SharedTimeSnapshot) -> Self {
        Self {
            instance: instance.into(),
            snapshot,
        }
    }

    pub fn base_uri(&self) -> String {
        time_input_base_uri(&self.instance)
    }

    fn value_for(snapshot: TimeSnapshot, path: &str) -> MResult<Value> {
        let value = match path {
            "unix-ms" => snapshot.unix_ms,
            "hour" => snapshot.hour,
            "minute" => snapshot.minute,
            "second" => snapshot.second,
            "millisecond" => snapshot.millisecond,
            other => {
                return Err(time_error(
                    "TimeResourceProvider",
                    format!("unknown time clock path `{other}`"),
                ));
            }
        };
        RuntimeHostInputValue::F64(value).into_value()
    }
}

impl RuntimeResourceProvider for TimeResourceProvider {
    fn scheme(&self) -> &str {
        "time"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![self.base_uri()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri != self.base_uri() {
            return Err(time_error(
                "TimeResourceProvider",
                format!("unknown time resource `{}`", request.base_uri),
            ));
        }
        Self::value_for(TimeSnapshot::default(), &request.path)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri != self.base_uri() {
            return Err(time_error(
                "TimeResourceProvider",
                format!("unknown time resource `{}`", request.base_uri),
            ));
        }
        let snapshot = *self
            .snapshot
            .lock()
            .map_err(|_| time_error("TimeResourceProvider", "time snapshot lock is poisoned"))?;
        Self::value_for(snapshot, &request.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CLOCK_PATHS, new_shared_snapshot};

    fn request(base_uri: &str, path: &str) -> RuntimeResourceReadRequest {
        RuntimeResourceReadRequest {
            base_uri: base_uri.to_owned(),
            path: path.to_owned(),
            context_name: "clock".to_owned(),
        }
    }

    fn f64_value(value: Value) -> f64 {
        match value.data() {
            mech_core::ValueData::F64(value) => value.to_f64(),
            value => panic!("expected F64, got {value:?}"),
        }
    }

    #[test]
    fn planning_returns_zero_for_every_clock_path_without_reading_snapshot() {
        let snapshot = new_shared_snapshot(TimeSnapshot {
            unix_ms: 1.0,
            hour: 2.0,
            minute: 3.0,
            second: 4.0,
            millisecond: 5.0,
        });
        let poison = snapshot.clone();
        assert!(
            std::thread::spawn(move || {
                let _guard = poison.lock().unwrap();
                panic!("poison planning snapshot");
            })
            .join()
            .is_err()
        );
        let provider = TimeResourceProvider::new("clock", snapshot);

        for path in CLOCK_PATHS {
            assert_eq!(
                f64_value(
                    provider
                        .plan_read(request("time://clock/clock", path))
                        .unwrap(),
                ),
                0.0,
            );
        }
    }

    #[test]
    fn planning_validates_exact_clock_base_and_path() {
        let provider = TimeResourceProvider::new("clock", new_shared_snapshot(Default::default()));
        assert!(
            provider
                .plan_read(request("time://other/clock", "second"))
                .is_err()
        );
        assert!(
            provider
                .plan_read(request("time://clock/clock", "seconds"))
                .is_err()
        );
    }

    #[test]
    fn execute_reads_the_real_snapshot() {
        let provider = TimeResourceProvider::new(
            "clock",
            new_shared_snapshot(TimeSnapshot {
                second: 37.0,
                ..Default::default()
            }),
        );
        assert_eq!(
            f64_value(
                provider
                    .read(request("time://clock/clock", "second"))
                    .unwrap(),
            ),
            37.0,
        );
    }
}

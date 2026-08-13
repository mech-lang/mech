use mech_core::{LegacyValue, MResult, OperationContractDeclaration, Ref};
use mech_runtime::{RuntimeResourceProvider, RuntimeResourceReadRequest};

use crate::{SharedTimerSnapshot, TimerSnapshot, timer_error, timer_input_base_uri};

#[derive(Debug)]
pub struct TimerResourceProvider {
    instance: String,
    snapshot: SharedTimerSnapshot,
    planning_snapshot: TimerSnapshot,
}

impl TimerResourceProvider {
    pub fn new(instance: impl Into<String>, snapshot: SharedTimerSnapshot) -> Self {
        Self::new_with_planning_snapshot(instance, snapshot, TimerSnapshot::default())
    }

    pub fn new_with_planning_snapshot(
        instance: impl Into<String>,
        snapshot: SharedTimerSnapshot,
        planning_snapshot: TimerSnapshot,
    ) -> Self {
        Self {
            instance: instance.into(),
            snapshot,
            planning_snapshot,
        }
    }

    pub fn base_uri(&self) -> String {
        timer_input_base_uri(&self.instance)
    }

    fn value_for(snapshot: TimerSnapshot, path: &str) -> MResult<LegacyValue> {
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
        Ok(LegacyValue::F64(Ref::new(value)))
    }
}

impl RuntimeResourceProvider for TimerResourceProvider {
    fn scheme(&self) -> &str {
        "timer"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base_uri()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if request.base_uri != self.base_uri() {
            return Err(timer_error(
                "TimerResourceProvider",
                format!("unknown timer resource `{}`", request.base_uri),
            ));
        }
        Self::value_for(self.planning_snapshot, &request.path)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TIMER_PATHS, new_shared_snapshot};
    use mech_core::{ExternalInteraction, ObservationContract, ObservationReplayPolicy};
    use mech_runtime::RuntimeHostInputValue;

    fn request(base_uri: &str, path: &str) -> RuntimeResourceReadRequest {
        RuntimeResourceReadRequest {
            base_uri: base_uri.to_owned(),
            path: path.to_owned(),
            context_name: "tick".to_owned(),
        }
    }

    fn f64_value(value: LegacyValue) -> f64 {
        match value {
            LegacyValue::F64(value) => *value.borrow(),
            value => panic!("expected F64, got {value:?}"),
        }
    }

    #[test]
    fn planning_returns_zero_for_every_timer_path_without_reading_snapshot() {
        let snapshot = new_shared_snapshot(TimerSnapshot::new(9, 20, 3));
        let poison = snapshot.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poison.lock().unwrap();
            panic!("poison planning snapshot");
        })
        .join();
        let provider = TimerResourceProvider::new("timer", snapshot);

        for path in TIMER_PATHS {
            assert_eq!(
                f64_value(
                    provider
                        .plan_read(request("timer://timer/tick", path))
                        .unwrap(),
                ),
                0.0,
            );
        }
    }

    #[test]
    fn planning_validates_exact_timer_base_and_path() {
        let provider = TimerResourceProvider::new("timer", new_shared_snapshot(Default::default()));
        assert!(
            provider
                .plan_read(request("timer://other/tick", "tick"))
                .is_err()
        );
        assert!(
            provider
                .plan_read(request("timer://timer/tick", "ticks"))
                .is_err()
        );
    }

    #[test]
    fn execute_reads_the_real_snapshot() {
        let provider =
            TimerResourceProvider::new("timer", new_shared_snapshot(TimerSnapshot::new(9, 20, 3)));
        assert_eq!(
            f64_value(
                provider
                    .read(request("timer://timer/tick", "tick"))
                    .unwrap(),
            ),
            9.0,
        );
    }

    #[test]
    fn resident_contract_captures_timer_reads_as_input_facts() {
        let provider =
            TimerResourceProvider::new("clock", new_shared_snapshot(TimerSnapshot::default()));
        assert!(matches!(
            &provider.semantic_read_contract().unwrap().interaction,
            ExternalInteraction::Observation(ObservationContract {
                replay: ObservationReplayPolicy::CaptureAsInputFact,
            })
        ));
    }

    #[test]
    fn sixty_hertz_planning_delta_is_one_sixtieth() {
        let snapshot = new_shared_snapshot(TimerSnapshot::default());
        let provider = TimerResourceProvider::new_with_planning_snapshot(
            "clock",
            snapshot,
            TimerSnapshot::new(0, 60, 0),
        );
        assert_eq!(
            f64_value(
                provider
                    .plan_read(request("timer://clock/tick", "delta-seconds"))
                    .unwrap(),
            ),
            1.0 / 60.0,
        );
    }

    #[test]
    fn live_read_matches_the_snapshot_carried_by_the_trigger_packet() {
        let emitted = TimerSnapshot::new(1, 60, 0);
        let snapshot = new_shared_snapshot(emitted);
        let provider = TimerResourceProvider::new("clock", snapshot);
        let packet = emitted.into_host_input("clock").unwrap();
        let update = packet
            .updates
            .iter()
            .find(|update| update.source.path() == "delta-seconds")
            .unwrap();
        let RuntimeHostInputValue::F64(trigger_value) = &update.value else {
            panic!("timer delta trigger must be f64")
        };
        assert_eq!(
            f64_value(
                provider
                    .read(request("timer://clock/tick", "delta-seconds"))
                    .unwrap(),
            ),
            *trigger_value,
        );
    }
}

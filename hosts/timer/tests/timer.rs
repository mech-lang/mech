use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use mech_core::Value;
use mech_host_timer::*;
use mech_runtime::{
    ConfigValue, RuntimeHostFactory, RuntimeHostInputDriver, RuntimeResourceProvider,
    RuntimeResourceReadRequest,
};

fn settings(freq: i64, catch: i64) -> ConfigValue {
    let mut map = BTreeMap::new();
    map.insert("frequency-hz".to_string(), ConfigValue::Integer(freq));
    map.insert(
        "max-catch-up-steps".to_string(),
        ConfigValue::Integer(catch),
    );
    ConfigValue::Map(map)
}

#[test]
fn timer_config_rejects_interval_ms() {
    let mut map = BTreeMap::new();
    map.insert("interval-ms".to_string(), ConfigValue::Integer(16));
    assert!(timer_settings_from_config(&ConfigValue::Map(map)).is_err());
}

#[test]
fn timer_config_accepts_frequency_and_catchup() {
    let parsed = timer_settings_from_config(&settings(120, 8)).unwrap();
    assert_eq!(parsed.frequency_hz, 120);
    assert_eq!(parsed.max_catch_up_steps, 8);
}

#[test]
fn timer_packet_contains_all_fields_atomically() {
    let packet = TimerSnapshot::new(1, 120, 0)
        .into_host_input("physics")
        .unwrap();
    let paths: Vec<_> = packet.updates.iter().map(|u| u.source.path()).collect();
    assert_eq!(paths, TIMER_PATHS);
    assert_eq!(packet.updates.len(), 6);
}

#[test]
fn provider_reads_all_timer_fields() {
    let provider = TimerResourceProvider::new(
        "physics",
        new_shared_snapshot(TimerSnapshot::new(2, 120, 1)),
    );
    for path in TIMER_PATHS {
        let value = provider
            .read(RuntimeResourceReadRequest {
                base_uri: "timer://physics/tick".to_string(),
                path: path.to_string(),
                context_name: "tick".to_string(),
            })
            .unwrap();
        assert!(matches!(value, Value::F64(_)));
    }
}

#[test]
fn manual_timer_advance_publishes_due_steps_without_sleeping() {
    let mut driver = ManualTimerInputDriver::new("physics", 100, 8);
    driver.start().unwrap();
    driver.advance_ms(0.0).unwrap();
    assert_eq!(driver.publish_due_steps().unwrap(), 0);
    driver.advance_ms(25.0).unwrap();
    assert_eq!(driver.publish_due_steps().unwrap(), 2);
}

#[test]
fn start_and_stop_are_idempotent_and_restartable() {
    let mut driver = ManualTimerInputDriver::new("physics", 60, 8);
    driver.start().unwrap();
    driver.start().unwrap();
    assert!(driver.is_live());
    driver.stop().unwrap();
    driver.stop().unwrap();
    assert!(!driver.is_live());
    driver.start().unwrap();
    assert!(driver.is_live());
}

#[cfg(feature = "native")]
#[test]
fn native_stop_wakes_promptly() {
    let factory = NativeTimerHostFactory::new().unwrap();
    let mut installation = factory.instantiate("physics", &settings(60, 8)).unwrap();
    let mut driver = installation.input_drivers.remove(0);
    let runtime = mech_runtime::RuntimeBuilder::new().build().unwrap();
    driver.attach(runtime.ingress()).unwrap();
    driver.start().unwrap();
    let start = Instant::now();
    driver.stop().unwrap();
    assert!(start.elapsed() < Duration::from_millis(100));
}

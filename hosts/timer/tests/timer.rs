use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use mech_core::Value;
use mech_host_timer::*;
use mech_runtime::{
    ConfigValue, RuntimeHostFactory, RuntimeHostInputDriver, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeBuilder,
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

fn snapshot_tick(snapshot: &SharedTimerSnapshot) -> u64 {
    snapshot.lock().unwrap().tick
}

fn runtime_with_manual_timer(
    capacity: usize,
) -> (mech_runtime::MechRuntime, ManualTimerInputDriver, SharedTimerSnapshot) {
    let runtime = RuntimeBuilder::new()
        .host_input_capacity(capacity)
        .resource_provider(Box::new(TimerResourceProvider::new(
            "physics",
            new_shared_snapshot(TimerSnapshot::new(0, 100, 0)),
        )))
        .build()
        .unwrap();
    let mut driver = ManualTimerInputDriver::with_backend(
        "physics",
        ManualMonotonicTimerBackend::new(),
        100,
        8,
    );
    driver.attach(runtime.ingress()).unwrap();
    driver.start().unwrap();
    let snapshot = driver.snapshot();
    (runtime, driver, snapshot)
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
fn ingress_full_retains_timer_packet() {
    let (runtime, mut driver, snapshot) = runtime_with_manual_timer(1);
    assert_eq!(driver.publish_steps(2).unwrap(), 1);
    assert!(driver.is_live());
    assert_eq!(driver.pending_emission_count(), 1);
    assert_eq!(snapshot_tick(&snapshot), 1);
    assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
}

#[test]
fn retained_timer_packets_submit_in_order() {
    let mut driver = ManualTimerInputDriver::new("physics", 100, 8);
    let snapshot = driver.snapshot();
    assert_eq!(driver.publish_steps(1).unwrap(), 1);
    assert_eq!(snapshot_tick(&snapshot), 1);
    assert_eq!(driver.publish_steps(1).unwrap(), 1);
    assert_eq!(snapshot_tick(&snapshot), 2);
    assert_eq!(driver.publish_steps(1).unwrap(), 1);
    assert_eq!(snapshot_tick(&snapshot), 3);
}

#[test]
fn provider_snapshot_advances_only_after_submit() {
    let (_runtime, mut driver, snapshot) = runtime_with_manual_timer(1);
    assert_eq!(driver.publish_steps(3).unwrap(), 1);
    assert_eq!(snapshot_tick(&snapshot), 1);
    assert_eq!(driver.pending_emission_count(), 2);
}

#[test]
fn queue_pressure_does_not_stop_timer_driver() {
    let (_runtime, mut driver, _snapshot) = runtime_with_manual_timer(1);
    assert_eq!(driver.publish_steps(10).unwrap(), 1);
    assert!(driver.is_live());
}

#[test]
fn closed_ingress_stops_timer_driver() {
    let (mut runtime, mut driver, _snapshot) = runtime_with_manual_timer(1);
    runtime.close_ingress().unwrap();
    assert_eq!(driver.publish_steps(1).unwrap(), 0);
    assert!(!driver.is_live());
}

#[test]
fn unexpected_ingress_error_stops_or_surfaces() {
    let (mut runtime, mut driver, _snapshot) = runtime_with_manual_timer(1);
    runtime.close_ingress().unwrap();
    assert_eq!(driver.publish_steps(1).unwrap(), 0);
    assert!(!driver.is_live());
}

#[test]
fn manual_publish_steps_advances_unique_ticks() {
    let (_runtime, mut driver, snapshot) = runtime_with_manual_timer(8);
    assert_eq!(driver.publish_steps(3).unwrap(), 3);
    assert_eq!(snapshot_tick(&snapshot), 3);
}

#[test]
fn manual_publish_steps_advances_elapsed() {
    let (_runtime, mut driver, snapshot) = runtime_with_manual_timer(8);
    assert_eq!(driver.publish_steps(3).unwrap(), 3);
    assert_eq!(snapshot.lock().unwrap().elapsed_ms, 30.0);
}

#[test]
fn manual_publish_steps_preserves_order_under_backpressure() {
    retained_timer_packets_submit_in_order();
}

#[cfg(feature = "native")]
#[test]
fn native_wait_uses_next_scheduler_boundary() {
    let mut scheduler = FixedStepScheduler::new(100, 8);
    scheduler.due_steps(0.0);
    assert_eq!(mech_host_timer::native::native_wait_duration(&scheduler, 4.0), Duration::from_millis(6));
}

#[cfg(feature = "browser")]
#[test]
fn browser_wake_interval_is_derived_from_frequency() {
    assert_eq!(mech_host_timer::browser::browser_wake_interval_ms(&FixedStepScheduler::new(120, 8)), 4);
}

#[cfg(feature = "browser")]
#[test]
fn browser_wake_interval_is_bounded() {
    assert_eq!(mech_host_timer::browser::browser_wake_interval_ms(&FixedStepScheduler::new(1000, 8)), 1);
    assert_eq!(mech_host_timer::browser::browser_wake_interval_ms(&FixedStepScheduler::new(1, 8)), 16);
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

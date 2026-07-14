pub mod config;
pub mod manual;
pub mod module;
pub mod provider;
pub mod snapshot;

#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "native")]
pub mod native;

pub use config::{time_settings_from_config, TimeHostSettings};
pub use manual::ManualTimeInputDriver;
pub use module::{time_host_manifest, TIME_HOST_MCFG};
pub use provider::TimeResourceProvider;
pub use snapshot::{new_shared_snapshot, SharedTimeSnapshot, TimeBackend, TimeSnapshot, CLOCK_PATHS};

#[cfg(feature = "browser")]
pub use browser::{BrowserTimeBackend, BrowserTimeHostFactory, BrowserTimeInputDriver};
#[cfg(feature = "native")]
pub use native::{NativeTimeBackend, NativeTimeHostFactory, NativeTimeInputDriver};

use mech_core::{MechError, MechErrorKind};

#[derive(Debug, Clone)]
pub struct TimeHostError { pub name: &'static str, pub message: String }
impl MechErrorKind for TimeHostError {
  fn name(&self) -> &str { self.name }
  fn message(&self) -> String { self.message.clone() }
}

pub(crate) fn time_error(name: &'static str, message: impl Into<String>) -> MechError {
  MechError::new(TimeHostError { name, message: message.into() }, None)
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use mech_core::Value;
  use mech_runtime::{ConfigValue, RuntimeHostFactory, RuntimeHostInputDriver, RuntimeResourceProvider, RuntimeResourceReadRequest};

  use super::*;

  fn snapshot() -> TimeSnapshot {
    TimeSnapshot { unix_ms: 1.0, hour: 3.0, minute: 4.0, second: 5.0, millisecond: 6.0 }
  }

  fn empty_settings() -> ConfigValue { ConfigValue::Map(BTreeMap::new()) }

  #[test]
  fn snapshot_packet_contains_all_five_paths() {
    let packet = snapshot().into_host_input("clock").unwrap();
    let paths: Vec<_> = packet.updates.iter().map(|u| u.source.path()).collect();
    assert_eq!(paths, CLOCK_PATHS);
    assert_eq!(packet.updates.len(), 5);
  }

  #[test]
  fn snapshot_packet_uses_one_base_uri() {
    let packet = snapshot().into_host_input("clock").unwrap();
    assert!(packet.updates.iter().all(|u| u.source.base_uri() == "time://clock/clock"));
  }

  #[test]
  fn snapshot_packet_has_no_duplicate_sources() {
    let packet = snapshot().into_host_input("clock").unwrap();
    packet.validate().unwrap();
  }

  #[test]
  fn provider_reads_all_supported_paths() {
    let provider = TimeResourceProvider::new("clock", new_shared_snapshot(snapshot()));
    for path in CLOCK_PATHS {
      let value = provider.read(RuntimeResourceReadRequest {
        base_uri: "time://clock/clock".to_string(),
        path: path.to_string(),
        context_name: "clock".to_string(),
      }).unwrap();
      assert!(matches!(value, Value::F64(_)));
    }
  }

  #[test]
  fn provider_rejects_unknown_path() {
    let provider = TimeResourceProvider::new("clock", new_shared_snapshot(snapshot()));
    let err = provider.read(RuntimeResourceReadRequest {
      base_uri: "time://clock/clock".to_string(),
      path: "unknown".to_string(),
      context_name: "clock".to_string(),
    }).unwrap_err();
    assert!(format!("{:?}", err).contains("TimeResourceProvider"));
  }

  #[test]
  fn manual_driver_requires_attachment() {
    let mut driver = ManualTimeInputDriver::new("clock", new_shared_snapshot(snapshot()));
    assert!(driver.start().is_err());
  }

  #[test]
  fn manual_driver_requires_start() {
    let runtime = mech_runtime::MechRuntime::builder().build().unwrap();
    let mut driver = ManualTimeInputDriver::new("clock", new_shared_snapshot(snapshot()));
    driver.attach(runtime.ingress()).unwrap();
    assert!(driver.publish(snapshot()).is_err());
  }

  #[test]
  fn manual_driver_publishes_one_packet() {
    let runtime = mech_runtime::MechRuntime::builder().build().unwrap();
    let mut driver = ManualTimeInputDriver::new("clock", new_shared_snapshot(snapshot()));
    driver.attach(runtime.ingress()).unwrap();
    driver.start().unwrap();
    driver.publish(snapshot()).unwrap();
    assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
  }

  #[test]
  fn manual_driver_updates_shared_snapshot() {
    let runtime = mech_runtime::MechRuntime::builder().build().unwrap();
    let shared = new_shared_snapshot(TimeSnapshot::default());
    let mut driver = ManualTimeInputDriver::new("clock", shared.clone());
    driver.attach(runtime.ingress()).unwrap();
    driver.start().unwrap();
    driver.publish(snapshot()).unwrap();
    assert_eq!(*shared.lock().unwrap(), snapshot());
  }

  #[test]
  fn manual_driver_start_and_stop_are_idempotent() {
    let runtime = mech_runtime::MechRuntime::builder().build().unwrap();
    let mut driver = ManualTimeInputDriver::new("clock", new_shared_snapshot(snapshot()));
    driver.attach(runtime.ingress()).unwrap();
    driver.start().unwrap();
    driver.start().unwrap();
    assert!(driver.is_live());
    driver.stop().unwrap();
    driver.stop().unwrap();
    assert!(!driver.is_live());
  }

  #[test]
  fn settings_accept_default() {
    assert_eq!(time_settings_from_config(&empty_settings()).unwrap(), TimeHostSettings { interval_ms: 100 });
  }

  #[test]
  fn settings_accept_positive_interval() {
    let mut map = BTreeMap::new();
    map.insert("interval-ms".to_string(), ConfigValue::Integer(250));
    assert_eq!(time_settings_from_config(&ConfigValue::Map(map)).unwrap(), TimeHostSettings { interval_ms: 250 });
  }

  #[test]
  fn settings_reject_zero() {
    let mut map = BTreeMap::new();
    map.insert("interval-ms".to_string(), ConfigValue::Integer(0));
    assert!(time_settings_from_config(&ConfigValue::Map(map)).is_err());
  }

  #[test]
  fn settings_reject_unknown_keys() {
    let mut map = BTreeMap::new();
    map.insert("frequency-hz".to_string(), ConfigValue::Integer(10));
    assert!(time_settings_from_config(&ConfigValue::Map(map)).is_err());
  }

  #[cfg(feature = "native")]
  #[derive(Clone, Debug)]
  struct FixedBackend;

  #[cfg(feature = "native")]
  impl TimeBackend for FixedBackend {
    fn snapshot(&self) -> mech_core::MResult<TimeSnapshot> { Ok(snapshot()) }
  }

  #[cfg(feature = "native")]
  #[test]
  fn native_factory_installs_one_provider_and_one_driver() {
    let factory = NativeTimeHostFactory::with_backend(FixedBackend).unwrap();
    let installation = factory.instantiate("clock", &empty_settings()).unwrap();
    assert_eq!(installation.resource_providers.len(), 1);
    assert_eq!(installation.input_drivers.len(), 1);
    assert_eq!(installation.interface.instance, "clock");
    assert_eq!(installation.interface.provider, "time");
  }
}

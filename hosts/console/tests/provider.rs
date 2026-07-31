use mech_core::Value;
use mech_host_console::{ConsoleHostFactory, ConsoleResourceProvider, RecordingConsoleBackend};
use mech_runtime::{RuntimeHostFactory, RuntimeResourceProvider, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest, RuntimeCapabilityOperation};

#[test]
fn provider_writes_line_to_backend() {
  let backend = RecordingConsoleBackend::new();
  let observed = backend.clone();
  let mut provider = ConsoleResourceProvider::new("console", backend);
  provider.write(RuntimeResourceWriteRequest {
    base_uri: "console://console/output".to_string(),
    path: "line".to_string(),
    context_name: "out".to_string(),
    operation: RuntimeCapabilityOperation::Write,
    value: Value::from("hello".to_string()),
    intent: RuntimeResourceWriteIntent::Send,
  }).unwrap();
  assert_eq!(observed.lines(), vec!["hello".to_string()]);
}

#[test]
fn provider_rejects_unknown_path() {
  let backend = RecordingConsoleBackend::new();
  let mut provider = ConsoleResourceProvider::new("console", backend);
  let err = provider.write(RuntimeResourceWriteRequest {
    base_uri: "console://console/output".to_string(),
    path: "text".to_string(),
    context_name: "out".to_string(),
    operation: RuntimeCapabilityOperation::Write,
    value: Value::from("hello".to_string()),
    intent: RuntimeResourceWriteIntent::Send,
  }).unwrap_err();
  assert!(format!("{err:?}").contains("line"));
}

#[test]
fn factory_advertises_console_provider() {
  let factory = ConsoleHostFactory::with_backend(RecordingConsoleBackend::new()).unwrap();
  assert_eq!(factory.provider_name(), "console");
  let installation = factory.instantiate("console", &mech_runtime::ConfigValue::Map(Default::default())).unwrap();
  assert_eq!(installation.interface.provider, "console");
  assert_eq!(installation.resource_providers.len(), 1);
}

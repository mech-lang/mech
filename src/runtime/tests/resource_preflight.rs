use mech_core::{MResult, Value};
use mech_runtime::{
  RuntimeCapabilityOperation, RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
  RuntimeResourceWritePreflightRequest,
};

#[derive(Debug)]
struct ReadOnlyProvider;

impl RuntimeResourceProvider for ReadOnlyProvider {
  fn scheme(&self) -> &str {
    "readonly"
  }

  fn base_uris(&self) -> Vec<String> {
    vec!["readonly://root".to_string()]
  }

  fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
    Ok(Value::Empty)
  }
}

#[test]
fn default_resource_preflight_rejects_unsupported_write() {
  let provider = ReadOnlyProvider;
  let err = provider.preflight_write(RuntimeResourceWritePreflightRequest {
    base_uri: "readonly://root".to_string(),
    path: "value".to_string(),
    context_name: "ro".to_string(),
    operation: RuntimeCapabilityOperation::Write,
    intent: RuntimeResourceWriteIntent::Assign,
  }).unwrap_err();

  let msg = format!("{:?}", err);
  assert!(
    msg.contains("RuntimeResourceWriteUnsupported"),
    "default preflight should mirror default write unsupported error, got: {msg}"
  );
}

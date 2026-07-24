use std::sync::{Arc, Mutex};

use mech_core::{MResult, Value};
use mech_runtime::{
  materialize_host_manifest, ConfigValue, HostManifestConfig, RuntimeHostFactory,
  RuntimeHostInstallation, RuntimeResourceProvider, RuntimeResourceReadRequest,
  RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

use crate::{console_error, console_host_manifest};

pub trait ConsoleBackend: std::fmt::Debug {
  fn write_line(&mut self, text: &str) -> MResult<()>;
}

#[derive(Clone, Debug, Default)]
pub struct RecordingConsoleBackend {
  lines: Arc<Mutex<Vec<String>>>,
  fail_next: Arc<Mutex<Option<String>>>,
}

impl RecordingConsoleBackend {
  pub fn new() -> Self { Self::default() }
  pub fn lines(&self) -> Vec<String> { self.lines.lock().unwrap().clone() }
  pub fn fail_next(&self, reason: impl Into<String>) { *self.fail_next.lock().unwrap() = Some(reason.into()); }
}

impl ConsoleBackend for RecordingConsoleBackend {
  fn write_line(&mut self, text: &str) -> MResult<()> {
    if let Some(reason) = self.fail_next.lock().unwrap().take() {
      return Err(console_error("console://recording/output", reason));
    }
    self.lines.lock().unwrap().push(text.to_string());
    Ok(())
  }
}

#[derive(Debug)]
pub struct ConsoleResourceProvider<B: ConsoleBackend> {
  instance: String,
  backend: B,
}

impl<B: ConsoleBackend> ConsoleResourceProvider<B> {
  pub fn new(instance: impl Into<String>, backend: B) -> Self {
    Self { instance: instance.into(), backend }
  }

  pub fn backend(&self) -> &B { &self.backend }
  pub fn backend_mut(&mut self) -> &mut B { &mut self.backend }

  fn base(&self) -> String { format!("console://{}/output", self.instance) }

  fn matches_base(&self, base_uri: &str) -> bool {
    base_uri == self.base() || (self.instance == "console" && base_uri == "console://output")
  }
}

impl<B: ConsoleBackend> RuntimeResourceProvider for ConsoleResourceProvider<B> {
  fn scheme(&self) -> &str { "console" }

  fn base_uris(&self) -> Vec<String> {
    let mut bases = vec![self.base()];
    if self.instance == "console" { bases.push("console://output".to_string()); }
    bases
  }

  fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
    if self.instance == "console" {
      vec![vec![self.base(), "console://output".to_string()]]
    } else {
      Vec::new()
    }
  }

  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
    Err(console_error(request.base_uri, "console output is send-only and cannot be read"))
  }

  fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
    if !self.matches_base(&request.base_uri) {
      return Err(console_error(request.base_uri, "unsupported console resource"));
    }
    if request.intent != RuntimeResourceWriteIntent::Send {
      return Err(console_error(request.base_uri, "console output is send-only; use <-"));
    }
    if request.path != "line" {
      return Err(console_error(request.base_uri, "console output supports only the `line` path"));
    }
    Ok(())
  }

  fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
    self.preflight_write(RuntimeResourceWritePreflightRequest {
      base_uri: request.base_uri.clone(),
      path: request.path.clone(),
      context_name: request.context_name.clone(),
      operation: request.operation.clone(),
      intent: request.intent,
    })?;
    self.backend.write_line(&value_to_text(&request.value))
  }
}

fn value_to_text(value: &Value) -> String {
  match value {
    Value::String(value) => value.borrow().clone(),
    other => format!("{}", other),
  }
}

pub fn validate_console_settings(settings: &ConfigValue) -> MResult<()> {
  match settings {
    ConfigValue::Map(map) if map.is_empty() => Ok(()),
    _ => Err(console_error("console://settings", "console host settings must be an empty map")),
  }
}

#[derive(Debug)]
pub struct ConsoleHostFactory<B: ConsoleBackend + Clone> {
  backend: B,
  manifest: HostManifestConfig,
}

impl<B: ConsoleBackend + Clone> ConsoleHostFactory<B> {
  pub fn with_backend(backend: B) -> MResult<Self> {
    Ok(Self { backend, manifest: console_host_manifest()? })
  }
}

impl<B: ConsoleBackend + Clone + 'static> RuntimeHostFactory for ConsoleHostFactory<B> {
  fn provider_name(&self) -> &str { "console" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
    validate_console_settings(settings)
  }
  fn instantiate(&self, instance_name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
    self.validate_settings(instance_name, settings)?;
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, &self.manifest)?,
      resource_providers: vec![Box::new(ConsoleResourceProvider::new(instance_name, self.backend.clone()))],
      input_drivers: Vec::new(),
    })
  }
}

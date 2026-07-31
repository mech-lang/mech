use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind, Ref, Value};
use mech_runtime::*;

#[derive(Debug)]
struct FakeRobotFactory {
  manifest: HostManifestConfig,
  log: Arc<Mutex<Vec<String>>>,
}

impl FakeRobotFactory {
  fn new(log: Arc<Mutex<Vec<String>>>) -> Self {
    Self {
      manifest: HostManifestConfig {
        provider: "fake-robot".to_string(),
        contexts: vec![
          HostContextManifest { name: "state".to_string(), base_uri_template: "fake-robot://{instance}/state".to_string(), operations: vec!["read".to_string()] },
          HostContextManifest { name: "commands".to_string(), base_uri_template: "fake-robot://{instance}/commands".to_string(), operations: vec!["write".to_string()] },
        ],
      },
      log,
    }
  }
}

impl RuntimeHostFactory for FakeRobotFactory {
  fn provider_name(&self) -> &str { "fake-robot" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
    match settings { ConfigValue::Map(_) => Ok(()), _ => Err(fake_error("settings must be a map")) }
  }
  fn instantiate(&self, instance_name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
    self.validate_settings(instance_name, settings)?;
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, &self.manifest)?,
      input_drivers: Vec::new(),
      resource_providers: vec![Box::new(FakeRobotProvider { instance: instance_name.to_string(), log: self.log.clone() })],
    })
  }
}

#[derive(Debug)]
struct FakeRobotProvider {
  instance: String,
  log: Arc<Mutex<Vec<String>>>,
}

impl FakeRobotProvider {
  fn commands_base(&self) -> String { format!("fake-robot://{}/commands", self.instance) }
}

impl RuntimeResourceProvider for FakeRobotProvider {
  fn scheme(&self) -> &str { "fake-robot" }
  fn base_uris(&self) -> Vec<String> { vec![self.commands_base()] }
  fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> { Err(fake_error("fake robot state reads are not implemented in this test")) }
  fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
    if request.base_uri != self.commands_base() { return Err(fake_error("unknown fake robot base URI")); }
    if request.intent != RuntimeResourceWriteIntent::Send { return Err(fake_error("fake robot commands accept only send intent")); }
    match request.path.as_str() {
      "joints/shoulder/target" | "joints/elbow/target" | "joints/wrist/target" | "gripper/closed" | "raw/motor/shoulder" => Ok(()),
      _ => Err(fake_error(format!("unsupported fake robot command path `{}`", request.path))),
    }
  }
  fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
    self.preflight_write(RuntimeResourceWritePreflightRequest { base_uri: request.base_uri.clone(), path: request.path.clone(), context_name: request.context_name.clone(), operation: request.operation.clone(), intent: request.intent })?;
    self.log.lock().unwrap().push(request.path);
    Ok(())
  }
}

#[derive(Debug, Clone)]
struct FakeRobotError { message: String }
impl MechErrorKind for FakeRobotError { fn name(&self) -> &str { "FakeRobotError" } fn message(&self) -> String { self.message.clone() } }
fn fake_error(message: impl Into<String>) -> MechError { MechError::new(FakeRobotError { message: message.into() }, None) }

#[derive(Debug)]
struct AliasProvider {
  bases: Vec<String>,
  equivalent_groups: Vec<Vec<String>>,
}

impl AliasProvider {
  fn new(bases: &[&str]) -> Self {
    Self {
      bases: bases.iter().map(|base| base.to_string()).collect(),
      equivalent_groups: Vec::new(),
    }
  }

  fn with_equivalent_group(mut self, group: &[&str]) -> Self {
    self.equivalent_groups.push(group.iter().map(|base| base.to_string()).collect());
    self
  }
}

impl RuntimeResourceProvider for AliasProvider {
  fn scheme(&self) -> &str { "test" }
  fn base_uris(&self) -> Vec<String> { self.bases.clone() }
  fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> { self.equivalent_groups.clone() }
  fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
    Ok(Value::String(Ref::new("ok".to_string())))
  }
}

fn runtime_with_alias_provider() -> MechRuntime {
  let mut runtime = RuntimeBuilder::new().build().unwrap();
  runtime
    .register_resource_provider(Box::new(
      AliasProvider::new(&[
        "test://default/context",
        "test://context",
      ])
      .with_equivalent_group(&[
        "test://default/context",
        "test://context",
      ]),
    ))
    .unwrap();
  runtime
}

fn grant_test_read(runtime: &mut MechRuntime, resource: &str) {
  runtime
    .grant_capability(RuntimeCapabilityGrant {
      subject: "subject".to_string(),
      resource: resource.to_string(),
      operations: vec![RuntimeCapabilityOperation::Read],
      paths: vec!["item".to_string()],
    })
    .unwrap();
}

#[derive(Debug)]
struct FakeBrowserFactory {
  manifest: HostManifestConfig,
}

impl FakeBrowserFactory {
  fn new() -> Self {
    Self {
      manifest: HostManifestConfig {
        provider: "browser".to_string(),
        contexts: vec![HostContextManifest {
          name: "dom".to_string(),
          base_uri_template: "browser://{instance}/dom".to_string(),
          operations: vec!["read".to_string(), "write".to_string()],
        }],
      },
    }
  }
}

impl RuntimeHostFactory for FakeBrowserFactory {
  fn provider_name(&self) -> &str { "browser" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
    match settings { ConfigValue::Map(_) => Ok(()), _ => Err(fake_error("settings must be a map")) }
  }
  fn instantiate(&self, instance_name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
    self.validate_settings(instance_name, settings)?;
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, &self.manifest)?,
      input_drivers: Vec::new(),
      resource_providers: Vec::new(),
    })
  }
}

fn robot_runtime(log: Arc<Mutex<Vec<String>>>) -> MResult<MechRuntime> {
  RuntimeBuilder::new()
    .host_factory(Box::new(FakeRobotFactory::new(log)))?
    .host_instance(HostInstanceConfig { name: "arm".to_string(), provider: "fake-robot".to_string(), settings: ConfigValue::Map(Default::default()) })
    .run_resource_grant(RunResourceGrantConfig {
      target: "arm/commands".to_string(),
      operations: vec!["write".to_string()],
      paths: vec![
        "joints/shoulder/target".to_string(),
        "joints/elbow/target".to_string(),
        "joints/wrist/target".to_string(),
        "gripper/closed".to_string(),
      ],
    })
    .build()
}

#[test]
fn duplicate_host_instance_registration_fails_generically() {
  let error = RuntimeBuilder::new()
    .host_factory(Box::new(FakeBrowserFactory::new()))
    .unwrap()
    .host_factory(Box::new(FakeRobotFactory::new(Arc::new(Mutex::new(Vec::new())))))
    .unwrap()
    .host_instance(HostInstanceConfig {
      name: "shared".to_string(),
      provider: "browser".to_string(),
      settings: ConfigValue::Map(Default::default()),
    })
    .host_instance(HostInstanceConfig {
      name: "shared".to_string(),
      provider: "fake-robot".to_string(),
      settings: ConfigValue::Map(Default::default()),
    })
    .build()
    .expect_err("duplicate host instance registration should fail");
  let error = format!("{error:?}");
  assert!(error.contains("shared"), "got {error}");
  assert!(error.contains("duplicate") || error.contains("already"), "got {error}");
}

#[test]
fn provider_advertised_alias_grant_authorizes_materialized_base() {
  let mut runtime = runtime_with_alias_provider();
  grant_test_read(&mut runtime, "test://context");

  assert!(runtime.has_capability_grant(
    "subject",
    "test://default/context",
    &RuntimeCapabilityOperation::Read,
    "item",
  ));
}

#[test]
fn provider_advertised_materialized_grant_authorizes_alias_base() {
  let mut runtime = runtime_with_alias_provider();
  grant_test_read(&mut runtime, "test://default/context");

  assert!(runtime.has_capability_grant(
    "subject",
    "test://context",
    &RuntimeCapabilityOperation::Read,
    "item",
  ));
}

#[test]
fn provider_advertised_alias_grant_does_not_authorize_unregistered_base() {
  let mut runtime = runtime_with_alias_provider();
  grant_test_read(&mut runtime, "test://context");

  assert!(!runtime.has_capability_grant(
    "subject",
    "test://other/context",
    &RuntimeCapabilityOperation::Read,
    "item",
  ));
}

#[test]
fn provider_advertised_alias_grants_do_not_use_string_heuristics() {
  let mut runtime = RuntimeBuilder::new().build().unwrap();
  runtime
    .register_resource_provider(Box::new(AliasProvider::new(&["test://context"])))
    .unwrap();
  grant_test_read(&mut runtime, "test://context");

  assert!(!runtime.has_capability_grant(
    "subject",
    "test://default/context",
    &RuntimeCapabilityOperation::Read,
    "item",
  ));
}

#[test]
fn multiple_provider_bases_are_not_implicit_aliases() {
  let mut runtime = RuntimeBuilder::new().build().unwrap();
  runtime
    .register_resource_provider(Box::new(AliasProvider::new(&[
      "test://default/context",
      "test://context",
    ])))
    .unwrap();
  grant_test_read(&mut runtime, "test://context");

  assert!(!runtime.has_capability_grant(
    "subject",
    "test://default/context",
    &RuntimeCapabilityOperation::Read,
    "item",
  ));
}

#[test]
fn in_memory_docs_bases_are_not_implicit_aliases() {
  let mut docs = InMemoryDocsProvider::new();
  docs
    .insert("docs://manual", "title", Value::String(Ref::new("manual".to_string())))
    .unwrap();
  docs
    .insert("docs://guide", "title", Value::String(Ref::new("guide".to_string())))
    .unwrap();

  let mut runtime = RuntimeBuilder::new().build().unwrap();
  runtime.register_resource_provider(Box::new(docs)).unwrap();
  runtime
    .grant_capability(RuntimeCapabilityGrant {
      subject: "subject".to_string(),
      resource: "docs://manual".to_string(),
      operations: vec![RuntimeCapabilityOperation::Read],
      paths: vec!["title".to_string()],
    })
    .unwrap();

  assert!(!runtime.has_capability_grant(
    "subject",
    "docs://guide",
    &RuntimeCapabilityOperation::Read,
    "title",
  ));
}

#[test]
fn host_instance_same_provider_builtin_configures_default() {
  let mut runtime = RuntimeBuilder::new()
    .host_factory(Box::new(FakeBrowserFactory::new()))
    .unwrap()
    .host_instance(HostInstanceConfig {
      name: "browser".to_string(),
      provider: "browser".to_string(),
      settings: ConfigValue::Map(Default::default()),
    })
    .build()
    .unwrap();
  runtime.bind_context_export("dom", "browser", "dom").unwrap();
}

#[test]
fn host_instance_custom_browser_name_succeeds() {
  let mut runtime = RuntimeBuilder::new()
    .host_factory(Box::new(FakeBrowserFactory::new()))
    .unwrap()
    .host_instance(HostInstanceConfig {
      name: "ui".to_string(),
      provider: "browser".to_string(),
      settings: ConfigValue::Map(Default::default()),
    })
    .build()
    .unwrap();
  runtime.bind_context_export("dom", "ui", "dom").unwrap();
}

#[test]
fn fake_robot_safe_program_writes_four_commands() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = robot_runtime(log.clone()).unwrap();
  runtime.run_string("+> @arm := arm/commands\n@arm/joints/shoulder/target <- 0.35\n@arm/joints/elbow/target <- 0.80\n@arm/joints/wrist/target <- -0.45\n@arm/gripper/closed <- true\n").unwrap();
  assert_eq!(log.lock().unwrap().len(), 4);
}

#[test]
fn fake_robot_unsafe_program_fails_before_provider_write() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = robot_runtime(log.clone()).unwrap();
  let error = runtime.run_string("+> @arm := arm/commands\n@arm/joints/shoulder/target <- 0.35\n@arm/raw/motor/shoulder <- 1.0\n").expect_err("unsafe path should fail");
  assert!(format!("{error:?}").contains("raw/motor/shoulder"));
  assert_eq!(log.lock().unwrap().len(), 0);
}

#[derive(Debug)]
struct PlotterFactory {
  manifest: HostManifestConfig,
  log: Arc<Mutex<Vec<String>>>,
}

impl PlotterFactory {
  fn new(operations: &[&str], log: Arc<Mutex<Vec<String>>>) -> Self {
    Self {
      manifest: HostManifestConfig {
        provider: "plotter".to_string(),
        contexts: vec![HostContextManifest {
          name: "commands".to_string(),
          base_uri_template: "plotter://{instance}/commands".to_string(),
          operations: operations.iter().map(|operation| operation.to_string()).collect(),
        }],
      },
      log,
    }
  }
}

impl RuntimeHostFactory for PlotterFactory {
  fn provider_name(&self) -> &str { "plotter" }
  fn manifest(&self) -> &HostManifestConfig { &self.manifest }
  fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
    match settings { ConfigValue::Map(_) => Ok(()), _ => Err(fake_error("settings must be a map")) }
  }
  fn instantiate(&self, instance_name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
    self.validate_settings(instance_name, settings)?;
    Ok(RuntimeHostInstallation {
      interface: materialize_host_manifest(instance_name, self.manifest())?,
      input_drivers: Vec::new(),
      resource_providers: vec![Box::new(PlotterProvider { instance: instance_name.to_string(), log: self.log.clone() })],
    })
  }
}

#[derive(Debug)]
struct PlotterProvider {
  instance: String,
  log: Arc<Mutex<Vec<String>>>,
}

impl PlotterProvider {
  fn base(&self) -> String { format!("plotter://{}/commands", self.instance) }
}

impl RuntimeResourceProvider for PlotterProvider {
  fn scheme(&self) -> &str { "plotter" }
  fn base_uris(&self) -> Vec<String> { vec![self.base()] }
  fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
    if request.base_uri != self.base() { return Err(fake_error("unknown plotter base URI")); }
    match request.path.as_str() {
      "read" => Ok(Value::String(Ref::new("ok".to_string()))),
      _ => Err(fake_error("plotter read path is not implemented in this test")),
    }
  }
  fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
    if request.base_uri != self.base() { return Err(fake_error("unknown plotter base URI")); }
    if request.intent != RuntimeResourceWriteIntent::Send { return Err(fake_error("plotter commands accept only send intent")); }
    match request.path.as_str() {
      "line" | "text" | "read" => Ok(()),
      path if path.starts_with("line/") => Ok(()),
      path if path.starts_with("read/") => Ok(()),
      _ => Err(fake_error(format!("unsupported plotter command path `{}`", request.path))),
    }
  }
  fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
    self.preflight_write(RuntimeResourceWritePreflightRequest { base_uri: request.base_uri.clone(), path: request.path.clone(), context_name: request.context_name.clone(), operation: request.operation.clone(), intent: request.intent })?;
    self.log.lock().unwrap().push(request.operation.name().to_string());
    Ok(())
  }
}

fn plotter_runtime(operations: &[&str], grants: &[&str], log: Arc<Mutex<Vec<String>>>) -> MResult<MechRuntime> {
  RuntimeBuilder::new()
    .host_factory(Box::new(PlotterFactory::new(operations, log)))?
    .host_instance(HostInstanceConfig { name: "plotter".to_string(), provider: "plotter".to_string(), settings: ConfigValue::Map(Default::default()) })
    .run_resource_grant(RunResourceGrantConfig {
      target: "plotter/commands".to_string(),
      operations: grants.iter().map(|grant| grant.to_string()).collect(),
      paths: vec!["line".to_string(), "text".to_string(), "read".to_string(), "read/*".to_string(), "line/safe/*".to_string(), "line/unsafe".to_string()],
    })
    .build()
}

#[test]
fn custom_line_send_is_not_forced_to_write() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["line"], &["line"], log.clone()).unwrap();
  runtime.run_string("+> @plotter := plotter/commands\n@plotter/line <- { x1: 0 y1: 0 x2: 10 y2: 10 }\n").unwrap();
  assert_eq!(log.lock().unwrap().as_slice(), &["line".to_string()]);
}

#[test]
fn custom_text_send_is_not_forced_to_write() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["text"], &["text"], log.clone()).unwrap();
  runtime.run_string("+> @plotter := plotter/commands\n@plotter/text <- { message: \"hello\" }\n").unwrap();
  assert_eq!(log.lock().unwrap().as_slice(), &["text".to_string()]);
}

#[test]
fn legacy_write_only_line_send_still_uses_write() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["write"], &["write"], log.clone()).unwrap();
  runtime.run_string("+> @plotter := plotter/commands\n@plotter/line <- \"hello\"\n").unwrap();
  assert_eq!(log.lock().unwrap().as_slice(), &["write".to_string()]);
}

#[test]
fn context_send_read_path_does_not_use_read_grant() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["read"], &["read"], log.clone()).unwrap();
  let error = runtime.run_string("+> @plotter := plotter/commands\n@plotter/read <- \"bad\"\n").expect_err("read must not authorize send");
  let message = format!("{error:?}");
  assert!(message.contains("context_send") || message.contains("reserved"), "got {message}");
  assert!(message.contains("read"), "got {message}");
  assert!(log.lock().unwrap().is_empty());
}

#[test]
fn context_send_read_subpath_does_not_use_read_grant() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["read"], &["read"], log.clone()).unwrap();
  let error = runtime.run_string("+> @plotter := plotter/commands\n@plotter/read/value <- \"bad\"\n").expect_err("read must not authorize send");
  let message = format!("{error:?}");
  assert!(message.contains("context_send") || message.contains("reserved"), "got {message}");
  assert!(message.contains("read"), "got {message}");
  assert!(log.lock().unwrap().is_empty());
}

#[test]
fn actual_read_path_still_uses_read_grant() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["read"], &["read"], log).unwrap();
  runtime.run_string("+> @plotter := plotter/commands\nvalue := @plotter/read\n").unwrap();
}

#[test]
fn custom_line_preferred_over_write_fallback() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["write", "line"], &["line"], log.clone()).unwrap();
  runtime.run_string("+> @plotter := plotter/commands\n@plotter/line <- { x1: 0 y1: 0 x2: 10 y2: 10 }\n").unwrap();
  assert_eq!(log.lock().unwrap().as_slice(), &["line".to_string()]);
}

#[test]
fn scoped_custom_line_does_not_fallback_to_write() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["write", "line"], &["write", "line"], log.clone()).unwrap();
  let error = runtime.run_string("+> @plotter := plotter/commands{:line(line/safe/*), :write(*)}
@plotter/line/unsafe <- { x1: 0 y1: 0 x2: 10 y2: 10 }
").expect_err("line outside its scoped capability should fail");
  let message = format!("{error:?}");
  assert!(message.contains("line"), "got {message}");
  assert!(!message.contains("Write"), "got {message}");
  assert!(log.lock().unwrap().is_empty());
}

#[test]
fn denied_line_send_reports_candidate_operation() {
  let log = Arc::new(Mutex::new(Vec::new()));
  let mut runtime = plotter_runtime(&["line", "text"], &["text"], log.clone()).unwrap();
  let error = runtime.run_string("+> @plotter := plotter/commands\n@plotter/line <- { x1: 0 y1: 0 x2: 10 y2: 10 }\n").expect_err("missing line grant should fail");
  let message = format!("{error:?}");
  assert!(message.contains("line"), "got {message}");
  assert!(!message.contains("Write"), "got {message}");
  assert!(log.lock().unwrap().is_empty());
}

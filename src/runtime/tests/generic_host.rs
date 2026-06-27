use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind, Value};
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
                    HostContextManifest {
                        name: "state".to_string(),
                        base_uri_template: "fake-robot://{instance}/state".to_string(),
                        operations: vec!["read".to_string()],
                    },
                    HostContextManifest {
                        name: "commands".to_string(),
                        base_uri_template: "fake-robot://{instance}/commands".to_string(),
                        operations: vec!["write".to_string()],
                    },
                ],
            },
            log,
        }
    }
}

impl RuntimeHostFactory for FakeRobotFactory {
    fn provider_name(&self) -> &str {
        "fake-robot"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        match settings {
            ConfigValue::Map(_) => Ok(()),
            _ => Err(fake_error("settings must be a map")),
        }
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(FakeRobotProvider {
                instance: instance_name.to_string(),
                log: self.log.clone(),
            })],
        })
    }
}

#[derive(Debug)]
struct FakeRobotProvider {
    instance: String,
    log: Arc<Mutex<Vec<String>>>,
}

impl FakeRobotProvider {
    fn commands_base(&self) -> String {
        format!("fake-robot://{}/commands", self.instance)
    }
}

impl RuntimeResourceProvider for FakeRobotProvider {
    fn scheme(&self) -> &str {
        "fake-robot"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.commands_base()]
    }
    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(fake_error(
            "fake robot state reads are not implemented in this test",
        ))
    }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != self.commands_base() {
            return Err(fake_error("unknown fake robot base URI"));
        }
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(fake_error("fake robot commands accept only send intent"));
        }
        match request.path.as_str() {
            "joints/shoulder/target"
            | "joints/elbow/target"
            | "joints/wrist/target"
            | "gripper/closed"
            | "raw/motor/shoulder" => Ok(()),
            _ => Err(fake_error(format!(
                "unsupported fake robot command path `{}`",
                request.path
            ))),
        }
    }
    fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;
        self.log.lock().unwrap().push(request.path);
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct FakeRobotError {
    message: String,
}
impl MechErrorKind for FakeRobotError {
    fn name(&self) -> &str {
        "FakeRobotError"
    }
    fn message(&self) -> String {
        self.message.clone()
    }
}
fn fake_error(message: impl Into<String>) -> MechError {
    MechError::new(
        FakeRobotError {
            message: message.into(),
        },
        None,
    )
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
    fn provider_name(&self) -> &str {
        "browser"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        match settings {
            ConfigValue::Map(_) => Ok(()),
            _ => Err(fake_error("settings must be a map")),
        }
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: Vec::new(),
        })
    }
}

fn robot_runtime(log: Arc<Mutex<Vec<String>>>) -> MResult<MechRuntime> {
    RuntimeBuilder::new()
        .host_factory(Box::new(FakeRobotFactory::new(log)))?
        .host_instance(HostInstanceConfig {
            name: "arm".to_string(),
            provider: "fake-robot".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
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
fn host_instance_different_provider_builtin_collision_fails() {
    let error = RuntimeBuilder::new()
        .host_instance(HostInstanceConfig {
            name: "cli".to_string(),
            provider: "browser".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .build()
        .expect_err("different-provider built-in host collision should fail");
    let error = format!("{error:?}");
    assert!(error.contains("cli"), "got {error}");
    assert!(error.contains("built in"), "got {error}");
    assert!(error.contains("browser"), "got {error}");
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
    runtime
        .bind_context_export("dom", "browser", "dom")
        .unwrap();
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

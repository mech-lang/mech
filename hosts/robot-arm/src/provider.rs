use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind, Ref, Value};
use mech_runtime::{
    ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInstallation,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, materialize_host_manifest,
};

#[derive(Debug, Default)]
pub struct RobotArmState {
    pub position: Option<Value>,
    pub gripper: Option<Value>,
    pub last_command: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RobotArmResourceProvider {
    instance: String,
    state: Arc<Mutex<RobotArmState>>,
}

impl RobotArmResourceProvider {
    pub fn new(instance: impl Into<String>) -> Self {
        Self {
            instance: instance.into(),
            state: Arc::new(Mutex::new(RobotArmState::default())),
        }
    }
    pub fn state(&self) -> Arc<Mutex<RobotArmState>> {
        self.state.clone()
    }
    fn base(&self, context: &str) -> String {
        format!("robot://{}/{}", self.instance, context)
    }
    fn matches_base(&self, base_uri: &str, context: &str) -> bool {
        base_uri == self.base(context)
    }
}

impl RuntimeResourceProvider for RobotArmResourceProvider {
    fn scheme(&self) -> &str {
        "robot"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base("commands"), self.base("state")]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if !self.matches_base(&request.base_uri, "state") {
            return Err(robot_error(request.base_uri, "only state can be read"));
        }
        let state = self
            .state
            .lock()
            .map_err(|_| robot_error(request.base_uri.clone(), "robot state lock poisoned"))?;
        match request.path.as_str() {
            "position" => Ok(state.position.clone().unwrap_or(Value::Empty)),
            "gripper" => Ok(state.gripper.clone().unwrap_or(Value::Empty)),
            "last-command" => Ok(state
                .last_command
                .clone()
                .map(|s| Value::String(Ref::new(s)))
                .unwrap_or(Value::Empty)),
            _ => Err(robot_error(
                request.base_uri,
                format!("unsupported robot state path `{}`", request.path),
            )),
        }
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if !self.matches_base(&request.base_uri, "commands") {
            return Err(robot_error(
                request.base_uri,
                "only commands can be written",
            ));
        }
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(robot_error(
                request.base_uri,
                "robot commands require context send (`<-`)",
            ));
        }
        validate_command_target(
            request.base_uri,
            request.operation.name(),
            &request.path,
        )
    }

    fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| robot_error(request.base_uri.clone(), "robot state lock poisoned"))?;
        match (request.operation.name(), request.path.as_str()) {
            ("move", "move") => {
                state.position = Some(request.value);
                state.last_command = Some("move".to_string());
                Ok(())
            }
            ("grip", "grip") => {
                state.gripper = Some(request.value);
                state.last_command = Some("grip".to_string());
                Ok(())
            }
            ("home", "home") => {
                state.position = Some(Value::Empty);
                state.last_command = Some("home".to_string());
                Ok(())
            }
            (operation, path) => Err(robot_error(
                request.base_uri,
                format!("unsupported robot command `{operation}` at path `{path}`"),
            )),
        }
    }
}

#[derive(Debug)]
pub struct RobotArmHostFactory {
    manifest: HostManifestConfig,
}
impl RobotArmHostFactory {
    pub fn new() -> MResult<Self> {
        Ok(Self {
            manifest: crate::robot_arm_host_manifest()?,
        })
    }
}

impl RuntimeHostFactory for RobotArmHostFactory {
    fn provider_name(&self) -> &str {
        "robot-arm"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        match settings {
            ConfigValue::Map(map) if map.is_empty() => Ok(()),
            ConfigValue::Map(map)
                if map.len() == 1
                    && matches!(map.get("backend"), Some(ConfigValue::String(value)) if value == "mock") =>
            {
                Ok(())
            }
            ConfigValue::Map(map) if map.contains_key("backend") => Err(robot_error(
                "robot://settings".to_string(),
                "robot-arm backend must be `mock`",
            )),
            ConfigValue::Map(_) => Err(robot_error(
                "robot://settings".to_string(),
                "unknown robot-arm host setting",
            )),
            _ => Err(robot_error(
                "robot://settings".to_string(),
                "robot-arm settings must be a map",
            )),
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
            input_drivers: Vec::new(),
      resource_providers: vec![Box::new(RobotArmResourceProvider::new(instance_name))],
        })
    }
}

fn validate_command_target(base_uri: String, operation: &str, path: &str) -> MResult<()> {
    match (operation, path) {
        ("move", "move") => Ok(()),
        ("grip", "grip") => Ok(()),
        ("home", "home") => Ok(()),
        (operation, path) => Err(robot_error(
            base_uri,
            format!("unsupported robot command `{operation}` at path `{path}`"),
        )),
    }
}

fn robot_error(resource: String, reason: impl Into<String>) -> MechError {
    MechError::new(
        RobotArmResourceProviderError {
            resource,
            reason: reason.into(),
        },
        None,
    )
}
#[derive(Debug, Clone)]
pub struct RobotArmResourceProviderError {
    pub resource: String,
    pub reason: String,
}
impl MechErrorKind for RobotArmResourceProviderError {
    fn name(&self) -> &str {
        "RobotArmResourceProvider"
    }
    fn message(&self) -> String {
        format!("{}: {}", self.resource, self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_runtime::RuntimeCapabilityOperation;

    fn bool_value(value: bool) -> Value {
        Value::Bool(Ref::new(value))
    }

    #[test]
    fn robot_provider_receives_move_and_grip_operations() {
        let mut provider = RobotArmResourceProvider::new("arm");
        provider
            .write(RuntimeResourceWriteRequest {
                base_uri: "robot://arm/commands".to_string(),
                path: "move".to_string(),
                context_name: "commands".to_string(),
                operation: RuntimeCapabilityOperation::Custom("move".to_string()),
                value: bool_value(true),
                intent: RuntimeResourceWriteIntent::Send,
            })
            .unwrap();
        provider
            .write(RuntimeResourceWriteRequest {
                base_uri: "robot://arm/commands".to_string(),
                path: "grip".to_string(),
                context_name: "commands".to_string(),
                operation: RuntimeCapabilityOperation::Custom("grip".to_string()),
                value: bool_value(false),
                intent: RuntimeResourceWriteIntent::Send,
            })
            .unwrap();
        let state = provider.state.lock().unwrap();
        assert_eq!(state.last_command.as_deref(), Some("grip"));
        assert!(state.position.is_some());
        assert!(state.gripper.is_some());
    }

    fn send_request(operation: &str, path: &str) -> RuntimeResourceWriteRequest {
        RuntimeResourceWriteRequest {
            base_uri: "robot://arm/commands".to_string(),
            path: path.to_string(),
            context_name: "commands".to_string(),
            operation: RuntimeCapabilityOperation::Custom(operation.to_string()),
            value: bool_value(true),
            intent: RuntimeResourceWriteIntent::Send,
        }
    }

    #[test]
    fn robot_provider_accepts_exact_command_paths() {
        let mut provider = RobotArmResourceProvider::new("arm");
        for (operation, path) in [("move", "move"), ("grip", "grip"), ("home", "home")] {
            provider.write(send_request(operation, path)).unwrap();
        }
    }

    #[test]
    fn robot_provider_rejects_command_subpaths() {
        let mut provider = RobotArmResourceProvider::new("arm");
        for (operation, path) in [("move", "move/typo"), ("grip", "grip/closed"), ("home", "home/reset")] {
            let error = provider.write(send_request(operation, path)).expect_err("subpath should be rejected");
            let message = error.display_message();
            assert!(message.contains(operation), "got {message}");
            assert!(message.contains(path), "got {message}");
        }
    }

    #[test]
    fn robot_provider_rejects_mismatched_operation_and_path() {
        let mut provider = RobotArmResourceProvider::new("arm");
        for (operation, path) in [("move", "grip"), ("grip", "move")] {
            let error = provider.write(send_request(operation, path)).expect_err("mismatch should be rejected");
            let message = error.display_message();
            assert!(message.contains(operation), "got {message}");
            assert!(message.contains(path), "got {message}");
        }
    }

    #[test]
    fn robot_provider_rejects_assignment_and_unsupported_operation() {
        let mut provider = RobotArmResourceProvider::new("arm");
        assert!(
            provider
                .write(RuntimeResourceWriteRequest {
                    base_uri: "robot://arm/commands".to_string(),
                    path: "move".to_string(),
                    context_name: "commands".to_string(),
                    operation: RuntimeCapabilityOperation::Write,
                    value: bool_value(true),
                    intent: RuntimeResourceWriteIntent::Assign,
                })
                .is_err()
        );
        assert!(
            provider
                .write(RuntimeResourceWriteRequest {
                    base_uri: "robot://arm/commands".to_string(),
                    path: "dance".to_string(),
                    context_name: "commands".to_string(),
                    operation: RuntimeCapabilityOperation::Custom("dance".to_string()),
                    value: bool_value(true),
                    intent: RuntimeResourceWriteIntent::Send,
                })
                .is_err()
        );
    }
}

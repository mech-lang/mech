use std::sync::{Arc, Mutex};

use mech_core::{
    LegacyValue, MResult, MechError, MechErrorKind, OperationContractDeclaration, Ref,
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeCompensatableEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInstallation, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    materialize_host_manifest,
};

#[derive(Clone, Debug, Default)]
pub struct RobotArmState {
    pub position: Option<LegacyValue>,
    pub gripper: Option<LegacyValue>,
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

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send)
            .then(mech_runtime::prepare_commit_compensate_contract)
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Err(MechError::new(
            NativeResourceReadNotPlannable {
                base_uri: request.base_uri,
                path: request.path,
            },
            None,
        ))
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if !self.matches_base(&request.base_uri, "state") {
            return Err(robot_error(request.base_uri, "only state can be read"));
        }
        let state = self
            .state
            .lock()
            .map_err(|_| robot_error(request.base_uri.clone(), "robot state lock poisoned"))?;
        match request.path.as_str() {
            "position" => Ok(state.position.clone().unwrap_or(LegacyValue::Empty)),
            "gripper" => Ok(state.gripper.clone().unwrap_or(LegacyValue::Empty)),
            "last-command" => Ok(state
                .last_command
                .clone()
                .map(|s| LegacyValue::String(Ref::new(s)))
                .unwrap_or(LegacyValue::Empty)),
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
        validate_command_target(request.base_uri, request.operation.name(), &request.path)
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;
        Ok(PreparedRuntimeEffect::Compensatable(Box::new(
            RobotCommandEffect {
                state: self.state.clone(),
                base_uri: request.base_uri,
                operation: request.operation.name().to_string(),
                path: request.path,
                value: request.value,
                previous: None,
                applied: false,
            },
        )))
    }
}

#[derive(Debug, Clone)]
pub struct NativeResourceReadNotPlannable {
    pub base_uri: String,
    pub path: String,
}

impl MechErrorKind for NativeResourceReadNotPlannable {
    fn name(&self) -> &str {
        "NativeResourceReadNotPlannable"
    }

    fn message(&self) -> String {
        format!(
            "native planning cannot infer one static value type for robot resource `{}/{}`",
            self.base_uri, self.path
        )
    }
}

#[derive(Debug)]
struct RobotCommandEffect {
    state: Arc<Mutex<RobotArmState>>,
    base_uri: String,
    operation: String,
    path: String,
    value: LegacyValue,
    previous: Option<RobotArmState>,
    applied: bool,
}

impl RuntimeCompensatableEffect for RobotCommandEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "robot".to_string(),
            },
            self.operation.clone(),
        )
        .with_resource(format!("{}/{}", self.base_uri, self.path))
        .with_cost(RuntimeEffectCost { bytes: 0, items: 1 })
    }

    fn apply(&mut self) -> MResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| robot_error(self.base_uri.clone(), "robot state lock poisoned"))?;
        let previous = state.clone();
        match (self.operation.as_str(), self.path.as_str()) {
            ("move", "move") => {
                state.position = Some(self.value.clone());
                state.last_command = Some("move".to_string());
            }
            ("grip", "grip") => {
                state.gripper = Some(self.value.clone());
                state.last_command = Some("grip".to_string());
            }
            ("home", "home") => {
                state.position = Some(LegacyValue::Empty);
                state.last_command = Some("home".to_string());
            }
            (operation, path) => Err(robot_error(
                self.base_uri.clone(),
                format!("unsupported robot command `{operation}` at path `{path}`"),
            ))?,
        }
        self.previous = Some(previous);
        self.applied = true;
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        if !self.applied {
            return Ok(());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| robot_error(self.base_uri.clone(), "robot state lock poisoned"))?;
        if let Some(previous) = self.previous.take() {
            *state = previous;
        }
        self.applied = false;
        Ok(())
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
    use mech_runtime::{RuntimeCapabilityOperation, RuntimeResourceProvider};

    fn bool_value(value: bool) -> LegacyValue {
        LegacyValue::Bool(Ref::new(value))
    }

    fn apply_write(
        provider: &mut RobotArmResourceProvider,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        let mut effect = provider.prepare_write(request)?;
        match &mut effect {
            PreparedRuntimeEffect::Compensatable(effect) => effect.apply()?,
            effect => panic!("expected robot compensatable effect, got {effect:?}"),
        }
        Ok(effect)
    }

    #[test]
    fn robot_provider_receives_move_and_grip_operations() {
        let mut provider = RobotArmResourceProvider::new("arm");
        apply_write(
            &mut provider,
            RuntimeResourceWriteRequest {
                base_uri: "robot://arm/commands".to_string(),
                path: "move".to_string(),
                context_name: "commands".to_string(),
                operation: RuntimeCapabilityOperation::Custom("move".to_string()),
                value: bool_value(true),
                intent: RuntimeResourceWriteIntent::Send,
            },
        )
        .unwrap();
        apply_write(
            &mut provider,
            RuntimeResourceWriteRequest {
                base_uri: "robot://arm/commands".to_string(),
                path: "grip".to_string(),
                context_name: "commands".to_string(),
                operation: RuntimeCapabilityOperation::Custom("grip".to_string()),
                value: bool_value(false),
                intent: RuntimeResourceWriteIntent::Send,
            },
        )
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
            apply_write(&mut provider, send_request(operation, path)).unwrap();
        }
    }

    #[test]
    fn robot_provider_rejects_command_subpaths() {
        let mut provider = RobotArmResourceProvider::new("arm");
        for (operation, path) in [
            ("move", "move/typo"),
            ("grip", "grip/closed"),
            ("home", "home/reset"),
        ] {
            let error = provider
                .prepare_write(send_request(operation, path))
                .expect_err("subpath should be rejected");
            let message = error.display_message();
            assert!(message.contains(operation), "got {message}");
            assert!(message.contains(path), "got {message}");
        }
    }

    #[test]
    fn robot_provider_rejects_mismatched_operation_and_path() {
        let mut provider = RobotArmResourceProvider::new("arm");
        for (operation, path) in [("move", "grip"), ("grip", "move")] {
            let error = provider
                .prepare_write(send_request(operation, path))
                .expect_err("mismatch should be rejected");
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
                .prepare_write(RuntimeResourceWriteRequest {
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
                .prepare_write(RuntimeResourceWriteRequest {
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

    #[test]
    fn robot_state_reads_are_explicitly_not_plannable() {
        let provider = RobotArmResourceProvider::new("arm");
        for path in ["position", "gripper", "last-command"] {
            let error = provider
                .plan_read(RuntimeResourceReadRequest {
                    base_uri: "robot://arm/state".to_owned(),
                    path: path.to_owned(),
                    context_name: "state".to_owned(),
                })
                .unwrap_err();
            assert_eq!(error.kind_name(), "NativeResourceReadNotPlannable");
            let message = error.display_message();
            assert!(message.contains("robot://arm/state"), "got {message}");
            assert!(message.contains(path), "got {message}");
        }
    }

    #[test]
    fn robot_effect_compensation_restores_mock_state() {
        let mut provider = RobotArmResourceProvider::new("arm");
        let mut effect = apply_write(&mut provider, send_request("move", "move")).unwrap();
        assert_eq!(
            provider.state.lock().unwrap().last_command.as_deref(),
            Some("move"),
        );

        match &mut effect {
            PreparedRuntimeEffect::Compensatable(effect) => {
                effect.compensate().unwrap();
            }
            effect => panic!("expected robot compensatable effect, got {effect:?}"),
        }

        let state = provider.state.lock().unwrap();
        assert!(state.position.is_none());
        assert!(state.last_command.is_none());
    }
}

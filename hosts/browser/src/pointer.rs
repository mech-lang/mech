//! Pointer host contracts shared by browser planning and live WASM ingress.

use mech_core::{MResult, MechError};
use mech_runtime::{
    ConfigValue, HostContextManifest, HostManifestConfig, RuntimeHostFactory, RuntimeHostInput,
    RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue,
    RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest,
    materialize_host_manifest,
};
use std::sync::{Arc, Mutex};

const POINTER_PATHS: [&str; 4] = ["pulse", "position", "pressed", "delta-seconds"];

pub fn pointer_host_manifest() -> HostManifestConfig {
    HostManifestConfig {
        provider: "pointer".to_owned(),
        contexts: vec![HostContextManifest {
            name: "frame".to_owned(),
            base_uri_template: "pointer://{instance}/frame".to_owned(),
            operations: vec!["read".to_owned()],
        }],
    }
}

pub fn validate_pointer_host_settings(settings: &ConfigValue) -> MResult<()> {
    match settings {
        ConfigValue::Map(map) if map.is_empty() => Ok(()),
        _ => Err(pointer_error("pointer host settings must be an empty map")),
    }
}

fn pointer_error(message: impl Into<String>) -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[derive(Clone, Debug)]
pub struct PointerInputHandle {
    base_uri: Arc<str>,
    state: Arc<Mutex<PointerDriverState>>,
}

#[derive(Debug, Default)]
struct PointerDriverState {
    ingress: Option<RuntimeIngress>,
    pulse: u64,
    live: bool,
}

impl PointerInputHandle {
    pub fn new(instance: impl AsRef<str>) -> Self {
        Self {
            base_uri: format!("pointer://{}/frame", instance.as_ref()).into(),
            state: Arc::new(Mutex::new(PointerDriverState::default())),
        }
    }

    pub fn submit(&self, x: f64, y: f64, pressed: bool, delta_seconds: f64) -> MResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| pointer_error("pointer input state lock is poisoned"))?;
        if !state.live {
            return Err(pointer_error("pointer input host is not running"));
        }
        state.pulse = state.pulse.saturating_add(1);
        let pulse = state.pulse;
        let ingress = state
            .ingress
            .clone()
            .ok_or_else(|| pointer_error("pointer input host is not attached"))?;
        drop(state);
        ingress.submit(RuntimeHostInput::new(vec![
            pointer_update(
                &self.base_uri,
                "pulse",
                RuntimeHostInputValue::F64(pulse as f64),
            )?,
            pointer_update(
                &self.base_uri,
                "position",
                RuntimeHostInputValue::F32Matrix {
                    rows: 2,
                    columns: 1,
                    values: vec![x as f32, y as f32],
                },
            )?,
            pointer_update(
                &self.base_uri,
                "pressed",
                RuntimeHostInputValue::F32(f32::from(pressed)),
            )?,
            pointer_update(
                &self.base_uri,
                "delta-seconds",
                RuntimeHostInputValue::F32(delta_seconds as f32),
            )?,
        ])?)
    }
}

fn pointer_update(
    base_uri: &str,
    path: &str,
    value: RuntimeHostInputValue,
) -> MResult<RuntimeHostInputUpdate> {
    Ok(RuntimeHostInputUpdate {
        source: RuntimeHostInputSource::new(base_uri, path)?,
        value,
    })
}

#[derive(Debug)]
pub struct PointerHostFactory {
    handle: Option<PointerInputHandle>,
    manifest: HostManifestConfig,
}

impl PointerHostFactory {
    /// Use the same provider contracts and initial snapshot during compilation,
    /// without creating an input driver or connecting to browser events.
    pub fn planning() -> Self {
        Self {
            handle: None,
            manifest: pointer_host_manifest(),
        }
    }

    pub fn new(handle: PointerInputHandle) -> Self {
        Self {
            handle: Some(handle),
            manifest: pointer_host_manifest(),
        }
    }
}

impl RuntimeHostFactory for PointerHostFactory {
    fn provider_name(&self) -> &str {
        "pointer"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        validate_pointer_host_settings(settings)
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(PointerResourceProvider {
                instance: instance_name.to_owned(),
            })],
            input_drivers: self
                .handle
                .as_ref()
                .map(|handle| {
                    vec![Box::new(PointerInputDriver {
                        instance: instance_name.to_owned(),
                        state: handle.state.clone(),
                    }) as Box<dyn RuntimeHostInputDriver>]
                })
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
struct PointerResourceProvider {
    instance: String,
}

impl PointerResourceProvider {
    fn base(&self) -> String {
        format!("pointer://{}/frame", self.instance)
    }
    fn value(&self, request: RuntimeResourceReadRequest) -> MResult<mech_core::Value> {
        if request.base_uri != self.base() || !POINTER_PATHS.contains(&request.path.as_str()) {
            return Err(pointer_error(format!(
                "unknown pointer input `{}/{}`",
                request.base_uri, request.path
            )));
        }
        if request.path == "position" {
            RuntimeHostInputValue::F32Matrix {
                rows: 2,
                columns: 1,
                values: vec![0.0, 0.0],
            }
            .into_value()
        } else if request.path == "pulse" {
            RuntimeHostInputValue::F64(0.0).into_value()
        } else {
            RuntimeHostInputValue::F32(0.0).into_value()
        }
    }
}

impl RuntimeResourceProvider for PointerResourceProvider {
    fn scheme(&self) -> &str {
        "pointer"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base()]
    }
    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }
    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<mech_core::Value> {
        self.value(request)
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<mech_core::Value> {
        self.value(request)
    }
}

#[derive(Debug)]
struct PointerInputDriver {
    instance: String,
    state: Arc<Mutex<PointerDriverState>>,
}

impl RuntimeHostInputDriver for PointerInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == format!("pointer://{}/frame", self.instance)
            && POINTER_PATHS.contains(&source.path())
    }
    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| pointer_error("pointer input state lock is poisoned"))?
            .ingress = Some(ingress);
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| pointer_error("pointer input state lock is poisoned"))?
            .live = true;
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| pointer_error("pointer input state lock is poisoned"))?
            .live = false;
        Ok(())
    }
    fn is_live(&self) -> bool {
        self.state.lock().map(|state| state.live).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planning_uses_the_live_pointer_schema_without_an_input_driver() {
        let factory = PointerHostFactory::planning();
        let installation = factory
            .instantiate("mouse", &ConfigValue::Map(Default::default()))
            .unwrap();
        assert!(installation.input_drivers.is_empty());
        assert_eq!(installation.resource_providers.len(), 1);
        let provider = &installation.resource_providers[0];
        assert_eq!(provider.base_uris(), ["pointer://mouse/frame"]);
        for (path, expected) in [
            ("pulse", RuntimeHostInputValue::F64(0.0)),
            (
                "position",
                RuntimeHostInputValue::F32Matrix {
                    rows: 2,
                    columns: 1,
                    values: vec![0.0, 0.0],
                },
            ),
            ("pressed", RuntimeHostInputValue::F32(0.0)),
            ("delta-seconds", RuntimeHostInputValue::F32(0.0)),
        ] {
            let value = provider
                .plan_read(RuntimeResourceReadRequest {
                    context_name: "pointer".into(),
                    base_uri: "pointer://mouse/frame".into(),
                    path: path.into(),
                })
                .unwrap();
            assert_eq!(
                RuntimeHostInputValue::from_numeric_value(&value).unwrap(),
                expected
            );
        }
        assert!(
            provider
                .plan_read(RuntimeResourceReadRequest {
                    context_name: "pointer".into(),
                    base_uri: "pointer://other/frame".into(),
                    path: "pulse".into()
                })
                .is_err()
        );
        assert!(
            provider
                .plan_read(RuntimeResourceReadRequest {
                    context_name: "pointer".into(),
                    base_uri: "pointer://mouse/frame".into(),
                    path: "unknown".into()
                })
                .is_err()
        );
    }

    #[test]
    fn live_and_planning_factories_share_settings_validation() {
        for factory in [
            PointerHostFactory::planning(),
            PointerHostFactory::new(PointerInputHandle::new("mouse")),
        ] {
            assert!(
                factory
                    .instantiate("mouse", &ConfigValue::String("invalid".into()))
                    .is_err()
            );
            assert!(
                factory
                    .instantiate("mouse", &ConfigValue::Map(Default::default()))
                    .is_ok()
            );
        }
    }
}

use std::sync::{Arc, Mutex};

use mech_core::{MResult, Value};
use mech_runtime::{
    ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInstallation,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, materialize_host_manifest,
};

use crate::{
    SceneHostSettings, SceneSnapshot, scene_error, scene_host_manifest, scene_settings_from_config,
};

pub trait SceneBackend: Clone + std::fmt::Debug + 'static {
    fn replace_scene(&mut self, scene: SceneSnapshot) -> MResult<()>;
}

#[derive(Clone, Debug, Default)]
pub struct RecordingSceneBackend {
    latest: Arc<Mutex<Option<SceneSnapshot>>>,
    generations: Arc<Mutex<u64>>,
}
impl RecordingSceneBackend {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn latest(&self) -> Option<SceneSnapshot> {
        self.latest.lock().unwrap().clone()
    }
    pub fn generation(&self) -> u64 {
        *self.generations.lock().unwrap()
    }
}
impl SceneBackend for RecordingSceneBackend {
    fn replace_scene(&mut self, scene: SceneSnapshot) -> MResult<()> {
        *self
            .latest
            .lock()
            .map_err(|_| scene_error("SceneBackend", "scene backend lock is poisoned"))? =
            Some(scene);
        *self
            .generations
            .lock()
            .map_err(|_| scene_error("SceneBackend", "scene generation lock is poisoned"))? += 1;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SceneResourceProvider<B: SceneBackend> {
    instance: String,
    backend: B,
}
impl<B: SceneBackend> SceneResourceProvider<B> {
    pub fn new(instance: impl Into<String>, backend: B) -> Self {
        Self {
            instance: instance.into(),
            backend,
        }
    }
    fn base(&self) -> String {
        format!("scene://{}/frame", self.instance)
    }
}
impl<B: SceneBackend> RuntimeResourceProvider for SceneResourceProvider<B> {
    fn scheme(&self) -> &str {
        "scene"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base()]
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(scene_error(
            "SceneResourceProvider",
            format!("scene resource `{}` is write-only", request.base_uri),
        ))
    }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != self.base() {
            return Err(scene_error(
                "SceneResourceProvider",
                format!("unsupported scene resource `{}`", request.base_uri),
            ));
        }
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(scene_error(
                "SceneResourceProvider",
                "scene accepts send writes only; use <-",
            ));
        }
        if request.path != "replace" {
            return Err(scene_error(
                "SceneResourceProvider",
                "scene frame supports only the `replace` path",
            ));
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
        let scene = SceneSnapshot::from_value(&request.value)?;
        self.backend.replace_scene(scene)
    }
}

#[derive(Debug)]
pub struct SceneHostFactory<B: SceneBackend> {
    backend: B,
    manifest: HostManifestConfig,
}
impl<B: SceneBackend> SceneHostFactory<B> {
    pub fn with_backend(backend: B) -> MResult<Self> {
        Ok(Self {
            backend,
            manifest: scene_host_manifest()?,
        })
    }
}
impl<B: SceneBackend> RuntimeHostFactory for SceneHostFactory<B> {
    fn provider_name(&self) -> &str {
        "scene"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        scene_settings_from_config(settings).map(|_| ())
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        let _settings: SceneHostSettings = scene_settings_from_config(settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(SceneResourceProvider::new(
                instance_name,
                self.backend.clone(),
            ))],
            input_drivers: Vec::new(),
        })
    }
}

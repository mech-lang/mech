use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mech_core::MResult;
use mech_runtime::{
    ConfigValue, HostManifestConfig, RuntimeHostFactory, RuntimeHostInstallation,
    materialize_host_manifest,
};

use crate::{
    RecordingSceneBackend, SceneBackend, SceneHostSettings, SceneResourceProvider, SceneSnapshot,
    scene_error, scene_host_manifest, scene_settings_from_config,
};

#[derive(Clone, Debug, Default)]
pub struct NativeSceneRegistry {
    instances: Arc<Mutex<HashMap<String, RecordingSceneBackend>>>,
}

impl NativeSceneRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&self, instance: impl Into<String>) -> MResult<RecordingSceneBackend> {
        let backend = RecordingSceneBackend::new();
        self.instances
            .lock()
            .map_err(|_| scene_error("NativeSceneRegistry", "scene registry lock is poisoned"))?
            .insert(instance.into(), backend.clone());
        Ok(backend)
    }
    pub fn latest(&self, instance: &str) -> Option<SceneSnapshot> {
        self.instances
            .lock()
            .ok()
            .and_then(|instances| instances.get(instance).cloned())
            .and_then(|backend| backend.latest())
    }
}

#[derive(Clone, Debug)]
pub struct NativeSceneBackend {
    instance: String,
    registry: NativeSceneRegistry,
}

impl NativeSceneBackend {
    fn new(instance: impl Into<String>, registry: NativeSceneRegistry) -> Self {
        Self {
            instance: instance.into(),
            registry,
        }
    }
}

impl SceneBackend for NativeSceneBackend {
    fn replace_scene(&mut self, scene: SceneSnapshot) -> MResult<()> {
        let backend = self
            .registry
            .instances
            .lock()
            .map_err(|_| scene_error("NativeSceneRegistry", "scene registry lock is poisoned"))?
            .get(&self.instance)
            .cloned()
            .ok_or_else(|| {
                scene_error(
                    "NativeSceneRegistry",
                    format!("unknown scene instance `{}`", self.instance),
                )
            })?;
        let mut backend = backend;
        backend.replace_scene(scene)
    }
}

#[derive(Clone, Debug)]
pub struct NativeSceneHostFactory {
    registry: NativeSceneRegistry,
    manifest: HostManifestConfig,
}

impl NativeSceneHostFactory {
    pub fn new() -> MResult<Self> {
        Self::with_registry(NativeSceneRegistry::new())
    }
    pub fn with_registry(registry: NativeSceneRegistry) -> MResult<Self> {
        Ok(Self {
            registry,
            manifest: scene_host_manifest()?,
        })
    }
    pub fn registry(&self) -> NativeSceneRegistry {
        self.registry.clone()
    }
}

impl RuntimeHostFactory for NativeSceneHostFactory {
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
        self.registry.register(instance_name)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(SceneResourceProvider::new(
                instance_name,
                NativeSceneBackend::new(instance_name, self.registry.clone()),
            ))],
            input_drivers: Vec::new(),
        })
    }
}

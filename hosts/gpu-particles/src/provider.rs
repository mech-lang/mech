use std::sync::{Arc, Mutex};

use mech_core::{MResult, Value};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInstallation, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    materialize_host_manifest,
};

use crate::{
    GpuParticleControl, GpuParticleHostSettings, gpu_particle_error, gpu_particle_host_manifest,
    gpu_particle_settings_from_config,
};

pub trait GpuParticleBackend: Clone + std::fmt::Debug + 'static {
    fn configure(&mut self, control: GpuParticleControl) -> MResult<()>;
}

#[derive(Clone, Debug, Default)]
pub struct RecordingGpuParticleBackend {
    controls: Arc<Mutex<Vec<GpuParticleControl>>>,
}

impl RecordingGpuParticleBackend {
    pub fn controls(&self) -> Vec<GpuParticleControl> {
        self.controls.lock().unwrap().clone()
    }
}

impl GpuParticleBackend for RecordingGpuParticleBackend {
    fn configure(&mut self, control: GpuParticleControl) -> MResult<()> {
        self.controls
            .lock()
            .map_err(|_| {
                gpu_particle_error(
                    "GpuParticleBackend",
                    "gpu particle recording backend lock is poisoned",
                )
            })?
            .push(control);
        Ok(())
    }
}

#[derive(Debug)]
pub struct GpuParticleResourceProvider<B: GpuParticleBackend> {
    instance: String,
    max_particles: u32,
    backend: Arc<Mutex<B>>,
    last_accepted: Arc<Mutex<Option<GpuParticleControl>>>,
}

impl<B: GpuParticleBackend> GpuParticleResourceProvider<B> {
    pub fn new(instance: impl Into<String>, max_particles: u32, backend: B) -> Self {
        Self {
            instance: instance.into(),
            max_particles,
            backend: Arc::new(Mutex::new(backend)),
            last_accepted: Arc::new(Mutex::new(None)),
        }
    }

    fn base(&self) -> String {
        format!("gpu-particles://{}/simulation", self.instance)
    }
}

impl<B: GpuParticleBackend> RuntimeResourceProvider for GpuParticleResourceProvider<B> {
    fn scheme(&self) -> &str {
        "gpu-particles"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![self.base()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        Err(gpu_particle_error(
            "GpuParticleResourceProvider",
            format!("gpu particle resource `{}` is write-only", request.base_uri),
        ))
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != self.base() {
            return Err(gpu_particle_error(
                "GpuParticleResourceProvider",
                format!("unsupported gpu particle resource `{}`", request.base_uri),
            ));
        }
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(gpu_particle_error(
                "GpuParticleResourceProvider",
                "gpu particle controls accept send writes only; use <-",
            ));
        }
        if request.path != "control" {
            return Err(gpu_particle_error(
                "GpuParticleResourceProvider",
                "gpu particle simulation supports only the `control` path",
            ));
        }
        Ok(())
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path,
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        GpuParticleControl::from_value(&request.value, self.max_particles).map(|_| ())
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
        let control = GpuParticleControl::from_value(&request.value, self.max_particles)?;
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            GpuParticleControlEffect {
                backend: self.backend.clone(),
                last_accepted: self.last_accepted.clone(),
                control,
                resource: request.base_uri,
            },
        )))
    }
}

#[derive(Debug)]
struct GpuParticleControlEffect<B: GpuParticleBackend> {
    backend: Arc<Mutex<B>>,
    last_accepted: Arc<Mutex<Option<GpuParticleControl>>>,
    control: GpuParticleControl,
    resource: String,
}

impl<B: GpuParticleBackend> RuntimeAfterCommitEffect for GpuParticleControlEffect<B> {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "gpu-particles".to_string(),
            },
            "control",
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost {
            bytes: 20,
            items: 1,
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut last_accepted = self.last_accepted.lock().map_err(|_| {
            gpu_particle_error(
                "GpuParticleResourceProvider",
                "gpu particle acceptance lock is poisoned",
            )
        })?;
        if last_accepted.as_ref() == Some(&self.control) {
            return Ok(());
        }
        self.backend
            .lock()
            .map_err(|_| {
                gpu_particle_error(
                    "GpuParticleResourceProvider",
                    "gpu particle backend lock is poisoned",
                )
            })?
            .configure(self.control.clone())?;
        *last_accepted = Some(self.control.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub struct GpuParticleHostFactory<B: GpuParticleBackend> {
    backend: B,
    manifest: HostManifestConfig,
}

impl<B: GpuParticleBackend> GpuParticleHostFactory<B> {
    pub fn with_backend(backend: B) -> MResult<Self> {
        Ok(Self {
            backend,
            manifest: gpu_particle_host_manifest()?,
        })
    }
}

impl<B: GpuParticleBackend> RuntimeHostFactory for GpuParticleHostFactory<B> {
    fn provider_name(&self) -> &str {
        "gpu-particles"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        gpu_particle_settings_from_config(settings).map(|_| ())
    }

    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        let settings: GpuParticleHostSettings = gpu_particle_settings_from_config(settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(GpuParticleResourceProvider::new(
                instance_name,
                settings.max_particles,
                self.backend.clone(),
            ))],
            input_drivers: Vec::new(),
        })
    }
}

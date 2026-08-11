use js_sys::{Object, Reflect};
use mech_core::MResult;
use wasm_bindgen::prelude::*;

use crate::{
    GpuParticleBackend, GpuParticleControl, GpuParticleHostFactory, GpuParticleHostSettings,
    gpu_particle_error,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        catch,
        js_namespace = MechGpuParticles,
        js_name = configure
    )]
    fn configure_gpu_particles(
        instance: &str,
        selector: &str,
        max_particles: u32,
        control: &JsValue,
    ) -> Result<JsValue, JsValue>;
}

#[derive(Clone, Debug)]
pub struct BrowserGpuParticleBackend {
    instance: String,
    settings: GpuParticleHostSettings,
}

impl BrowserGpuParticleBackend {
    pub fn new(instance: impl Into<String>, settings: GpuParticleHostSettings) -> Self {
        Self {
            instance: instance.into(),
            settings,
        }
    }
}

impl GpuParticleBackend for BrowserGpuParticleBackend {
    fn configure(&mut self, control: GpuParticleControl) -> MResult<()> {
        let value = Object::new();
        set(&value, "particleCount", control.particle_count)?;
        set(&value, "gravity", control.gravity)?;
        set(&value, "drag", control.drag)?;
        set(&value, "pointSize", control.point_size)?;
        set(&value, "timeScale", control.time_scale)?;
        configure_gpu_particles(
            &self.instance,
            &self.settings.selector,
            self.settings.max_particles,
            &value,
        )
        .map_err(|error| {
            gpu_particle_error(
                "BrowserGpuParticleBackend",
                format!("browser GPU particle backend rejected control: {error:?}"),
            )
        })?;
        Ok(())
    }
}

fn set(value: &Object, name: &str, field: impl Into<JsValue>) -> MResult<()> {
    Reflect::set(value, &JsValue::from_str(name), &field.into()).map_err(|error| {
        gpu_particle_error(
            "BrowserGpuParticleBackend",
            format!("failed to encode gpu particle field `{name}`: {error:?}"),
        )
    })?;
    Ok(())
}

#[derive(Debug)]
pub struct BrowserGpuParticleHostFactory {
    manifest: mech_runtime::HostManifestConfig,
}

impl BrowserGpuParticleHostFactory {
    pub fn new() -> MResult<Self> {
        Ok(Self {
            manifest: crate::gpu_particle_host_manifest()?,
        })
    }
}

impl mech_runtime::RuntimeHostFactory for BrowserGpuParticleHostFactory {
    fn provider_name(&self) -> &str {
        "gpu-particles"
    }

    fn manifest(&self) -> &mech_runtime::HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(
        &self,
        _instance_name: &str,
        settings: &mech_runtime::ConfigValue,
    ) -> MResult<()> {
        crate::gpu_particle_settings_from_config(settings).map(|_| ())
    }

    fn instantiate(
        &self,
        instance_name: &str,
        settings: &mech_runtime::ConfigValue,
    ) -> MResult<mech_runtime::RuntimeHostInstallation> {
        let parsed = crate::gpu_particle_settings_from_config(settings)?;
        GpuParticleHostFactory::with_backend(BrowserGpuParticleBackend::new(instance_name, parsed))?
            .instantiate(instance_name, settings)
    }
}

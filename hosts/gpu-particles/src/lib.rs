#![forbid(unsafe_code)]

#[cfg(feature = "browser")]
mod browser;
mod config;
mod module;
mod provider;
mod schema;

#[cfg(feature = "browser")]
pub use browser::{BrowserGpuParticleBackend, BrowserGpuParticleHostFactory};
pub use config::{GpuParticleHostSettings, gpu_particle_settings_from_config};
pub use module::gpu_particle_host_manifest;
pub use provider::{
    GpuParticleBackend, GpuParticleHostFactory, GpuParticleResourceProvider,
    RecordingGpuParticleBackend,
};
pub use schema::GpuParticleControl;

use mech_core::{MechError, MechErrorKind};

#[derive(Clone, Debug)]
pub struct GpuParticleHostError {
    pub name: &'static str,
    pub message: String,
}

impl MechErrorKind for GpuParticleHostError {
    fn name(&self) -> &str {
        self.name
    }

    fn message(&self) -> String {
        self.message.clone()
    }
}

pub(crate) fn gpu_particle_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        GpuParticleHostError {
            name,
            message: message.into(),
        },
        None,
    )
}

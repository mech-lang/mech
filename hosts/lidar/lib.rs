pub mod config;
pub mod module;
pub mod provider;
pub mod snapshot;

#[cfg(feature = "native")]
pub mod native;

pub use config::{lidar_settings_from_config, LidarHostSettings};
pub use module::{lidar_host_manifest, LIDAR_HOST_MCFG};
pub use provider::LidarResourceProvider;
pub use snapshot::{
    lidar_input_base_uri, lidar_source_matches, new_shared_snapshot, LidarSnapshot,
    SharedLidarSnapshot, SCAN_PATHS,
};

#[cfg(feature = "native")]
pub use native::{NativeLidarHostFactory, NativeLidarInputDriver};

use mech_core::{MechError, MechErrorKind};

#[derive(Debug, Clone)]
pub struct LidarHostError {
    pub name: &'static str,
    pub message: String,
}

impl MechErrorKind for LidarHostError {
    fn name(&self) -> &str { self.name }
    fn message(&self) -> String { self.message.clone() }
}

pub(crate) fn lidar_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(LidarHostError { name, message: message.into() }, None)
}

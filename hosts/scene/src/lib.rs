pub mod config;
pub mod module;
pub mod provider;
pub mod schema;

#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "native")]
pub mod native;

pub use config::{SceneHostSettings, SceneRendererKind, scene_settings_from_config};
pub use module::{SCENE_HOST_MCFG, scene_host_manifest};
pub use provider::{RecordingSceneBackend, SceneBackend, SceneHostFactory, SceneResourceProvider};
pub use schema::{CircleElement, LineElement, SceneSnapshot};

#[cfg(feature = "browser")]
pub use browser::{BrowserSceneBackend, BrowserSceneHostFactory, BrowserSceneRegistry};
#[cfg(feature = "native")]
pub use native::{NativeSceneBackend, NativeSceneHostFactory, NativeSceneRegistry};

use mech_core::{MechError, MechErrorKind};

#[derive(Debug, Clone)]
pub struct SceneHostError {
    pub name: &'static str,
    pub message: String,
}
impl MechErrorKind for SceneHostError {
    fn name(&self) -> &str {
        self.name
    }
    fn message(&self) -> String {
        self.message.clone()
    }
}

pub(crate) fn scene_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        SceneHostError {
            name,
            message: message.into(),
        },
        None,
    )
}

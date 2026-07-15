use mech_core::MResult;

use crate::{RecordingSceneBackend, SceneHostFactory};

pub type NativeSceneBackend = RecordingSceneBackend;
pub type NativeSceneHostFactory = SceneHostFactory<NativeSceneBackend>;

impl NativeSceneHostFactory {
    pub fn new() -> MResult<Self> {
        Self::with_backend(NativeSceneBackend::new())
    }
}

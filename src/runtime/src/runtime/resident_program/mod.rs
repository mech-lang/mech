mod admission;
mod authority;
mod execution;
mod input;
mod load;
mod route;

#[cfg(all(test, feature = "resident-production-source"))]
mod tests;

#[cfg(all(test, feature = "resident-production"))]
mod artifact_tests;

pub use input::*;
pub use route::*;

use std::sync::Arc;

use mech_engine::{ProgramArtifact, resident::ReactiveInstance};

use crate::{ResidentExternalCoordinator, RuntimeHostInputSource};

pub(crate) enum ActiveProgramExecution {
    None,
    Legacy,
    ResidentPure(ResidentPureExecution),
    ResidentExternal(ResidentExternalExecution),
}

pub(crate) struct ResidentPureExecution {
    pub(crate) artifact: Arc<ProgramArtifact>,
    pub(crate) instance: ReactiveInstance,
}

pub(crate) struct ResidentExternalExecution {
    pub(crate) artifact: Arc<ProgramArtifact>,
    pub(crate) coordinator: ResidentExternalCoordinator,
    pub(crate) trigger_sources: Box<[RuntimeHostInputSource]>,
}

impl Default for ActiveProgramExecution {
    fn default() -> Self {
        Self::None
    }
}

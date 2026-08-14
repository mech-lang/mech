mod admission;
mod authority;
mod execution;
mod input;
mod load;
mod route;

#[cfg(all(test, feature = "resident-routing-source"))]
mod tests;

#[cfg(all(test, feature = "resident-routing"))]
mod artifact_tests;

pub(crate) use execution::output_value;
pub use input::*;
pub use route::*;

use std::sync::Arc;

use mech_engine::{ProgramArtifact, resident::ReactiveInstance};

use crate::{ResidentExternalCoordinator, RuntimeHostInputSource};
use authority::ResidentGrantSet;

pub(crate) enum ActiveProgramExecution {
    None,
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
    pub(crate) grants: ResidentGrantSet,
}

impl Default for ActiveProgramExecution {
    fn default() -> Self {
        Self::None
    }
}

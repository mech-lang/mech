//! Compilation, activation, execution, and observation of resident programs.
//!
//! Compiler state is ephemeral and owned by [`ProgramCompiler`]. Live program
//! state is owned by [`MechRuntime`](crate::runtime::MechRuntime) through the
//! resident session types below. The engine's resident executor remains a
//! separate lower-level component.

#[cfg(feature = "resident-routing-source")]
mod compiler;
#[cfg(feature = "resident-routing")]
mod diagnostics;
mod drivers;
#[cfg(feature = "resident-external")]
pub mod external;
#[cfg(feature = "resident-routing")]
mod input;
#[cfg(feature = "resident-routing")]
mod loading;
mod query;
#[cfg(feature = "resident-routing")]
mod session;
#[cfg(feature = "resident-routing")]
mod value;

#[cfg(all(test, feature = "resident-routing-source"))]
mod tests;

#[cfg(all(test, feature = "resident-routing"))]
mod artifact_tests;

#[cfg(test)]
mod query_tests;

#[cfg(feature = "resident-routing-source")]
pub(crate) use compiler::ProgramCompilerView;
#[cfg(feature = "resident-routing-source")]
pub use compiler::{CompilerImportValueUnsupported, ProgramCompiler};
#[cfg(feature = "compute")]
pub use compiler::{ComputeRegionCompilation, MixedProgramCompilation};
#[cfg(feature = "resident-external")]
pub use external::*;
#[cfg(feature = "resident-routing")]
pub use input::*;
#[cfg(feature = "resident-routing")]
pub use session::*;
#[cfg(feature = "resident-routing")]
pub(crate) use value::output_value;

#[cfg(feature = "resident-routing")]
use std::sync::Arc;

#[cfg(feature = "resident-routing")]
use mech_engine::{ProgramArtifact, resident::ReactiveInstance};

#[cfg(feature = "resident-routing")]
use crate::RuntimeHostInputSource;
#[cfg(feature = "resident-routing")]
use external::ResidentAdmissionProof;

#[cfg(feature = "resident-routing")]
pub(crate) enum ActiveProgramExecution {
    None,
    ResidentPure(ResidentPureExecution),
    ResidentExternal(ResidentExternalExecution),
}

#[cfg(feature = "resident-routing")]
pub(crate) struct ResidentPureExecution {
    pub(crate) artifact: Arc<ProgramArtifact>,
    pub(crate) instance: ReactiveInstance,
}

#[cfg(feature = "resident-routing")]
pub(crate) struct ResidentExternalExecution {
    pub(crate) artifact: Arc<ProgramArtifact>,
    pub(crate) coordinator: external::ResidentExternalCoordinator,
    pub(crate) trigger_sources: Box<[RuntimeHostInputSource]>,
    pub(crate) grants: ResidentAdmissionProof,
}

#[cfg(feature = "resident-routing")]
impl Default for ActiveProgramExecution {
    fn default() -> Self {
        Self::None
    }
}

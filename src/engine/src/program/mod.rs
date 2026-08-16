mod state;
pub use state::*;

#[cfg(feature = "semantic-compiler")]
mod document_outputs;
#[cfg(feature = "semantic-compiler")]
pub use document_outputs::root_document_output_ids;

#[cfg(feature = "semantic-compiler")]
mod compiler_planning;
#[cfg(feature = "semantic-compiler")]
pub use compiler_planning::{
    CompiledResourceSendOperation, CompilerPlanningConfig, CompilerPlanningLimits,
    CompilerPlanningProgram, ProgramCompilationProduct,
};

#[cfg(all(test, feature = "semantic-compiler"))]
mod bytecode_plan_topology_tests;
#[cfg(all(test, feature = "semantic-compiler"))]
mod context_binding_tests;

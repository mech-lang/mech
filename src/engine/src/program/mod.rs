mod state;
pub use state::*;

#[cfg(feature = "semantic-compiler")]
mod document_outputs;
#[cfg(feature = "semantic-compiler")]
pub(crate) use document_outputs::{
    PROGRAM_OUTPUT_PUBLICATION_ANNOTATION, code_is_program_value, fenced_document_output_id,
};
#[cfg(feature = "semantic-compiler")]
pub use document_outputs::{
    configure_root_document_program_output_capture, insert_root_document_program_output_capture,
    root_document_inline_eval_count, root_document_output_ids, root_document_program_output_id,
};

#[cfg(feature = "semantic-compiler")]
mod compiler_planning;
#[cfg(feature = "semantic-compiler")]
mod compiler_value_source;
#[cfg(feature = "semantic-compiler")]
pub use compiler_planning::{
    CompiledResourceSendOperation, CompilerPlanningConfig, CompilerPlanningLimits,
    CompilerPlanningProgram, ProgramArtifactCompilationProduct, ProgramCompilationProduct,
};

#[cfg(all(test, feature = "semantic-compiler"))]
mod context_binding_tests;

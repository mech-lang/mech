//! Metadata-only lowering of a bounded canonical artifact subset.
//!
//! A plan records selected implementation semantics and explicit native host
//! memory metadata. It is not executable code and makes no AOT claim.
mod ir;
pub use crate::resident::budget::NativeLibraryWorkBudget;
pub use crate::resident::general::native_lowering::{
    NativeArtifactLoweringError, NativeArtifactPlan, lower_pure_native_artifact,
};
pub use ir::*;

mod control;
pub use crate::resident::general::native_match_lowering::{
    NativeMatchArtifactPlan, lower_scalar_match_artifact,
};
pub use control::*;

mod iteration;
pub use crate::resident::general::native_match_lowering::{
    NativeIterationArtifactPlan, lower_scalar_iteration_artifact,
};
pub use iteration::*;

pub(crate) mod iteration_validation;

mod program;
pub use crate::resident::general::native_match_lowering::lower_native_program_artifact;
pub use program::*;

#[cfg(any(feature = "pure_text", feature = "map_ops"))]
pub use crate::resident::general::native_collection_templates::{
    NativeCollectionLexicalSource, NativeCollectionLexicalTemplate,
    NativeCollectionOperationTemplate, NativeCollectionPrimitive,
    native_collection_lexical_template, native_collection_operation_template,
};

#[cfg(any(feature = "pure_text", feature = "map_ops"))]
pub use crate::resident::general::native_collection_calls::{
    NativeCollectionCall, NativeCollectionInvocationError, bind_native_collection_lexical,
    bind_native_collection_operation, bind_native_program_library_call,
};

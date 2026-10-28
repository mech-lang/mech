//! Metadata-only lowering of a bounded canonical artifact subset.
//!
//! A plan records selected implementation semantics and explicit native host
//! memory metadata. It is not executable code and makes no AOT claim.
mod ir;
pub use crate::resident::budget::{
    NativeLibraryWorkBudget, NativeWorkDebitBatch, NativeWorkGroup, NativeWorkerBudgetError,
    NativeWorkerCreditStatistics, NativeWorkerWorkLoan,
};
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

mod collection;
pub use collection::*;

mod collection_validation;
mod fusion_semantics;
pub use fusion_semantics::{
    NativeProgramDefinition, NativeProgramFacts, NativeProgramProducerRole, NativeProgramReadRole,
    NativeUniformWordCollection,
};
pub(crate) mod guarded_histogram_query;
pub(crate) mod histogram_sink;
pub(crate) mod iteration_validation;
pub(crate) mod range_cursor;
pub(crate) mod string_element_map_projection;
pub use string_element_map_projection::NativeStringElementMapProjection;
pub(crate) mod string_span_projection;
pub(crate) mod text_join_sink;

#[cfg(feature = "sequence_buffers")]
mod buffer_frames;
mod phase_validation;
mod program;
#[cfg(feature = "sequence_buffers")]
pub use buffer_frames::{
    NativeBufferFrameAction, NativeBufferFrameField, NativeBufferFrameLane,
    NativeBufferFrameOperation, NativeBufferFrames,
};
mod program_factory_linkage;
mod program_image;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub(crate) mod program_image_proofs;
pub(crate) mod program_image_read;
pub use crate::resident::general::native_match_lowering::{
    lower_native_program_artifact, lower_native_program_artifact_with_definition_table_limit,
};
pub use program::*;
pub use program_image::encode_native_program_plan_image;
pub use program_image_read::values::schemas as decode_native_schema_image;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use program_image_read::{
    decode_build_owned_native_plan_body, decode_build_owned_native_plan_image,
    decode_build_owned_native_plan_value,
};

#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
mod window_query_region;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use window_query_region::NativeWindowRegion;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
mod string_map_region;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use string_map_region::NativeStringMapRegion;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
mod scalar_aggregates;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use scalar_aggregates::{
    NativeAggregateWord, NativeScalarAggregateAction, NativeScalarAggregates,
};
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
mod borrowed_strings;
#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use borrowed_strings::{NativeBorrowedStringAction, NativeBorrowedStringRegion};

#[cfg(feature = "pure_text")]
mod selected_text_join;
#[cfg(feature = "pure_text")]
pub use crate::resident::general::native_collection_calls::{
    NativePrivateStringBuffer, NativeSelectedStringMatrix, NativeTextLineCaptureFacts,
    NativeTextLineChunks, NativeTextLineJoinChunk, NativeTextLineMatrix,
    NativeTextLineMatrixChoice, NativeTextLineWindow,
};
#[cfg(feature = "pure_text")]
pub use selected_text_join::{NativeSelectedTextJoin, NativeSelectedTextJoinRange};

#[cfg(feature = "pure_text")]
mod text_line_matrix;
#[cfg(feature = "pure_text")]
pub use text_line_matrix::NativeTextLineMatrixRegion;

#[cfg(feature = "pure_text")]
mod text_line_frontier;
#[cfg(feature = "pure_text")]
pub use text_line_frontier::{
    NativeTextLineCountFrontier, NativeTextLineCountGuard, NativeTextLineReadFrontier,
    NativeTextLineReadGuard, NativeTextLineRetentionFrontier, NativeTextLineRetentionState,
};

#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use crate::resident::general::native_collection_templates::{
    NativeCollectionLexicalSource, NativeCollectionLexicalTemplate,
    NativeCollectionOperationTemplate, NativeCollectionPrimitive,
    native_collection_lexical_template, native_collection_operation_template,
};

#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use crate::resident::general::native_collection_calls::{
    NativeCollectionCall, NativeCollectionInvocationError, NativePreparedLibraryCall,
    NativePreparedLibraryCallCache, NativeSelectedStringSpanSelector,
    bind_native_collection_lexical, bind_native_collection_operation,
    bind_native_program_library_call, prepare_native_program_library_call,
};

#[cfg(any(
    feature = "pure_text",
    feature = "map_ops",
    feature = "sequence_buffers"
))]
pub use crate::resident::general::native_structural_projection::native_project_structural_value;

pub(crate) mod structural_values;

#[cfg(feature = "map_ops")]
mod map_write_uses;
#[cfg(feature = "map_ops")]
pub use map_write_uses::NativeMapWriteUses;

#[cfg(feature = "map_ops")]
mod map_owner_region;
#[cfg(feature = "map_ops")]
pub use map_owner_region::NativeMapOwnerRegion;

#[cfg(feature = "map_ops")]
mod map_owner_words;
#[cfg(feature = "map_ops")]
pub use map_owner_words::NativeMapOwnerWords;

#[cfg(feature = "sequence_buffers")]
mod unsigned_buffer_call;
#[cfg(feature = "sequence_buffers")]
pub use unsigned_buffer_call::NativeUnsignedBufferCall;

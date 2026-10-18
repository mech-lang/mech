//! Checked canonical-artifact native execution.
//!
//! Runtime-only linked applications do not include the source parser, JIT or
//! code generator. JIT and Object emission share one checked definition pass;
//! JIT and linked invocation share one managed typed evaluator.
mod abi;
#[cfg(feature = "canonical-native")]
mod application;
#[cfg(feature = "canonical-native")]
mod compiler;
mod identity;
mod linked;
mod memory;
mod runtime;
pub use abi::{NATIVE_ARTIFACT_ABI_VERSION, NATIVE_ARTIFACT_ENTRY_SYMBOL, NativeAbiDescriptor};
#[cfg(feature = "canonical-native")]
pub use application::{
    GeneratedLinkedApplication, NativeApplicationError, generate_linked_application,
};
#[cfg(feature = "canonical-native")]
pub use compiler::{
    JitArtifact, NativeCompileError, NativeCompileLimits, NativeObject, emit_object,
    emit_object_with_limits,
};
pub use identity::{
    NATIVE_LINK_IDENTITY_BYTES, NATIVE_TYPED_IR_VERSION, NATIVE_WORK_POLICY_VERSION,
    NativeObjectIdentity,
};
pub use linked::{LinkedArtifact, NativeLinkError};
pub use runtime::{
    NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase, NativeExecutionReport,
};

#[cfg(feature = "canonical-native")]
mod control_abi;
#[cfg(feature = "canonical-native")]
mod control_compiler;
#[cfg(feature = "canonical-native")]
mod control_identity;
#[cfg(feature = "canonical-native")]
mod control_runtime;
#[cfg(feature = "canonical-native")]
pub use control_abi::{NativeMatchDescriptor, NativeMatchVisit};
#[cfg(feature = "canonical-native")]
pub use control_compiler::{
    JitMatchArtifact, NativeMatchObject, emit_match_object, emit_match_object_with_limits,
};
#[cfg(feature = "canonical-native")]
pub use control_runtime::{NativeMatchExecutionError, NativeMatchExecutionReport};

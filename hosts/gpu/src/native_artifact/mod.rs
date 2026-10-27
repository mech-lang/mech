//! Checked canonical-artifact native execution.
//!
//! Runtime-only linked applications do not include the source parser, JIT or
//! code generator. JIT and Object emission share one checked definition pass;
//! JIT and linked invocation share one managed typed evaluator.
mod abi;
#[cfg(feature = "canonical-native")]
mod application;
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
mod benchmark_policy;
#[cfg(feature = "canonical-native")]
mod compiler;
mod identity;
mod linked;
mod memory;
mod runtime;
mod values;
mod work;
pub use crate::native_artifact::abi::{
    NATIVE_ARTIFACT_ABI_VERSION, NATIVE_ARTIFACT_ENTRY_SYMBOL, NativeAbiDescriptor,
};
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::application::{
    GeneratedLinkedApplication, NativeApplicationError, generate_linked_application,
    validate_linked_application_binary_name,
};
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
pub use crate::native_artifact::benchmark_policy::{
    NativeBenchmarkCapabilityKernel, NativeBenchmarkPolicy,
};
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::compiler::{
    JitArtifact, NativeCompileError, NativeCompileLimits, NativeObject, emit_object,
    emit_object_with_limits,
};
pub use crate::native_artifact::identity::{
    NATIVE_LINK_IDENTITY_BYTES, NATIVE_TYPED_IR_VERSION, NATIVE_WORK_POLICY_VERSION,
    NativeObjectIdentity,
};
pub use crate::native_artifact::linked::{LinkedArtifact, NativeLinkError};
pub use crate::native_artifact::runtime::{
    NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase, NativeExecutionReport,
};
pub use crate::native_artifact::values::{
    NativeValueArena, NativeValueError, NativeValueHandle, NativeValueSlot,
};
pub use crate::native_artifact::work::NativeWorkMeter;

mod control_abi;
#[cfg(feature = "canonical-native")]
mod control_application;
#[cfg(feature = "canonical-native")]
mod control_compiler;
mod control_identity;
mod control_linked;
mod control_runtime;
pub use crate::native_artifact::control_abi::{NativeMatchDescriptor, NativeMatchVisit};
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::control_compiler::{
    JitMatchArtifact, NativeMatchObject, emit_match_object, emit_match_object_with_limits,
};
pub use crate::native_artifact::control_runtime::{
    NativeMatchExecutionError, NativeMatchExecutionReport,
};

pub use crate::native_artifact::control_identity::{
    NATIVE_MATCH_LINK_IDENTITY_BYTES, NATIVE_MATCH_TYPED_IR_VERSION,
    NATIVE_MATCH_WORK_POLICY_VERSION, NativeMatchObjectIdentity,
};
pub use crate::native_artifact::control_linked::LinkedMatchArtifact;

#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::control_application::generate_linked_match_application;

mod iteration_abi;
#[cfg(feature = "canonical-native")]
mod iteration_application;

#[cfg(feature = "canonical-native")]
mod iteration_compiler;
mod iteration_identity;
mod iteration_linked;
mod iteration_runtime;
#[cfg(any(
    feature = "canonical-native",
    feature = "canonical-native-program-runtime"
))]
mod program;

#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::linked::LinkedProgramArtifact;

pub use crate::native_artifact::iteration_abi::{NativeIterationDescriptor, NativeIterationVisit};
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::iteration_application::generate_linked_iteration_application;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::iteration_compiler::{
    JitIterationArtifact, NativeIterationObject, emit_iteration_object,
    emit_iteration_object_with_limits,
};
pub use crate::native_artifact::iteration_identity::{
    NATIVE_ITERATION_LINK_IDENTITY_BYTES, NATIVE_ITERATION_TYPED_IR_VERSION,
    NATIVE_ITERATION_WORK_POLICY_VERSION, NativeIterationObjectIdentity,
};
pub use crate::native_artifact::iteration_linked::LinkedIterationArtifact;
pub use crate::native_artifact::iteration_runtime::{
    NativeIterationExecutionError, NativeIterationExecutionReport,
};
#[cfg(feature = "canonical-native-benchmark-policy")]
pub use crate::native_artifact::program::application::generate_linked_benchmark_program_application;
#[cfg(feature = "canonical-native-program-host")]
pub use crate::native_artifact::program::application::generate_linked_hosted_program_application;
#[cfg(feature = "canonical-native-program")]
pub use crate::native_artifact::program::application::generate_linked_program_application;
#[cfg(feature = "canonical-native-program")]
pub use crate::native_artifact::program::compiler::{
    JitProgramArtifact, NativeProgramObject, emit_program_object, emit_program_object_with_limits,
};
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::image::NativeProgramDescriptor;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::image::identity::{
    NATIVE_PROGRAM_LINK_IDENTITY_BYTES, NATIVE_PROGRAM_TYPED_IR_VERSION,
    NATIVE_PROGRAM_WORK_POLICY_VERSION, NativeProgramObjectIdentity,
};
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use crate::native_artifact::program::runtime::NativeProgramExecutionSession;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::runtime::{
    NativeProgramExecutionError, NativeProgramExecutionLimits, NativeProgramExecutionReport,
};

#[cfg(feature = "canonical-native")]
mod codegen;

#[cfg(feature = "canonical-native-runtime")]
pub mod launch;

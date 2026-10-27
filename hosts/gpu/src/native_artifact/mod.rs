//! Checked canonical-artifact native execution.
//!
//! Runtime-only linked applications do not include the source parser, JIT or
//! code generator. JIT and Object emission share one checked definition pass;
//! JIT and linked invocation share one managed typed evaluator.
#[path = "abi.rs"]
pub(in crate::native_artifact) mod abi;
#[cfg(feature = "canonical-native")]
#[path = "application/mod.rs"]
pub(in crate::native_artifact) mod application;
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
#[path = "benchmark_policy/mod.rs"]
pub(in crate::native_artifact) mod benchmark_policy;
#[cfg(feature = "canonical-native")]
#[path = "compiler/mod.rs"]
pub(in crate::native_artifact) mod compiler;
#[path = "identity/mod.rs"]
pub(in crate::native_artifact) mod identity;
#[path = "linked.rs"]
pub(in crate::native_artifact) mod linked;
#[path = "memory/mod.rs"]
pub(in crate::native_artifact) mod memory;
#[path = "runtime.rs"]
pub(in crate::native_artifact) mod runtime;
#[path = "values/mod.rs"]
pub(in crate::native_artifact) mod values;
#[path = "work/mod.rs"]
pub(in crate::native_artifact) mod work;
pub use crate::native_artifact::abi::{
    NATIVE_ARTIFACT_ABI_VERSION, NATIVE_ARTIFACT_ENTRY_SYMBOL, NativeAbiDescriptor,
};
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::application::GeneratedLinkedApplication;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::application::NativeApplicationError;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::application::generate_linked_application;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::application::validate_linked_application_binary_name;
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
pub use crate::native_artifact::benchmark_policy::NativeBenchmarkCapabilityKernel;
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
pub use crate::native_artifact::benchmark_policy::NativeBenchmarkPolicy;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::compiler::JitArtifact;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::compiler::NativeCompileError;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::compiler::NativeCompileLimits;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::compiler::NativeObject;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::compiler::emit_object;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::compiler::emit_object_with_limits;
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

#[path = "control_abi.rs"]
pub(in crate::native_artifact) mod control_abi;
#[cfg(feature = "canonical-native")]
#[path = "control_application.rs"]
pub(in crate::native_artifact) mod control_application;
#[cfg(feature = "canonical-native")]
#[path = "control_compiler.rs"]
pub(in crate::native_artifact) mod control_compiler;
#[path = "control_identity/mod.rs"]
pub(in crate::native_artifact) mod control_identity;
#[path = "control_linked.rs"]
pub(in crate::native_artifact) mod control_linked;
#[path = "control_runtime/mod.rs"]
pub(in crate::native_artifact) mod control_runtime;
pub use crate::native_artifact::control_abi::{NativeMatchDescriptor, NativeMatchVisit};
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::control_compiler::JitMatchArtifact;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::control_compiler::NativeMatchObject;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::control_compiler::emit_match_object;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::control_compiler::emit_match_object_with_limits;
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

#[path = "iteration_abi.rs"]
pub(in crate::native_artifact) mod iteration_abi;
#[cfg(feature = "canonical-native")]
#[path = "iteration_application.rs"]
pub(in crate::native_artifact) mod iteration_application;

#[cfg(feature = "canonical-native")]
#[path = "iteration_compiler.rs"]
pub(in crate::native_artifact) mod iteration_compiler;
#[path = "iteration_identity/mod.rs"]
pub(in crate::native_artifact) mod iteration_identity;
#[path = "iteration_linked.rs"]
pub(in crate::native_artifact) mod iteration_linked;
#[path = "iteration_runtime/mod.rs"]
pub(in crate::native_artifact) mod iteration_runtime;
#[cfg(any(
    feature = "canonical-native",
    feature = "canonical-native-program-runtime"
))]
#[path = "program/mod.rs"]
pub(in crate::native_artifact) mod program;

#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::linked::LinkedProgramArtifact;

pub use crate::native_artifact::iteration_abi::{NativeIterationDescriptor, NativeIterationVisit};
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::iteration_application::generate_linked_iteration_application;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::iteration_compiler::JitIterationArtifact;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::iteration_compiler::NativeIterationObject;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::iteration_compiler::emit_iteration_object;
#[cfg(feature = "canonical-native")]
pub use crate::native_artifact::iteration_compiler::emit_iteration_object_with_limits;
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
pub use crate::native_artifact::program::compiler::JitProgramArtifact;
#[cfg(feature = "canonical-native-program")]
pub use crate::native_artifact::program::compiler::NativeProgramObject;
#[cfg(feature = "canonical-native-program")]
pub use crate::native_artifact::program::compiler::emit_program_object;
#[cfg(feature = "canonical-native-program")]
pub use crate::native_artifact::program::compiler::emit_program_object_with_limits;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::image::NativeProgramDescriptor;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::image::identity::NATIVE_PROGRAM_LINK_IDENTITY_BYTES;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::image::identity::NATIVE_PROGRAM_TYPED_IR_VERSION;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::image::identity::NATIVE_PROGRAM_WORK_POLICY_VERSION;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::image::identity::NativeProgramObjectIdentity;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::runtime::NativeProgramExecutionError;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::runtime::NativeProgramExecutionLimits;
#[cfg(feature = "canonical-native-program-runtime")]
pub use crate::native_artifact::program::runtime::NativeProgramExecutionReport;
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use crate::native_artifact::program::runtime::NativeProgramExecutionSession;

#[cfg(feature = "canonical-native")]
#[path = "codegen/mod.rs"]
pub(in crate::native_artifact) mod codegen;

#[cfg(feature = "canonical-native-runtime")]
#[path = "launch/mod.rs"]
pub mod launch;

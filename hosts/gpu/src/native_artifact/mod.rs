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
pub use abi::{NATIVE_ARTIFACT_ABI_VERSION, NATIVE_ARTIFACT_ENTRY_SYMBOL, NativeAbiDescriptor};
#[cfg(feature = "canonical-native")]
pub use application::{
    GeneratedLinkedApplication, NativeApplicationError, generate_linked_application,
    validate_linked_application_binary_name,
};
#[cfg(feature = "canonical-native-benchmark-policy-runtime")]
pub use benchmark_policy::{NativeBenchmarkCapabilityKernel, NativeBenchmarkPolicy};
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
pub use values::{NativeValueArena, NativeValueError, NativeValueHandle, NativeValueSlot};
pub use work::NativeWorkMeter;

mod control_abi;
#[cfg(feature = "canonical-native")]
mod control_application;
#[cfg(feature = "canonical-native")]
mod control_compiler;
mod control_identity;
mod control_linked;
mod control_runtime;
pub use control_abi::{NativeMatchDescriptor, NativeMatchVisit};
#[cfg(feature = "canonical-native")]
pub use control_compiler::{
    JitMatchArtifact, NativeMatchObject, emit_match_object, emit_match_object_with_limits,
};
pub use control_runtime::{NativeMatchExecutionError, NativeMatchExecutionReport};

pub use control_identity::{
    NATIVE_MATCH_LINK_IDENTITY_BYTES, NATIVE_MATCH_TYPED_IR_VERSION,
    NATIVE_MATCH_WORK_POLICY_VERSION, NativeMatchObjectIdentity,
};
pub use control_linked::LinkedMatchArtifact;

#[cfg(feature = "canonical-native")]
pub use control_application::generate_linked_match_application;

mod iteration_abi;
#[cfg(feature = "canonical-native")]
mod iteration_application;
#[cfg(feature = "canonical-native")]
mod iteration_code_plan;
#[cfg(feature = "canonical-native")]
mod iteration_compiler;
mod iteration_identity;
mod iteration_linked;
mod iteration_runtime;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_abi;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_map_owner_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_map_packed_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_key_domain;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_packed_map;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_map_admission;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_borrowed_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_descriptor;
#[cfg(feature = "canonical-native")]
mod program_hooks;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_identity;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_image_packing;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_line_matrix_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_linked;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_prefix_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_runtime;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_parallel_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_selected_join_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_tail_layout;
#[cfg(feature = "canonical-native-program-runtime")]
mod program_window_histogram_layout;
#[cfg(feature = "canonical-native-program-runtime")]
pub use program_linked::LinkedProgramArtifact;
#[cfg(feature = "canonical-native-program")]
mod program_application;
#[cfg(feature = "canonical-native-program")]
mod program_compiler;
pub use iteration_abi::{NativeIterationDescriptor, NativeIterationVisit};
#[cfg(feature = "canonical-native")]
pub use iteration_application::generate_linked_iteration_application;
#[cfg(feature = "canonical-native")]
pub use iteration_compiler::{
    JitIterationArtifact, NativeIterationObject, emit_iteration_object,
    emit_iteration_object_with_limits,
};
pub use iteration_identity::{
    NATIVE_ITERATION_LINK_IDENTITY_BYTES, NATIVE_ITERATION_TYPED_IR_VERSION,
    NATIVE_ITERATION_WORK_POLICY_VERSION, NativeIterationObjectIdentity,
};
pub use iteration_linked::LinkedIterationArtifact;
pub use iteration_runtime::{NativeIterationExecutionError, NativeIterationExecutionReport};
#[cfg(feature = "canonical-native-benchmark-policy")]
pub use program_application::generate_linked_benchmark_program_application;
#[cfg(feature = "canonical-native-program-host")]
pub use program_application::generate_linked_hosted_program_application;
#[cfg(feature = "canonical-native-program")]
pub use program_application::generate_linked_program_application;
#[cfg(feature = "canonical-native-program")]
pub use program_compiler::{
    JitProgramArtifact, NativeProgramObject, emit_program_object, emit_program_object_with_limits,
};
#[cfg(feature = "canonical-native-program-runtime")]
pub use program_descriptor::NativeProgramDescriptor;
#[cfg(feature = "canonical-native-program-runtime")]
pub use program_identity::{
    NATIVE_PROGRAM_LINK_IDENTITY_BYTES, NATIVE_PROGRAM_TYPED_IR_VERSION,
    NATIVE_PROGRAM_WORK_POLICY_VERSION, NativeProgramObjectIdentity,
};
#[cfg(feature = "canonical-native-program-host-runtime")]
pub use program_runtime::NativeProgramExecutionSession;
#[cfg(feature = "canonical-native-program-runtime")]
pub use program_runtime::{
    NativeProgramExecutionError, NativeProgramExecutionLimits, NativeProgramExecutionReport,
};

#[cfg(feature = "canonical-native-program-runtime")]
mod owned_map_binding;
#[cfg(feature = "canonical-native-program-runtime")]
mod owned_string_map;

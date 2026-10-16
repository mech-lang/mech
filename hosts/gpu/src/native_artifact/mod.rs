//! Checked Cranelift lowering of the canonical artifact scalar plan.
//!
//! The JIT and Object share one definition pass. Object emission alone is not
//! evidence that an executable has been linked or run.
mod abi;
mod compiler;
mod memory;
mod runtime;
pub use abi::{NATIVE_ARTIFACT_ABI_VERSION, NATIVE_ARTIFACT_ENTRY_SYMBOL, NativeAbiDescriptor};
pub use compiler::{
    JitArtifact, NativeCompileError, NativeCompileLimits, NativeObject, emit_object,
    emit_object_with_limits,
};
pub use runtime::{
    NativeExecutionError, NativeExecutionLimits, NativeExecutionPhase, NativeExecutionReport,
};

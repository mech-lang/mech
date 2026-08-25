#![forbid(unsafe_code)]

// The canonical recorder lives with the language's semantic compiler types so
// source-only products can compile ProgramArtifacts without linking the
// bytecode producer package. This crate remains the public bytecode-v1 producer
// facade selected by full compiler distributions.
pub use mech_core::{
    CompileCtx, CompiledBytecode, CompiledComputeRegion, CompiledInstructionRole,
    CompiledIntegrityConstraint, CompiledNodeKind, CompiledSymbolDefinition,
};

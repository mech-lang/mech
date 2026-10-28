//! Compiler-only native emission shared across strategy owners.
pub(in crate::native_artifact) mod backend;
pub(in crate::native_artifact) mod common;
pub(in crate::native_artifact) mod control;

#[cfg(feature = "canonical-native")]
pub(in crate::native_artifact) mod options;

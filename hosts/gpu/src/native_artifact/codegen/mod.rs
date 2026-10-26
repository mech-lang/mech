//! Compiler-only native emission shared across strategy owners.
#[path = "common/mod.rs"]
pub(in crate::native_artifact) mod common;
#[path = "control/mod.rs"]
pub(in crate::native_artifact) mod control;

#[cfg(feature = "canonical-native")]
#[path = "options.rs"]
pub(in crate::native_artifact) mod options;

//! Metadata-only lowering of a bounded canonical artifact subset.
//!
//! A plan records selected implementation semantics and explicit native host
//! memory metadata. It is not executable code and makes no AOT claim.
mod ir;
pub use crate::resident::general::native_lowering::{
    NativeArtifactLoweringError, NativeArtifactPlan, lower_pure_native_artifact,
};
pub use ir::*;

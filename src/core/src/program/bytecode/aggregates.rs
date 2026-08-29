//! Canonical bytecode aggregate reconstruction.

pub use crate::legacy_adapter::{
    canonical_bytecode_composite_children, rebuild_canonical_bytecode_composite,
};

// Compiler compatibility still consumes the quarantined reconstruction
// helpers. Keep their public names stable while their implementation lives at
// the explicit legacy boundary.
#[cfg(feature = "semantic-compiler")]
pub use crate::legacy_adapter::{
    bytecode_composite_children, rebuild_bytecode_composite, validate_bytecode_composite_children,
};

//! Deterministic analysis of validated Mech bytecode.
//!
//! This module deliberately consumes only trusted catalogs and the official
//! bytecode model.  It does not inspect source-level exports or infer Cargo
//! metadata from names embedded in a program.

pub(crate) mod artifact;
#[cfg(test)]
pub(crate) mod bytecode;
pub(crate) mod requirements;
#[cfg(test)]
pub(crate) mod runtime_types;

pub(crate) use requirements::{NativeBytecodeContractResolver, application_requires_hosting};

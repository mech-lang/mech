//! Deterministic analysis of validated Mech bytecode.
//!
//! This module deliberately consumes only trusted catalogs and the official
//! bytecode model.  It does not inspect source-level exports or infer Cargo
//! metadata from names embedded in a program.

pub(crate) mod bytecode;
pub(crate) mod requirements;
pub(crate) mod runtime_types;

pub(crate) use bytecode::analyze_runtime_functions;
pub(crate) use requirements::{analyze_application_requirements, application_requires_hosting};
pub(crate) use runtime_types::analyze_runtime_types;

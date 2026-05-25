//! Runtime module building and storage.
//!
//! Resolvers locate and read source. ModuleBuilder turns resolved source into
//! runtime module records. Runtime store owns persistence and activation.

pub mod record;
pub mod builder;

pub use record::*;
pub use builder::*;
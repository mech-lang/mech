//! Process-local realization adapters from R5 plans to the R6 memory domain.
//!
//! These adapters borrow existing plan records and never add handles or
//! runtime ownership to the artifact or deterministic planner model.

mod realize;
mod resident;

pub use realize::*;
pub use resident::*;

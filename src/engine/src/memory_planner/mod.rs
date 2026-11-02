//! Deterministic program-level memory planning.
//!
//! This layer composes the target-local value and call plans from `mech-core`
//! into graph lifetimes, arena projections, reuse groups, transactions, and
//! per-turn plans. It remains process-local and contains no allocation handle.

mod audit;
mod call;
mod program;
#[cfg(feature = "resident-artifact")]
mod resident;
mod turn;

pub use audit::*;
pub(crate) use call::*;
pub use program::*;
#[cfg(feature = "resident-artifact")]
pub use resident::*;
pub use turn::*;

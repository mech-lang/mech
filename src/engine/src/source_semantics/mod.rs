//! Engine-owned semantics for strictly valid canonical syntax.
//!
//! This frontend consumes typed red-tree views directly. It never constructs
//! or accepts the retiring aggregate parser tree.

mod disposition;
mod frontend;

pub use disposition::*;
pub use frontend::*;

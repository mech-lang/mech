//! Engine-owned semantics for strictly valid canonical syntax.
//!
//! This frontend consumes typed red-tree views directly. It never constructs
//! or accepts the retiring aggregate parser tree.
//!
//! Tuples, records, maps, tables, and tuple structures emit the maintained
//! `core/composite-pack` operation with canonical children and one declared
//! output schema. Its bound canonical constructor validates child identities;
//! field labels and tuple tags never come from diagnostic text during execution.

mod disposition;
mod frontend;

pub use disposition::*;
pub use frontend::*;

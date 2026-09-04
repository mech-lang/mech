//! Deterministic, process-local physical memory planning.
//!
//! R5 plans describe allocations owned by the existing runtime. They contain
//! no allocation handles, pointers, backing storage, or wire representation.

mod budget;
mod derive;
mod error;
mod implementation;
mod model;
mod target;

pub use self::budget::*;
#[cfg(feature = "functions")]
pub use self::derive::*;
pub use self::error::*;
pub use self::implementation::*;
pub use self::model::*;
pub use self::target::*;

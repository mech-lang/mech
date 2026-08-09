//! Deterministic, immutable semantic program artifacts.

mod encoding;
mod ir;
mod model;
mod snapshot;
mod validation;

pub use self::ir::*;
pub use self::model::*;

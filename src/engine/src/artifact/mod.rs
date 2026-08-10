//! Deterministic, immutable semantic program artifacts.

mod compiler;
mod encoding;
mod ir;
mod model;
mod snapshot;
mod validation;

#[cfg(feature = "compiler")]
mod bytecode;

pub use self::compiler::*;
pub use self::ir::*;
pub use self::model::*;

#[cfg(feature = "compiler")]
pub use self::bytecode::*;

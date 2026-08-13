//! Deterministic, immutable semantic program artifacts.

mod compiler;
mod encoding;
mod ir;
mod model;
mod requirements;
mod snapshot;
mod validation;

#[cfg(feature = "artifact-codec")]
mod bytecode;

pub use self::compiler::*;
pub use self::ir::*;
pub use self::model::*;
pub use self::requirements::*;

#[cfg(feature = "artifact-codec")]
pub use self::bytecode::*;

//! Backend-neutral planning and intermediate representations for resident
//! Mech compute regions.

#[cfg(feature = "runtime-values")]
mod activation;
mod diagnostic;
mod fixed_shape;
mod ir;
mod memory;
mod placement;
mod port;
mod program;
mod registry;
mod shape;

#[cfg(feature = "runtime-values")]
pub use activation::*;
pub use diagnostic::*;
pub use fixed_shape::*;
pub use ir::*;
pub use memory::*;
pub use placement::*;
pub use port::*;
pub use program::*;
pub use registry::*;
pub use shape::*;

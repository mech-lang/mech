//! Backend-neutral planning and intermediate representations for resident
//! Mech compute regions.

mod diagnostic;
mod fixed_shape;
mod ir;
mod placement;

pub use diagnostic::*;
pub use fixed_shape::*;
pub use ir::*;
pub use placement::*;

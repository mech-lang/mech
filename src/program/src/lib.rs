//! Program orchestration layer for Mech.
//! This crate is being reintroduced for v0.3.5 with a minimal surface area.

pub mod program;
pub mod runloop;
pub mod persister;

pub use crate::program::{Program, ProgramConfig};
pub use crate::runloop::{ClientMessage, ProgramRunner, RunLoop, RunLoopMessage};
pub use crate::persister::Persister;

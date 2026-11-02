//! Portable semantic operation and port contracts.
//!
//! These contracts describe the resolved meaning of artifact bindings. They do
//! not contain runtime validators, factories, representation choices, kernels,
//! or storage strategies.

mod declaration;
mod encoding;
mod maintained;
mod resolved;
mod validation;

pub use declaration::*;
pub use maintained::*;
pub use resolved::*;
pub use validation::*;

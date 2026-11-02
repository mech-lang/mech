#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod module;
pub use module::*;

#[cfg(feature = "provider")]
pub mod provider;
#[cfg(feature = "provider")]
pub use provider::*;

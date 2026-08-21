#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod authority;
pub mod config;
#[cfg(feature = "provider")]
pub mod provider;

#[cfg(feature = "delegation")]
pub mod delegation;

pub use self::authority::*;
pub use self::config::*;
#[cfg(feature = "provider")]
pub use self::provider::*;

#[cfg(feature = "delegation")]
pub use self::delegation::*;

pub mod module;
pub use self::module::*;

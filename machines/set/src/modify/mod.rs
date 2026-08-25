// modify module (insert/remove etc.)
#[macro_use]
#[cfg(feature = "insert")]
pub mod insert;
#[cfg(feature = "remove")]
pub mod remove;

#[cfg(feature = "insert")]
#[cfg(feature = "source")]
pub use self::insert::*;
#[cfg(feature = "remove")]
#[cfg(feature = "source")]
pub use self::remove::*;

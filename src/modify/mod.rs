// modify module (insert/remove etc.)
#[macro_use]

#[cfg(feature = "insert")]
pub mod insert;
#[cfg(feature = "remove")]
pub mod remove;

#[cfg(feature = "insert")]
pub use self::insert::*;
#[cfg(feature = "remove")]
pub use self::remove::*;
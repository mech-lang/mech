#[macro_use]
pub use crate::*;
#[cfg(feature = "erf")]
pub mod erf;
#[cfg(feature = "erfc")]
pub mod erfc;

#[cfg(feature = "erf")]
pub use self::erf::*;
#[cfg(feature = "erfc")]
pub use self::erfc::*;
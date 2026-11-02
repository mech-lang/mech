pub use crate::*;
#[cfg(feature = "erf")]
pub mod erf;
#[cfg(feature = "erfc")]
pub mod erfc;

#[cfg(all(feature = "erf", feature = "source"))]
pub use self::erf::*;
#[cfg(all(feature = "erfc", feature = "source"))]
pub use self::erfc::*;

#[macro_use]
pub use crate::*;
#[cfg(feature = "exp2")]
pub mod exp2;
#[cfg(feature = "exp10")]
pub mod exp10;

#[cfg(feature = "exp2")]
pub use self::exp2::*;
#[cfg(feature = "exp10")]
pub use self::exp10::*;
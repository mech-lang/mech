#[macro_use]
pub use crate::*;
#[cfg(feature = "lgamma")]
pub mod lgamma;
#[cfg(feature = "tgamma")]
pub mod tgamma;

#[cfg(feature = "lgamma")]
pub use self::lgamma::*;
#[cfg(feature = "tgamma")]
pub use self::tgamma::*;
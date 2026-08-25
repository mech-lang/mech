pub use crate::*;
#[cfg(feature = "lgamma")]
pub mod lgamma;
#[cfg(feature = "tgamma")]
pub mod tgamma;

#[cfg(all(feature = "lgamma", feature = "source"))]
pub use self::lgamma::*;
#[cfg(all(feature = "tgamma", feature = "source"))]
pub use self::tgamma::*;

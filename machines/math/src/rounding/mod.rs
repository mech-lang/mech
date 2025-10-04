#[macro_use]
pub use crate::*;
#[cfg(feature = "ceil")]
pub mod ceil;
#[cfg(feature = "floor")]
pub mod floor;
#[cfg(feature = "rint")]
pub mod rint;
#[cfg(feature = "round")]
pub mod round;
#[cfg(feature = "roundeven")]
pub mod roundeven;
#[cfg(feature = "trunc")]
pub mod trunc;

#[cfg(feature = "ceil")]
pub use self::ceil::*;
#[cfg(feature = "floor")]
pub use self::floor::*;
#[cfg(feature = "rint")]
pub use self::rint::*;
#[cfg(feature = "round")]
pub use self::round::*;
#[cfg(feature = "roundeven")]
pub use self::roundeven::*;
#[cfg(feature = "trunc")]
pub use self::trunc::*;
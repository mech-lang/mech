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

#[cfg(all(feature = "ceil", feature = "source"))]
pub use self::ceil::*;
#[cfg(all(feature = "floor", feature = "source"))]
pub use self::floor::*;
#[cfg(all(feature = "rint", feature = "source"))]
pub use self::rint::*;
#[cfg(all(feature = "round", feature = "source"))]
pub use self::round::*;
#[cfg(all(feature = "roundeven", feature = "source"))]
pub use self::roundeven::*;
#[cfg(all(feature = "trunc", feature = "source"))]
pub use self::trunc::*;

pub use crate::*;
#[cfg(feature = "abs")]
pub mod abs;
#[cfg(feature = "copysign")]
pub mod copysign;
#[cfg(feature = "fdim")]
pub mod fdim;
#[cfg(feature = "fmod")]
pub mod fmod;
#[cfg(feature = "nextafter")]
pub mod nextafter;
#[cfg(feature = "remainder")]
pub mod remainder;

#[cfg(all(feature = "abs", feature = "source"))]
pub use self::abs::*;
#[cfg(all(feature = "copysign", feature = "source"))]
pub use self::copysign::*;
#[cfg(all(feature = "fdim", feature = "source"))]
pub use self::fdim::*;
#[cfg(all(feature = "fmod", feature = "source"))]
pub use self::fmod::*;
#[cfg(all(feature = "nextafter", feature = "source"))]
pub use self::nextafter::*;
#[cfg(all(feature = "remainder", feature = "source"))]
pub use self::remainder::*;

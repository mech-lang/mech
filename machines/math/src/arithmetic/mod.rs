pub use crate::*;
#[cfg(feature = "abs")]
pub mod abs;
#[cfg(all(feature = "copysign", feature = "source"))]
pub mod copysign;
#[cfg(all(feature = "fdim", feature = "source"))]
pub mod fdim;
#[cfg(all(feature = "fmod", feature = "source"))]
pub mod fmod;
#[cfg(all(feature = "nextafter", feature = "source"))]
pub mod nextafter;
#[cfg(all(feature = "remainder", feature = "source"))]
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

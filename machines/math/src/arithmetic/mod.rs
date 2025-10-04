#[macro_use]
pub use crate::*;
#[cfg(feature = "copysign")]
pub mod copysign;
#[cfg(feature = "fabs")]
pub mod fabs;
#[cfg(feature = "fmod")]
pub mod fmod;
#[cfg(feature = "fdim")]
pub mod fdim;
#[cfg(feature = "remainder")]
pub mod remainder;
#[cfg(feature = "nextafter")]
pub mod nextafter;

#[cfg(feature = "copysign")]
pub use self::copysign::*;
#[cfg(feature = "fabs")]
pub use self::fabs::*;
#[cfg(feature = "fmod")]
pub use self::fmod::*;
#[cfg(feature = "fdim")]
pub use self::fdim::*;
#[cfg(feature = "remainder")]
pub use self::remainder::*;
#[cfg(feature = "nextafter")]
pub use self::nextafter::*;
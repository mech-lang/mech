#[macro_use]
pub use crate::*;

#[cfg(feature = "union")]
pub mod union;
//#[cfg(feature = "complement")]
//pub mod complement;
//#[cfg(feature = "difference")]
//pub mod difference;
//#[cfg(feature = "intersection")]
//pub mod intersection;
//
#[cfg(feature = "union")]
pub use self::union::*;
//#[cfg(feature = "complement")]
//pub use self::complement::*;
//#[cfg(feature = "difference")]
//pub use self::difference::*;
//#[cfg(feature = "intersection")]
//pub use self::intersection::*;
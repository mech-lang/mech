// operations module (union/intersect etc.)
#[macro_use]

#[cfg(feature = "cartesian_product")]
pub mod cartesian_product;
//#[cfg(feature = "complement")]
//pub mod complement;
#[cfg(feature = "difference")]
pub mod difference;
#[cfg(feature = "intersection")]
pub mod intersection;
#[cfg(feature = "powerset")]
pub mod powerset;
#[cfg(feature = "symmetric_difference")]
pub mod symmetric_difference;
#[cfg(feature = "union")]
pub mod union;

#[cfg(feature = "cartesian_product")]
pub use self::cartesian_product::*;
//#[cfg(feature = "complement")]
//pub use self::complement::*;
#[cfg(feature = "difference")]
pub use self::difference::*;
#[cfg(feature = "intersection")]
pub use self::intersection::*;
#[cfg(feature = "powerset")]
pub use self::powerset::*;
#[cfg(feature = "symmetric_difference")]
pub use self::symmetric_difference::*;
#[cfg(feature = "union")]
pub use self::union::*;
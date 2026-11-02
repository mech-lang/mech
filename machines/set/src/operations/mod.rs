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
#[cfg(feature = "source")]
pub use self::cartesian_product::*;
//#[cfg(feature = "complement")]
//pub use self::complement::*;
#[cfg(feature = "difference")]
#[cfg(feature = "source")]
pub use self::difference::*;
#[cfg(feature = "intersection")]
#[cfg(feature = "source")]
pub use self::intersection::*;
#[cfg(feature = "powerset")]
#[cfg(feature = "source")]
pub use self::powerset::*;
#[cfg(feature = "symmetric_difference")]
#[cfg(feature = "source")]
pub use self::symmetric_difference::*;
#[cfg(feature = "union")]
#[cfg(feature = "source")]
pub use self::union::*;

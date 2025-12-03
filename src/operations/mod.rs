// operations module (union/intersect etc.)
#[macro_use]

#[cfg(feature = "cartesianproduct")]
pub mod cartesianproduct;
//#[cfg(feature = "complement")]
//pub mod complement;
#[cfg(feature = "difference")]
pub mod difference;
#[cfg(feature = "intersection")]
pub mod intersection;
#[cfg(feature = "powerset")]
pub mod powerset;
#[cfg(feature = "sym_difference")]
pub mod sym_difference;
#[cfg(feature = "union")]
pub mod union;

#[cfg(feature = "cartesianproduct")]
pub use self::cartesianproduct::*;
//#[cfg(feature = "complement")]
//pub use self::complement::*;
#[cfg(feature = "difference")]
pub use self::difference::*;
#[cfg(feature = "intersection")]
pub use self::intersection::*;
#[cfg(feature = "powerset")]
pub use self::powerset::*;
#[cfg(feature = "sym_difference")]
pub use self::sym_difference::*;
#[cfg(feature = "union")]
pub use self::union::*;
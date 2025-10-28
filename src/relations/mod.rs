// relations module (subset/superset etc.)
#[macro_use]

#[cfg(feature = "subset")]
pub mod subset;
#[cfg(feature = "proper_subset")]
pub mod proper_subset;
#[cfg(feature = "superset")]
pub mod superset;
#[cfg(feature = "proper_superset")]
pub mod proper_superset;
#[cfg(feature = "equals")]
pub mod equals;
#[cfg(feature = "not_equals")]
pub mod not_equals;

#[cfg(feature = "subset")]
pub use self::subset::*;
#[cfg(feature = "proper_subset")]
pub use self::proper_subset::*;
#[cfg(feature = "superset")]
pub use self::superset::*;
#[cfg(feature = "proper_superset")]
pub use self::proper_superset::*;
#[cfg(feature = "equals")]
pub use self::equals::*;
#[cfg(feature = "not_equals")]
pub use self::not_equals::*;
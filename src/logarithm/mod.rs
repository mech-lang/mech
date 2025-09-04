#[macro_use]
pub use crate::*;
#[cfg(feature = "log")]
pub mod log;
#[cfg(feature = "log1p")]
pub mod log1p;
#[cfg(feature = "log2")]
pub mod log2;
#[cfg(feature = "log10")]
pub mod log10;

#[cfg(feature = "log")]
pub use self::log::*;
#[cfg(feature = "log1p")]
pub use self::log1p::*;
#[cfg(feature = "log2")]
pub use self::log2::*;
#[cfg(feature = "log10")]
pub use self::log10::*;
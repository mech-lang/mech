pub use crate::*;
#[cfg(feature = "log")]
pub mod log;
#[cfg(feature = "log10")]
pub mod log10;
#[cfg(feature = "log1p")]
pub mod log1p;
#[cfg(feature = "log2")]
pub mod log2;

#[cfg(all(feature = "log", feature = "source"))]
pub use self::log::*;
#[cfg(all(feature = "log1p", feature = "source"))]
pub use self::log1p::*;
#[cfg(all(feature = "log2", feature = "source"))]
pub use self::log2::*;
#[cfg(all(feature = "log10", feature = "source"))]
pub use self::log10::*;

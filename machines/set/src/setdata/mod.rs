// setdata module (size, etc.)
#[macro_use]

#[cfg(all(feature = "size", feature = "u64"))]
pub mod size;

#[cfg(all(feature = "size", feature = "u64"))]
pub use self::size::*;
#[cfg(feature = "matrix")]
pub mod matrix;
#[cfg(feature = "matrix")]
pub use self::matrix::*;

// Temporary compatibility re-exports. Mutable aggregate storage now lives
// exclusively inside the legacy adapter and is removed with that adapter.
#[cfg(feature = "enum")]
pub use crate::legacy_adapter::structures::enums;
#[cfg(feature = "enum")]
pub use crate::legacy_adapter::structures::enums::*;
#[cfg(feature = "map")]
pub use crate::legacy_adapter::structures::map;
#[cfg(feature = "map")]
pub use crate::legacy_adapter::structures::map::*;
#[cfg(feature = "record")]
pub use crate::legacy_adapter::structures::record;
#[cfg(feature = "record")]
pub use crate::legacy_adapter::structures::record::*;
#[cfg(feature = "set")]
pub use crate::legacy_adapter::structures::set;
#[cfg(feature = "set")]
pub use crate::legacy_adapter::structures::set::*;
#[cfg(feature = "table")]
pub use crate::legacy_adapter::structures::table;
#[cfg(feature = "table")]
pub use crate::legacy_adapter::structures::table::*;
#[cfg(feature = "tuple")]
pub use crate::legacy_adapter::structures::tuple;
#[cfg(feature = "tuple")]
pub use crate::legacy_adapter::structures::tuple::*;

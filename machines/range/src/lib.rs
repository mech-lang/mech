#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

#[cfg(feature = "source")]
use paste::paste;

#[cfg(all(feature = "source", feature = "matrixd", not(feature = "matrix1")))]
use nalgebra::DMatrix;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(all(feature = "source", feature = "row_vectord"))]
use nalgebra::RowDVector;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;

#[cfg(feature = "range")]
use num_traits::One;
#[cfg(all(feature = "range", feature = "source"))]
use num_traits::Zero;
use std::fmt::Debug;
use std::ops::*;

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "exclusive")]
pub mod exclusive;
#[cfg(feature = "exclusive")]
pub mod exclusive_increment;
#[cfg(feature = "inclusive")]
pub mod inclusive;
#[cfg(feature = "inclusive")]
pub mod inclusive_increment;

#[cfg(feature = "exclusive")]
pub use self::exclusive::*;
#[cfg(feature = "exclusive")]
pub use self::exclusive_increment::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive_increment::*;

use mech_core::MechErrorKind;

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EmptyRangeError;
impl MechErrorKind for EmptyRangeError {
    fn name(&self) -> &str {
        "EmptyRange"
    }
    fn message(&self) -> String {
        "Range size must be > 0".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct RangeSizeOverflowError;

impl MechErrorKind for RangeSizeOverflowError {
    fn name(&self) -> &str {
        "RangeSizeOverflow"
    }
    fn message(&self) -> String {
        "Range size overflow".to_string()
    }
}

#[macro_export]
macro_rules! range_size_to_usize {
    // Float f32 branch
    ($diff:expr, f32) => {{
        let v: f32 = $diff;
        if v < 0.0 {
            return Err(MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc());
        }
        v as usize
    }};

    // Float f64 branch
    ($diff:expr, f64) => {{
        let v: f64 = $diff;
        if v < 0.0 {
            return Err(MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc());
        }
        v as usize
    }};

    // Integer branch
    ($diff:expr, $ty:ty) => {{
        $diff
            .try_into()
            .map_err(|_| MechError::new(RangeSizeOverflowError {}, None).with_compiler_loc())?
    }};
}

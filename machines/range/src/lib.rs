#![feature(step_trait)]
#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

use paste::paste;

#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vectord")]
use nalgebra::DVector;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;
#[cfg(feature = "rowdvector")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;

use std::ops::*;
#[cfg(feature = "range")]
use num_traits::{Zero, One};
use std::fmt::Debug;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

#[cfg(feature = "exclusive")]
pub mod exclusive;
#[cfg(feature = "inclusive")]
pub mod inclusive;

#[cfg(feature = "exclusive")]
pub use self::exclusive::*;
#[cfg(feature = "inclusive")]
pub use self::inclusive::*;

// ----------------------------------------------------------------------------
// Range Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! register_range {
  ($fxn_name:tt, $scalar:tt, $scalar_string:tt, $row1:tt) => {
    paste! {
      register_descriptor! {
        FunctionDescriptor {
          name: concat!(stringify!($fxn_name), "<", $scalar_string , stringify!($row1), ">") ,
          ptr: $fxn_name::<$scalar, $row1<$scalar>>::new,
        }
      }
    }
  };
}
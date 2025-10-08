use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

use paste::paste;
#[cfg(feature = "matrix")]
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
use std::fmt::Debug;
use std::marker::PhantomData;
#[cfg(any(feature = "num-traits"))]
use num_traits::*;

#[cfg(feature = "access")]
pub mod access;
#[cfg(feature = "assign")]
pub mod assign;
#[cfg(feature = "convert")]
pub mod convert;
#[cfg(feature = "matrix_horzcat")]
pub mod horzcat;
#[cfg(feature = "matrix_vertcat")]
pub mod vertcat;

pub trait LosslessInto<T> {
  fn lossless_into(self) -> T;
}

pub trait LossyFrom<T> {
  fn lossy_from(value: T) -> Self;
}
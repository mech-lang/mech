#![feature(step_trait)]
#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

use paste::paste;
#[cfg(feature = "matrix")]
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
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
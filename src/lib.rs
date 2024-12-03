#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;

use paste::paste;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
use num_traits::*;
use std::fmt::Debug;
use simba::scalar::ClosedNeg;
use num_traits::Pow;
use mech_core::matrix::Matrix;

pub mod access;
pub mod set;
pub mod horzcat;
pub mod matmul;
pub mod transpose;

pub use self::access::*;
pub use self::set::*;
pub use self::horzcat::*;
pub use self::matmul::*;
pub use self::transpose::*;

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------
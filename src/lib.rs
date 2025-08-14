#![feature(step_trait)]
#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate libm;
extern crate nalgebra as na;
extern crate paste;
extern crate simba;

use paste::paste;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
use num_traits::*;
use std::fmt::Debug;
use simba::scalar::ClosedNeg;
use num_traits::Pow;
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
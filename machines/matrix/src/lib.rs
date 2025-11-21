#![no_main]
#![allow(warnings)]
#![feature(where_clause_attrs)]
#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;

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
#[cfg(any(feature = "transpose", feature = "matmul"))]
use num_traits::*;
use std::fmt::Debug;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

use std::fmt::Display;

#[cfg(feature = "matmul")]
pub mod matmul;
#[cfg(feature = "transpose")]
pub mod transpose;
#[cfg(feature = "dot")]
pub mod dot;
#[cfg(feature = "solve")]
pub mod solve;
//pub mod cross;

#[cfg(feature = "matmul")]
pub use self::matmul::*;
#[cfg(feature = "transpose")]
pub use self::transpose::*;
#[cfg(feature = "dot")]
pub use self::dot::*;
#[cfg(feature = "solve")]
pub use self::solve::*;
//pub use self::cross::*;

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------
#![cfg_attr(not(test), no_main)]
#![allow(warnings)]
#![feature(where_clause_attrs)]

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
}

#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;

use paste::paste;

#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;
#[cfg(feature = "vectord")]
use nalgebra::DVector;
#[cfg(any(feature = "matrix1", feature = "matmul"))]
use nalgebra::Matrix1;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "rowdvector")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;

#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
#[cfg(any(feature = "dot", feature = "transpose", feature = "matmul"))]
use num_traits::*;
use std::fmt::Debug;
use std::ops::*;

use std::fmt::Display;

#[cfg(feature = "runtime")]
pub mod catalog;
#[cfg(feature = "runtime")]
pub use self::catalog::*;

#[cfg(feature = "dot")]
pub mod dot;
#[cfg(feature = "matmul")]
pub mod matmul;
#[cfg(feature = "solve")]
pub mod solve;
#[cfg(feature = "transpose")]
pub mod transpose;
//pub mod cross;

#[cfg(feature = "dot")]
pub use self::dot::*;
#[cfg(feature = "matmul")]
pub use self::matmul::*;
#[cfg(feature = "solve")]
pub use self::solve::*;
#[cfg(feature = "transpose")]
pub use self::transpose::*;
//pub use self::cross::*;

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------

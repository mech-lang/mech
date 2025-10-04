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

use std::marker::PhantomData;
use std::fmt::{Debug, Display};

use mech_core::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::console;

#[cfg(target_arch = "wasm32")]
macro_rules! log {
  ( $( $t:tt )* ) => {
    web_sys::console::log_1(&format!( $( $t )* ).into());
  }
}

#[cfg(feature = "print")]
pub mod print;
#[cfg(feature = "println")]
pub mod println;

#[cfg(feature = "print")]
pub use self::print::*;
#[cfg(feature = "println")]
pub use self::println::*;

#[macro_export]
macro_rules! register_op {
  ($op:ident, $type:ty, $size:ty, $size_string:tt) => {
    paste!{
      inventory::submit! {
        FunctionDescriptor {
          name: concat!(stringify!($op),"<[",stringify!([<$type:lower>]),"]:", $size_string, ">"),
          ptr: $op::<$type,$size<$type>>::new,
        }
      }
    }
  };}

#[macro_export]
macro_rules! register_op_all {
  ($op:ident, $ty:ty, $ty_feature:literal) => {
    #[cfg(feature = "row_vector4")]
    register_op!($op, $ty, RowVector4, "1,4");
    #[cfg(feature = "row_vector3")]
    register_op!($op, $ty, RowVector3, "1,3");
    #[cfg(feature = "row_vector2")]
    register_op!($op, $ty, RowVector2, "1,2");
    #[cfg(feature = "vector2")]
    register_op!($op, $ty, Vector2, "2,1");
    #[cfg(feature = "vector3")]
    register_op!($op, $ty, Vector3, "3,1");
    #[cfg(feature = "vector4")]
    register_op!($op, $ty, Vector4, "4,1");
    #[cfg(feature = "matrix1")]
    register_op!($op, $ty, Matrix1, "1,1");
    #[cfg(feature = "matrix2")]
    register_op!($op, $ty, Matrix2, "2,2");
    #[cfg(feature = "matrix3")]
    register_op!($op, $ty, Matrix3, "3,3");
    #[cfg(feature = "matrix4")]
    register_op!($op, $ty, Matrix4, "4,4");
    #[cfg(feature = "matrix2x3")]
    register_op!($op, $ty, Matrix2x3, "2,3");
    #[cfg(feature = "matrix3x2")]
    register_op!($op, $ty, Matrix3x2, "3,2");
    #[cfg(feature = "vectord")]
    register_op!($op, $ty, DVector, "0,1");
    #[cfg(feature = "matrixd")]
    register_op!($op, $ty, DMatrix, "0,0");
    #[cfg(feature = "row_vectord")]
    register_op!($op, $ty, RowDVector, "1,0");
  };
}

#[macro_export]
macro_rules! impl_register_all {
  ($macro_name:ident) => {
    #[cfg(feature = "u8")]
    register_op_all!($macro_name, u8, "u8");
    #[cfg(feature = "u16")]
    register_op_all!($macro_name, u16, "u16");
    #[cfg(feature = "u32")]
    register_op_all!($macro_name, u32, "u32");
    #[cfg(feature = "u64")]
    register_op_all!($macro_name, u64, "u64");
    #[cfg(feature = "u128")]
    register_op_all!($macro_name, u128, "u128");
    #[cfg(feature = "i8")]
    register_op_all!($macro_name, i8, "i8");
    #[cfg(feature = "i16")]
    register_op_all!($macro_name, i16, "i16");
    #[cfg(feature = "i32")]
    register_op_all!($macro_name, i32, "i32");
    #[cfg(feature = "i64")]
    register_op_all!($macro_name, i64, "i64");
    #[cfg(feature = "i128")]
    register_op_all!($macro_name, i128, "i128");
    #[cfg(feature = "f32")]
    register_op_all!($macro_name, F32, "f32");
    #[cfg(feature = "f64")]
    register_op_all!($macro_name, F64, "f64");
    #[cfg(feature = "r64")]
    register_op_all!($macro_name, R64, "r64");
    #[cfg(feature = "c64")]
    register_op_all!($macro_name, C64, "c64");
  };
}
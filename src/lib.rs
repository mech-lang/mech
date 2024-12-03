#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use paste::paste;
use mech_core::matrix::Matrix;
use std::fmt::Debug;

pub mod gt;
pub mod lt;
pub mod lte;
pub mod gte;
pub mod eq;
pub mod neq;

pub use self::gt::*;
pub use self::lt::*;
pub use self::lte::*;
pub use self::gte::*;
pub use self::eq::*;
pub use self::neq::*;

// ----------------------------------------------------------------------------
// Compare Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_compare_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
      #[derive(Debug)]
      struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
      }
      impl<T> MechFunction for $struct_name<T>
      where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
      {
      fn solve(&self) {
          let lhs_ptr = self.lhs.as_ptr();
          let rhs_ptr = self.rhs.as_ptr();
          let out_ptr = self.out.as_ptr();
          $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
      }};}

#[macro_export]
macro_rules! impl_compare_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_compare_binop);
  }
}

#[macro_export]
macro_rules! impl_compare_fxns_bool {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_compare_binop);
  }
}
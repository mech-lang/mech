#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
extern crate paste;

use mech_core::*;
#[cfg(feature = "matrix")]
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use paste::paste;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

#[cfg(feature = "gt")]
pub mod gt;
#[cfg(feature = "lt")]
pub mod lt;
#[cfg(feature = "lte")]
pub mod lte;
#[cfg(feature = "gte")]
pub mod gte;
#[cfg(feature = "eq")]
pub mod eq;
#[cfg(feature = "neq")]
pub mod neq;

#[cfg(feature = "gt")]
pub use self::gt::*;
#[cfg(feature = "lt")]
pub use self::lt::*;
#[cfg(feature = "lte")]
pub use self::lte::*;
#[cfg(feature = "gte")]
pub use self::gte::*;
#[cfg(feature = "eq")]
pub use self::eq::*;
#[cfg(feature = "neq")]
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
    T: std::fmt::Debug + Clone + Sync + Send + 'static + 
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
    fn to_string(&self) -> String { format!("{:#?}", self) }
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
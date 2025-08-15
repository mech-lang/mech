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
use num_traits::*;
use std::ops::*;
use std::fmt::Debug;

#[cfg(feature = "sum")]
pub mod sum_column;
#[cfg(feature = "sum")]
pub mod sum_row;

#[cfg(feature = "sum")]
pub use self::sum_column::*;
#[cfg(feature = "sum")]
pub use self::sum_row::*;

#[macro_export]  
macro_rules! impl_stats_urop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      Add<Output = T> + AddAssign +
      Zero + One +
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}


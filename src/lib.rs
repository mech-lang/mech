#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate paste;

use mech_core::*;

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

use paste::paste;
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
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
    }};}


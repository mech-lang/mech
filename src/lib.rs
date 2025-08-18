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

#[cfg(feature = "or")]
pub mod or;
#[cfg(feature = "and")]
pub mod and;
#[cfg(feature = "not")]
pub mod not;
#[cfg(feature = "xor")]
pub mod xor;

#[cfg(feature = "or")]
pub use self::or::*;
#[cfg(feature = "and")]
pub use self::and::*;
#[cfg(feature = "not")]
pub use self::not::*;
#[cfg(feature = "xor")]
pub use self::xor::*;

// ----------------------------------------------------------------------------
// Logic Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_logic_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name {
      fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
    }};}

#[macro_export]
macro_rules! impl_logic_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,bool,bool,impl_logic_binop);
  }
}
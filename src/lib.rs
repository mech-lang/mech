#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
#[cfg(feature = "matrix")]
extern crate nalgebra as na;
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
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
    #[derive(Debug)]
    struct $struct_name {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl MechFunctionImpl for $struct_name {
      fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for $struct_name
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<bool>", stringify!($struct_name));
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx, $feature_flag);
      }
    }};}

#[macro_export]
macro_rules! impl_logic_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,bool,bool,impl_logic_binop);
  }
}
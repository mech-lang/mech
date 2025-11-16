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
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunctionFactory for $struct_name<T>
    where
      T: std::fmt::Debug + Clone + Sync + Send + 'static + 
      ConstElem + CompileConst + AsValueKind +
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let lhs: Ref<$arg1_type> = unsafe { arg1.as_unchecked() }.clone();
            let rhs: Ref<$arg2_type> = unsafe { arg2.as_unchecked() }.clone();
            let out: Ref<$out_type> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self {lhs, rhs, out }))
          },
          _ => Err(MechError2::new(
              IncorrectNumberOfArguments { expected: 2, found: args.len() }, 
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: std::fmt::Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
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
  impl<T> MechFunctionCompiler for $struct_name<T> 
  where
    T: ConstElem + CompileConst + AsValueKind
  {
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
      let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
      compile_binop!(name, self.out, self.lhs, self.rhs, ctx, $feature_flag);
    }
  }
  register_fxn_descriptor!($struct_name, bool, "bool", String, "string", u8, "u8", i8, "i8", u16, "u16", i16, "i16", u32, "u32", i32, "i32", u64, "u64", i64, "i64", u128, "u128", i128, "i128", F32, "f32", F64, "f64", R64, "r64", C64, "c64");
};}
  

#[macro_export]
macro_rules! impl_compare_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,bool,impl_compare_binop);
  }
}
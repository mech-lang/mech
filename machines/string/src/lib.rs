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

#[cfg(feature = "concat")]
pub mod concat;

#[cfg(feature = "concat")]
pub use self::concat::*;

// ----------------------------------------------------------------------------
// String Library
// ----------------------------------------------------------------------------

pub trait Concat {
  fn concat(&self, rhs: &Self) -> Self;
}

impl Concat for String {
  fn concat(&self, rhs: &Self) -> Self {
    let mut s = self.clone();
    s.push_str(rhs);
    s
  }
}

#[macro_export]
macro_rules! impl_string_binop {
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
      ConstElem + CompileConst + AsValueKind + Concat,
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
          _ => Err(MechError::new(
              IncorrectNumberOfArguments { expected: 2, found: args.len() }, 
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: std::fmt::Debug + Clone + Sync + Send + 'static + Concat,
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
  register_fxn_descriptor!($struct_name, String, "string");
};}

#[macro_export]
macro_rules! impl_string_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_string_binop);
  }
}
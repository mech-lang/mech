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
macro_rules! impl_stats_unop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunctionFactory for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      Add<Output = T> + AddAssign +
      CompileConst + ConstElem + AsValueKind +
      Zero + One +
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Unary(out, arg) => {
            let arg = unsafe{ arg.as_unchecked().clone() };
            let out = unsafe{ out.as_unchecked().clone() };
            Ok(Box::new($struct_name { arg, out }))
          }
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
    }
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: CompileConst + ConstElem + AsValueKind,
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Custom(hash_str("stats/sum")) );
      }
    }};}

#[macro_export]    
macro_rules! impls_stas {
  ($name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    impl_stats_unop!($name, $arg_type, $out_type, $op);
    register_fxn_descriptor!($name, u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", F32, "f32", F64, "f64", C64, "complex", R64, "rational");
  };
}

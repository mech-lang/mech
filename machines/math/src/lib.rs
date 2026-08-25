#![cfg_attr(not(test), no_main)]
#![feature(where_clause_attrs)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "matrix")]
extern crate nalgebra as na;

#[cfg(all(not(feature = "dynamic-module"), feature = "runtime"))]
use mech_core::*;

#[cfg(all(not(feature = "dynamic-module"), feature = "math"))]
use paste::paste;

#[cfg(feature = "matrixd")]
use na::DMatrix;
#[cfg(feature = "vectord")]
use na::DVector;
#[cfg(any(feature = "matrix1", feature = "matrix1_interop"))]
use na::Matrix1;
#[cfg(feature = "matrix2")]
use na::Matrix2;
#[cfg(feature = "matrix2x3")]
use na::Matrix2x3;
#[cfg(feature = "matrix3")]
use na::Matrix3;
#[cfg(feature = "matrix3x2")]
use na::Matrix3x2;
#[cfg(feature = "matrix4")]
use na::Matrix4;
#[cfg(feature = "row_vectord")]
use na::RowDVector;
#[cfg(feature = "row_vector2")]
use na::RowVector2;
#[cfg(feature = "row_vector3")]
use na::RowVector3;
#[cfg(feature = "row_vector4")]
use na::RowVector4;
#[cfg(feature = "vector2")]
use na::Vector2;
#[cfg(feature = "vector3")]
use na::Vector3;
#[cfg(feature = "vector4")]
use na::Vector4;

use std::fmt::{Debug, Display};
#[cfg(any(feature = "neg", feature = "op_assign"))]
use std::marker::PhantomData;
use std::ops::*;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathArithmeticOverflow {
    pub operation: &'static str,
    pub operand_type: &'static str,
}

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
impl MechErrorKind for MathArithmeticOverflow {
    fn name(&self) -> &str {
        "MathArithmeticOverflow"
    }

    fn message(&self) -> String {
        format!(
            "{} overflows operand type {}",
            self.operation, self.operand_type,
        )
    }
}

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub(crate) fn arithmetic_overflow<T>(operation: &'static str) -> MechError {
    MechError::new(
        MathArithmeticOverflow {
            operation,
            operand_type: std::any::type_name::<T>(),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(any(feature = "round", feature = "dynamic-module"))]
pub mod kernels;

#[cfg(feature = "dynamic-module")]
mod dynamic_module;

#[cfg(feature = "arithmetic")]
pub mod arithmetic;
#[cfg(feature = "bessel")]
pub mod bessel;
#[cfg(feature = "gamma")]
pub mod gamma;
#[cfg(feature = "logarithm")]
pub mod logarithm;
#[cfg(feature = "op_assign")]
pub mod op_assign;
#[cfg(feature = "ops")]
pub mod ops;
#[cfg(feature = "root")]
pub mod root;
#[cfg(feature = "rounding")]
pub mod rounding;
#[cfg(feature = "stat_error")]
pub mod stat_error;
#[cfg(feature = "trig")]
pub mod trig;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub mod catalog;

#[cfg(all(feature = "arithmetic", feature = "source"))]
pub use self::arithmetic::*;
#[cfg(all(feature = "bessel", feature = "source"))]
pub use self::bessel::*;
#[cfg(all(feature = "gamma", feature = "source"))]
pub use self::gamma::*;
#[cfg(all(feature = "logarithm", feature = "source"))]
pub use self::logarithm::*;
#[cfg(all(feature = "op_assign", feature = "source"))]
pub use self::op_assign::*;
#[cfg(all(feature = "ops", feature = "source"))]
pub use self::ops::*;
#[cfg(all(feature = "ops", feature = "runtime", not(feature = "source")))]
pub(crate) use self::ops::*;
#[cfg(all(feature = "root", feature = "source"))]
pub use self::root::*;
#[cfg(all(feature = "rounding", feature = "source"))]
pub use self::rounding::*;
#[cfg(all(feature = "stat_error", feature = "source"))]
pub use self::stat_error::*;
#[cfg(all(feature = "trig", feature = "source"))]
pub use self::trig::*;

#[cfg(all(feature = "runtime", not(feature = "dynamic-module")))]
pub use self::catalog::*;

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    pub use crate::catalog::__mech_native::*;
    #[cfg(feature = "add")]
    pub use crate::ops::add::__mech_native::*;
}

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_math_fxns {
    ($lib:ident) => {
        impl_fxns!($lib, T, T, impl_binop);
    };
}

#[cfg(feature = "source")]
#[macro_export]
macro_rules! impl_urnop_match_arms2 {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(feature = $value_string)]
            LegacyValue::$lhs_type(arg) => Ok(Box::new([<$lib $lhs_type:camel S>]{arg: arg.clone(), out: Ref::new($default) })),
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            LegacyValue::$matrix_kind(Matrix::Matrix1(arg)) => Ok(Box::new([<$lib $lhs_type:camel M1>]{arg, out: Ref::new(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            LegacyValue::$matrix_kind(Matrix::Matrix2(arg)) => Ok(Box::new([<$lib $lhs_type:camel M2>]{arg, out: Ref::new(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            LegacyValue::$matrix_kind(Matrix::Matrix3(arg)) => Ok(Box::new([<$lib $lhs_type:camel M3>]{arg, out: Ref::new(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            LegacyValue::$matrix_kind(Matrix::Matrix4(arg)) => Ok(Box::new([<$lib $lhs_type:camel M4>]{arg, out: Ref::new(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            LegacyValue::$matrix_kind(Matrix::Matrix2x3(arg)) => Ok(Box::new([<$lib $lhs_type:camel M2x3>]{arg, out: Ref::new(Matrix2x3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            LegacyValue::$matrix_kind(Matrix::Matrix3x2(arg)) => Ok(Box::new([<$lib $lhs_type:camel M3x2>]{arg, out: Ref::new(Matrix3x2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            LegacyValue::$matrix_kind(Matrix::RowVector2(arg)) => Ok(Box::new([<$lib $lhs_type:camel R2>]{arg: arg.clone(), out: Ref::new(RowVector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            LegacyValue::$matrix_kind(Matrix::RowVector3(arg)) => Ok(Box::new([<$lib $lhs_type:camel R3>]{arg: arg.clone(), out: Ref::new(RowVector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            LegacyValue::$matrix_kind(Matrix::RowVector4(arg)) => Ok(Box::new([<$lib $lhs_type:camel R4>]{arg: arg.clone(), out: Ref::new(RowVector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            LegacyValue::$matrix_kind(Matrix::RowDVector(arg)) => Ok(Box::new([<$lib $lhs_type:camel RD>]{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            LegacyValue::$matrix_kind(Matrix::Vector2(arg)) => Ok(Box::new([<$lib $lhs_type:camel V2>]{arg: arg.clone(), out: Ref::new(Vector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            LegacyValue::$matrix_kind(Matrix::Vector3(arg)) => Ok(Box::new([<$lib $lhs_type:camel V3>]{arg: arg.clone(), out: Ref::new(Vector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            LegacyValue::$matrix_kind(Matrix::Vector4(arg)) => Ok(Box::new([<$lib $lhs_type:camel V4>]{arg: arg.clone(), out: Ref::new(Vector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            LegacyValue::$matrix_kind(Matrix::DVector(arg)) => Ok(Box::new([<$lib $lhs_type:camel VD>]{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            LegacyValue::$matrix_kind(Matrix::DMatrix(arg)) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib $lhs_type:camel MD>]{arg, out: Ref::new(DMatrix::from_element(rows,cols,$default))}))},
          )+
        )+
        x => Err(MechError::new(
          UnhandledFunctionArgumentKind1{arg: x.kind(), fxn_name: stringify!($lib).to_string()},
          None
        ).with_compiler_loc()),
      }}}}

#[macro_export]
macro_rules! impl_math_unop {
  ($fxn_name:ident, $type:ident, $op_fxn:ident) => {
    paste!{
      impl_unop!([<$fxn_name $type:camel S>], $type, $type, [<$op_fxn _op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix1")]
      impl_unop!([<$fxn_name $type:camel M1>], Matrix1<$type>, Matrix1<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix2")]
      impl_unop!([<$fxn_name $type:camel M2>], Matrix2<$type>, Matrix2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix3")]
      impl_unop!([<$fxn_name $type:camel M3>], Matrix3<$type>, Matrix3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix4")]
      impl_unop!([<$fxn_name $type:camel M4>], Matrix4<$type>, Matrix4<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix2x3")]
      impl_unop!([<$fxn_name $type:camel M2x3>], Matrix2x3<$type>, Matrix2x3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrix3x2")]
      impl_unop!([<$fxn_name $type:camel M3x2>], Matrix3x2<$type>, Matrix3x2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "matrixd")]
      impl_unop!([<$fxn_name $type:camel MD>], DMatrix<$type>, DMatrix<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vector2")]
      impl_unop!([<$fxn_name $type:camel R2>], RowVector2<$type>, RowVector2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vector3")]
      impl_unop!([<$fxn_name $type:camel R3>], RowVector3<$type>, RowVector3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vector4")]
      impl_unop!([<$fxn_name $type:camel R4>], RowVector4<$type>, RowVector4<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "row_vectord")]
      impl_unop!([<$fxn_name $type:camel RD>], RowDVector<$type>, RowDVector<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vector2")]
      impl_unop!([<$fxn_name $type:camel V2>], Vector2<$type>, Vector2<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vector3")]
      impl_unop!([<$fxn_name $type:camel V3>], Vector3<$type>, Vector3<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vector4")]
      impl_unop!([<$fxn_name $type:camel V4>], Vector4<$type>, Vector4<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
      #[cfg(feature = "vectord")]
      impl_unop!([<$fxn_name $type:camel VD>], DVector<$type>, DVector<$type>, [<$op_fxn _vec_op>], crate::ops::unary_full_write_contract);
    }}}

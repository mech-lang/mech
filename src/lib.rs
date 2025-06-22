#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate libm;
extern crate nalgebra as na;
extern crate paste;
extern crate simba;

use paste::paste;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
use num_traits::*;
use std::fmt::Debug;
use simba::scalar::ClosedNeg;
use num_traits::Pow;
use mech_core::matrix::Matrix;

static PI: f64 = 3.14159265358979323846264338327950288;

pub mod acos;
pub mod acosh;
pub mod acot;
pub mod acsc;
pub mod asec;
pub mod asin;
pub mod asinh;
pub mod atan;
pub mod atan2;
pub mod cos;
pub mod cosh;
pub mod cot;
pub mod csc;
pub mod sec;
pub mod sin;
pub mod sinh;
pub mod tan;
pub mod tanh;

pub use self::acos::*;
pub use self::acosh::*;
pub use self::acot::*;
pub use self::acsc::*;
pub use self::asec::*;
pub use self::asin::*;
pub use self::asinh::*;
pub use self::atan::*;
pub use self::atan2::*;
pub use self::cos::*;
pub use self::cosh::*;
pub use self::cot::*;
pub use self::csc::*;
pub use self::sec::*;
pub use self::sin::*;
pub use self::sinh::*;
pub use self::tan::*;
pub use self::tanh::*;

// ----------------------------------------------------------------------------
// Math Library
// ----------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_math_fxns {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_binop);
  }}

#[macro_export]
macro_rules! impl_urnop_match_arms2 {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib $lhs_type S>]{arg: arg.clone(), out: new_ref($default) })),
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(arg))) => Ok(Box::new([<$lib $lhs_type M1>]{arg, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(arg))) => Ok(Box::new([<$lib $lhs_type M2>]{arg, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(arg))) => Ok(Box::new([<$lib $lhs_type M3>]{arg, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(arg))) => Ok(Box::new([<$lib $lhs_type M4>]{arg, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(arg))) => Ok(Box::new([<$lib $lhs_type M2x3>]{arg, out: new_ref(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(arg))) => Ok(Box::new([<$lib $lhs_type M3x2>]{arg, out: new_ref(Matrix3x2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(arg))) => Ok(Box::new([<$lib $lhs_type R2>]{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(arg))) => Ok(Box::new([<$lib $lhs_type R3>]{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(arg))) => Ok(Box::new([<$lib $lhs_type R4>]{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$matrix_kind(Matrix::RowDVector(arg))) => Ok(Box::new([<$lib $lhs_type RD>]{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(arg))) => Ok(Box::new([<$lib $lhs_type V2>]{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(arg))) => Ok(Box::new([<$lib $lhs_type V3>]{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(arg))) => Ok(Box::new([<$lib $lhs_type V4>]{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$matrix_kind(Matrix::DVector(arg))) => Ok(Box::new([<$lib $lhs_type VD>]{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib $lhs_type MD>]{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}", x).to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }}}}

#[macro_export]
macro_rules! impl_math_urop {
  ($fxn_name:ident, $type:ident, $op_fxn:ident) => {
    paste!{
      impl_urop!([<$fxn_name $type S>], $type, $type, [<$op_fxn _op>]);
      #[cfg(feature = "Matrix1")]
      impl_urop!([<$fxn_name $type M1>], Matrix1<$type>, Matrix1<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Matrix2")]
      impl_urop!([<$fxn_name $type M2>], Matrix2<$type>, Matrix2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Matrix3")]
      impl_urop!([<$fxn_name $type M3>], Matrix3<$type>, Matrix3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Matrix4")]
      impl_urop!([<$fxn_name $type M4>], Matrix4<$type>, Matrix4<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Matrix2x3")]
      impl_urop!([<$fxn_name $type M2x3>], Matrix2x3<$type>, Matrix2x3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Matrix3x2")]
      impl_urop!([<$fxn_name $type M3x2>], Matrix3x2<$type>, Matrix3x2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "MatrixD")]
      impl_urop!([<$fxn_name $type MD>], DMatrix<$type>, DMatrix<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "RowVector2")]
      impl_urop!([<$fxn_name $type R2>], RowVector2<$type>, RowVector2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "RowVector3")]
      impl_urop!([<$fxn_name $type R3>], RowVector3<$type>, RowVector3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "RowVector4")]
      impl_urop!([<$fxn_name $type R4>], RowVector4<$type>, RowVector4<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "RowVectorD")]
      impl_urop!([<$fxn_name $type RD>], RowDVector<$type>, RowDVector<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Vector2")]
      impl_urop!([<$fxn_name $type V2>], Vector2<$type>, Vector2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Vector3")]
      impl_urop!([<$fxn_name $type V3>], Vector3<$type>, Vector3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "Vector4")]
      impl_urop!([<$fxn_name $type V4>], Vector4<$type>, Vector4<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "VectorD")]
      impl_urop!([<$fxn_name $type VD>], DVector<$type>, DVector<$type>, [<$op_fxn _vec_op>]);
    }}}
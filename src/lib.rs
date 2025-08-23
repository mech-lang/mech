#![no_main]
#![allow(warnings)]

#[cfg(feature = "matrix")]
extern crate nalgebra as na;

use mech_core::*;

use paste::paste;

#[cfg(feature = "vector2")]
use na::Vector2;
#[cfg(feature = "vector3")]
use na::Vector3;
#[cfg(feature = "vector4")]
use na::Vector4;
#[cfg(feature = "vectord")]
use na::DVector;
#[cfg(feature = "matrix1")]
use na::Matrix1;
#[cfg(feature = "matrix2")]
use na::Matrix2;
#[cfg(feature = "matrix3")]
use na::Matrix3;
#[cfg(feature = "matrix4")]
use na::Matrix4;
#[cfg(feature = "matrix2x3")]
use na::Matrix2x3;
#[cfg(feature = "matrix3x2")]
use na::Matrix3x2;
#[cfg(feature = "matrixd")]
use na::DMatrix;
#[cfg(feature = "row_vector2")]
use na::RowVector2;
#[cfg(feature = "row_vector3")]
use na::RowVector3;
#[cfg(feature = "row_vector4")]
use na::RowVector4;
#[cfg(feature = "row_vectord")]
use na::RowDVector;

use std::ops::*;
use std::fmt::{Display, Debug};
use std::marker::PhantomData;

//#[cfg(feature = "trig")]
//pub mod trig;
#[cfg(feature = "ops")]
pub mod ops;
//#[cfg(feature = "op_assign")]
//pub mod op_assign;

//#[cfg(feature = "trig")]
//pub use self::trig::*;
#[cfg(feature = "ops")]
pub use self::ops::*;
//#[cfg(feature = "op_assign")]
//pub use self::op_assign::*;

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
            #[cfg(feature = $value_string)]
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib $lhs_type S>]{arg: arg.clone(), out: Ref::new($default) })),
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(arg))) => Ok(Box::new([<$lib $lhs_type M1>]{arg, out: Ref::new(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(arg))) => Ok(Box::new([<$lib $lhs_type M2>]{arg, out: Ref::new(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(arg))) => Ok(Box::new([<$lib $lhs_type M3>]{arg, out: Ref::new(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(arg))) => Ok(Box::new([<$lib $lhs_type M4>]{arg, out: Ref::new(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(arg))) => Ok(Box::new([<$lib $lhs_type M2x3>]{arg, out: Ref::new(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(arg))) => Ok(Box::new([<$lib $lhs_type M3x2>]{arg, out: Ref::new(Matrix3x2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(arg))) => Ok(Box::new([<$lib $lhs_type R2>]{arg: arg.clone(), out: Ref::new(RowVector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(arg))) => Ok(Box::new([<$lib $lhs_type R3>]{arg: arg.clone(), out: Ref::new(RowVector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(arg))) => Ok(Box::new([<$lib $lhs_type R4>]{arg: arg.clone(), out: Ref::new(RowVector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$matrix_kind(Matrix::RowDVector(arg))) => Ok(Box::new([<$lib $lhs_type RD>]{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(arg))) => Ok(Box::new([<$lib $lhs_type V2>]{arg: arg.clone(), out: Ref::new(Vector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(arg))) => Ok(Box::new([<$lib $lhs_type V3>]{arg: arg.clone(), out: Ref::new(Vector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(arg))) => Ok(Box::new([<$lib $lhs_type V4>]{arg: arg.clone(), out: Ref::new(Vector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$matrix_kind(Matrix::DVector(arg))) => Ok(Box::new([<$lib $lhs_type VD>]{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$matrix_kind(Matrix::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib $lhs_type MD>]{arg, out: Ref::new(DMatrix::from_element(rows,cols,$default))}))},
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}", x).to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }}}}

#[macro_export]
macro_rules! impl_math_urop {
  ($fxn_name:ident, $type:ident, $op_fxn:ident) => {
    paste!{
      impl_urop!([<$fxn_name $type S>], $type, $type, [<$op_fxn _op>]);
      #[cfg(feature = "matrix1")]
      impl_urop!([<$fxn_name $type M1>], Matrix1<$type>, Matrix1<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "matrix2")]
      impl_urop!([<$fxn_name $type M2>], Matrix2<$type>, Matrix2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "matrix3")]
      impl_urop!([<$fxn_name $type M3>], Matrix3<$type>, Matrix3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "matrix4")]
      impl_urop!([<$fxn_name $type M4>], Matrix4<$type>, Matrix4<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "matrix2x3")]
      impl_urop!([<$fxn_name $type M2x3>], Matrix2x3<$type>, Matrix2x3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "matrix3x2")]
      impl_urop!([<$fxn_name $type M3x2>], Matrix3x2<$type>, Matrix3x2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "matrixd")]
      impl_urop!([<$fxn_name $type MD>], DMatrix<$type>, DMatrix<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "row_vector2")]
      impl_urop!([<$fxn_name $type R2>], RowVector2<$type>, RowVector2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "row_vector3")]
      impl_urop!([<$fxn_name $type R3>], RowVector3<$type>, RowVector3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "row_vector4")]
      impl_urop!([<$fxn_name $type R4>], RowVector4<$type>, RowVector4<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "row_vectord")]
      impl_urop!([<$fxn_name $type RD>], RowDVector<$type>, RowDVector<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "vector2")]
      impl_urop!([<$fxn_name $type V2>], Vector2<$type>, Vector2<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "vector3")]
      impl_urop!([<$fxn_name $type V3>], Vector3<$type>, Vector3<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "vector4")]
      impl_urop!([<$fxn_name $type V4>], Vector4<$type>, Vector4<$type>, [<$op_fxn _vec_op>]);
      #[cfg(feature = "vectord")]
      impl_urop!([<$fxn_name $type VD>], DVector<$type>, DVector<$type>, [<$op_fxn _vec_op>]);
    }}}
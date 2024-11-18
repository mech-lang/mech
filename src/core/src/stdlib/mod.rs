use crate::*;
use crate::matrix::Matrix;

use paste::paste;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
use num_traits::*;
use std::fmt::Debug;
use simba::scalar::ClosedNeg;
use num_traits::Pow;

pub mod math;
pub mod logic;
pub mod compare;
pub mod matrix;
pub mod range;
pub mod table;
pub mod convert;

// ============================================================================
// The Standard Library!
// ============================================================================

#[macro_export]
macro_rules! impl_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
      #[derive(Debug)]
      struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
      }
      impl<T> MechFunction for $struct_name<T>
      where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + SubAssign +
      Mul<Output = T> + MulAssign +
      Div<Output = T> + DivAssign +
      Zero + One,
      Ref<$out_type>: ToValue
      {
      fn solve(&self) {
          let lhs_ptr = self.lhs.as_ptr();
          let rhs_ptr = self.rhs.as_ptr();
          let out_ptr = self.out.as_ptr();
          $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
      }};}

#[macro_export]
macro_rules! impl_bool_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

#[macro_export]  
macro_rules! impl_bool_urop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}

#[macro_export]  
macro_rules! impl_urop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }};}  

#[macro_export]
macro_rules! impl_fxns {
  ($lib:ident, $in:ident, $out:ident, $op:ident) => {
    paste!{
      // Scalar
      $op!([<$lib SS>], $in, $in, $out, [<$lib:lower _op>]);
      // Scalar Matrix
      $op!([<$lib SM1>], $in, Matrix1<$in>, Matrix1<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SM2>], $in, Matrix2<$in>, Matrix2<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SM3>], $in, Matrix3<$in>, Matrix3<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SM4>], $in, Matrix4<$in>, Matrix4<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SMD>], $in, DMatrix<$in>, DMatrix<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SM2x3>], $in, Matrix2x3<$in>, Matrix2x3<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SM3x2>], $in, Matrix3x2<$in>, Matrix3x2<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Row
      $op!([<$lib SR2>], $in, RowVector2<$in>, RowVector2<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SR3>], $in, RowVector3<$in>, RowVector3<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SR4>], $in, RowVector4<$in>, RowVector4<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SRD>], $in, RowDVector<$in>, RowDVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Vector
      $op!([<$lib SV2>], $in, Vector2<$in>, Vector2<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SV3>], $in, Vector3<$in>, Vector3<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SV4>], $in, Vector4<$in>, Vector4<$out>,[<$lib:lower _scalar_rhs_op>]);
      $op!([<$lib SVD>], $in, DVector<$in>, DVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Matrix Scalar
      $op!([<$lib M1S>], Matrix1<$in>, $in, Matrix1<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib M2S>], Matrix2<$in>, $in, Matrix2<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib M3S>], Matrix3<$in>, $in, Matrix3<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib M4S>], Matrix4<$in>, $in, Matrix4<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib MDS>], DMatrix<$in>, $in, DMatrix<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib M2x3S>], Matrix2x3<$in>, $in, Matrix2x3<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib M3x2S>], Matrix3x2<$in>, $in, Matrix3x2<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Row Scalar
      $op!([<$lib R2S>], RowVector2<$in>, $in, RowVector2<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib R3S>], RowVector3<$in>, $in, RowVector3<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib R4S>], RowVector4<$in>, $in, RowVector4<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib RDS>], RowDVector<$in>, $in, RowDVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Vector Scalar
      $op!([<$lib V2S>], Vector2<$in>, $in, Vector2<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib V3S>], Vector3<$in>, $in, Vector3<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib V4S>], Vector4<$in>, $in, Vector4<$out>,[<$lib:lower _scalar_lhs_op>]);
      $op!([<$lib VDS>], DVector<$in>, $in, DVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Matrix Matrix
      $op!([<$lib M2x3M2x3>], Matrix2x3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib M3x2M3x2>], Matrix3x2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib M1M1>], Matrix1<$in>, Matrix1<$in>, Matrix1<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib M2M2>], Matrix2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib M3M3>], Matrix3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib M4M4>], Matrix4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib MDMD>], DMatrix<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_op>]);
      // Row Row
      $op!([<$lib R2R2>], RowVector2<$in>, RowVector2<$in>, RowVector2<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib R3R3>], RowVector3<$in>, RowVector3<$in>, RowVector3<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib R4R4>], RowVector4<$in>, RowVector4<$in>, RowVector4<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib RDRD>], RowDVector<$in>, RowDVector<$in>, RowDVector<$out>, [<$lib:lower _vec_op>]);
      // Vector Vector
      $op!([<$lib V2V2>], Vector2<$in>, Vector2<$in>, Vector2<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib V3V3>], Vector3<$in>, Vector3<$in>, Vector3<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib V4V4>], Vector4<$in>, Vector4<$in>, Vector4<$out>, [<$lib:lower _vec_op>]);
      $op!([<$lib VDVD>], DVector<$in>, DVector<$in>, DVector<$out>, [<$lib:lower _vec_op>]);
    }
  }}

#[macro_export]
macro_rules! impl_binop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Scalar Scalar
            (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => Ok(Box::new([<$lib SS>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($default) })),
            // Scalar Matrix
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib SM1>]{lhs, rhs, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib SM2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib SM3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib SM4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib SM2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib SM3x2>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},   
            // Scalar Row
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib SR2>]{lhs, rhs, out: new_ref(RowVector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib SR3>]{lhs, rhs, out: new_ref(RowVector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib SR4>]{lhs, rhs, out: new_ref(RowVector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib SRD>]{lhs, rhs: rhs.clone(), out: new_ref(RowDVector::from_element(rhs.borrow().len(),$default))})),
            // Scalar Vector
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new([<$lib SV2>]{lhs, rhs, out: new_ref(Vector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new([<$lib SV3>]{lhs, rhs, out: new_ref(Vector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new([<$lib SV4>]{lhs, rhs, out: new_ref(Vector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new([<$lib SVD>]{lhs, rhs: rhs.clone(), out: new_ref(DVector::from_element(rhs.borrow().len(),$default))})),
            // Matrix Scalar
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M1S>]{lhs, rhs, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2S>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3S>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M4S>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2x3S>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3x2S>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(lhs)),Value::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDS>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},              
            // Row Scalar
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R2S>]{lhs, rhs, out: new_ref(RowVector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R3S>]{lhs, rhs, out: new_ref(RowVector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R4S>]{lhs, rhs, out: new_ref(RowVector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$matrix_kind(Matrix::RowDVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib RDS>]{lhs: lhs.clone(), rhs, out: new_ref(RowDVector::from_element(lhs.borrow().len(),$default))})),
            // Vector Scalar
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V2S>]{lhs, rhs, out: new_ref(Vector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V3S>]{lhs, rhs, out: new_ref(Vector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V4S>]{lhs, rhs, out: new_ref(Vector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$matrix_kind(Matrix::DVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib VDS>]{lhs: lhs.clone(), rhs, out: new_ref(DVector::from_element(lhs.borrow().len(),$default))})),
            // Matrix Matrix
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(lhs)), Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib M1M1>]{lhs, rhs, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(lhs)), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib M4M4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),  
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib M3x2M3x2>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
            // Row Row
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib RDRD>]{lhs: lhs.clone(), rhs, out: new_ref(RowDVector::from_element(lhs.borrow().len(),$default))})),
            // Vector Vector
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new([<$lib V2V2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Vector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new([<$lib V3V3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Vector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(lhs)), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new([<$lib V4V4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Vector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$matrix_kind(Matrix::DVector(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new([<$lib VDVD>]{lhs: lhs.clone(), rhs, out: new_ref(DVector::from_element(lhs.borrow().len(),$default))})),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_urnop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib S>]{arg: arg.clone(), out: new_ref($default) })),
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(arg))) => Ok(Box::new([<$lib M1>]{arg, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(arg))) => Ok(Box::new([<$lib M2>]{arg, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(arg))) => Ok(Box::new([<$lib M3>]{arg, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(arg))) => Ok(Box::new([<$lib M4>]{arg, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(arg))) => Ok(Box::new([<$lib M2x3>]{arg, out: new_ref(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(arg))) => Ok(Box::new([<$lib M3x2>]{arg, out: new_ref(Matrix3x2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(arg))) => Ok(Box::new([<$lib R2>]{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(arg))) => Ok(Box::new([<$lib R3>]{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(arg))) => Ok(Box::new([<$lib R4>]{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$matrix_kind(Matrix::RowDVector(arg))) => Ok(Box::new([<$lib RD>]{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(arg))) => Ok(Box::new([<$lib V2>]{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(arg))) => Ok(Box::new([<$lib V3>]{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(arg))) => Ok(Box::new([<$lib V4>]{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$matrix_kind(Matrix::DVector(arg))) => Ok(Box::new([<$lib VD>]{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib MD>]{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_binop_fxn {
  ($fxn_name:ident, $gen_fxn:ident) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
          return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
        }
        let lhs_value = arguments[0].clone();
        let rhs_value = arguments[1].clone();
        match $gen_fxn(lhs_value.clone(), rhs_value.clone()) {
          Ok(fxn) => Ok(fxn),
          Err(_) => {
            match (lhs_value,rhs_value) {
              (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {$gen_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
              (lhs_value,Value::MutableReference(rhs)) => { $gen_fxn(lhs_value.clone(), rhs.borrow().clone())}
              (Value::MutableReference(lhs),rhs_value) => { $gen_fxn(lhs.borrow().clone(), rhs_value.clone()) }
              x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
        }
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_urnop_fxn {
  ($fxn_name:ident, $gen_fxn:ident) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
          return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
        }
        let input = arguments[0].clone();
        match $gen_fxn(input.clone()) {
          Ok(fxn) => Ok(fxn),
          Err(_) => {
            match (input) {
              (Value::MutableReference(input)) => {$gen_fxn(input.borrow().clone())}
              x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
        }
      }
    }
  }
}

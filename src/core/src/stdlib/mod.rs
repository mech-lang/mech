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
      $op!([<$lib SMD>], $in, DMatrix<$in>, DMatrix<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Row
      $op!([<$lib SRD>], $in, RowDVector<$in>, RowDVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Vector
      $op!([<$lib SVD>], $in, DVector<$in>, DVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Matrix Scalar
      $op!([<$lib MDS>], DMatrix<$in>, $in, DMatrix<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Row Scalar
      $op!([<$lib RDS>], RowDVector<$in>, $in, RowDVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Vector Scalar
      $op!([<$lib VDS>], DVector<$in>, $in, DVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Matrix Matrix
      $op!([<$lib MDMD>], DMatrix<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_op>]);
      // Row Row
      $op!([<$lib RDRD>], RowDVector<$in>, RowDVector<$in>, RowDVector<$out>, [<$lib:lower _vec_op>]);
      // Vector Vector
      $op!([<$lib VDVD>], DVector<$in>, DVector<$in>, DVector<$out>, [<$lib:lower _vec_op>]);
    }
  }}

#[macro_export]
macro_rules! impl_binop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Scalar Scalar
            (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => Ok(Box::new([<$lib SS>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($default) })),
            // Scalar Matrix
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))
            },   
            // Scalar Row
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => Ok(Box::new([<$lib SRD>]{lhs, rhs: rhs.clone(), out: new_ref(RowDVector::from_element(rhs.borrow().len(),$default))})),
            // Scalar Vector
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => Ok(Box::new([<$lib SVD>]{lhs, rhs: rhs.clone(), out: new_ref(DVector::from_element(rhs.borrow().len(),$default))})),
            // Matrix Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)),Value::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDS>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))
            },              
            // Row Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib RDS>]{lhs: lhs.clone(), rhs, out: new_ref(RowDVector::from_element(lhs.borrow().len(),$default))})),
            // Vector Scalar
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib VDS>]{lhs: lhs.clone(), rhs, out: new_ref(DVector::from_element(lhs.borrow().len(),$default))})),
            // Matrix Matrix
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))
            },
            // Row Row
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => Ok(Box::new([<$lib RDRD>]{lhs: lhs.clone(), rhs, out: new_ref(RowDVector::from_element(lhs.borrow().len(),$default))})),
            // Vector Vector
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => Ok(Box::new([<$lib VDVD>]{lhs: lhs.clone(), rhs, out: new_ref(DVector::from_element(lhs.borrow().len(),$default))})),
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_urnop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib S>]{arg: arg.clone(), out: new_ref($default) })),
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg))) => Ok(Box::new([<$lib RD>]{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))) => Ok(Box::new([<$lib VD>]{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib MD>]{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))
            },
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

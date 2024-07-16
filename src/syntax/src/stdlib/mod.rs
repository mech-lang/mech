use crate::*;
use crate::matrix::Matrix;

use paste::paste;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};
use std::ops::*;
use num_traits::*;
use std::fmt::Debug;
use simba::scalar::ClosedNeg;


pub mod math;
pub mod logic;
pub mod compare;
pub mod matrix;
pub mod range;

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
macro_rules! generate_binop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => {
              Ok(Box::new([<$lib Scalar>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($default) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2(rhs))) => {
              Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib SM2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::Matrix2(rhs))) => {
              Ok(Box::new([<$lib SM2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::RowVector2(rhs))) => {
              Ok(Box::new([<$lib SR2>]{lhs, rhs, out: new_ref(RowVector2::from_element($default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::RowVector3(rhs))) => {
              Ok(Box::new([<$lib SR3>]{lhs, rhs, out: new_ref(RowVector3::from_element($default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::RowVector4(rhs))) => {
              Ok(Box::new([<$lib SR4>]{lhs, rhs, out: new_ref(RowVector4::from_element($default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => {
              let length = {rhs.borrow().len()};
              Ok(Box::new([<$lib SRD>]{lhs, rhs, out: new_ref(RowDVector::from_element(length,$default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => {
              let length = {rhs.borrow().len()};
              Ok(Box::new([<$lib SVD>]{lhs, rhs, out: new_ref(DVector::from_element(length,$default))}))},
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)),Value::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M2x3S>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(lhs)),Value::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib M2S>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(lhs)),Value::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib R2S>]{lhs, rhs, out: new_ref(RowVector2::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(lhs)),Value::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib R3S>]{lhs, rhs, out: new_ref(RowVector3::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(lhs)),Value::$lhs_type(rhs)) => {
              Ok(Box::new([<$lib R4S>]{lhs, rhs, out: new_ref(RowVector4::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)),Value::$lhs_type(rhs)) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib RDS>]{lhs, rhs, out: new_ref(RowDVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)),Value::$lhs_type(rhs)) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib VDS>]{lhs, rhs, out: new_ref(DVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)),Value::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDS>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix3(rhs))) => {
              Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector2(rhs))) => {
              Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector2::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector3(rhs))) => {
              Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector3::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowVector4(rhs))) => {
              Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector4::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(rhs))) => {
              Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))}))},          
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::RowDVector(rhs))) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib RDRD>]{lhs, rhs, out: new_ref(RowDVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DVector(rhs))) => {
              let length = {lhs.borrow().len()};
              Ok(Box::new([<$lib VDVD>]{lhs, rhs, out: new_ref(DVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(lhs)), Value::$matrix_kind(Matrix::<$target_type>::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

// ----------------------------------------------------------------------------
// Type Conversion Library
// ----------------------------------------------------------------------------

// Convert --------------------------------------------------------------------

#[derive(Debug)]
struct ConvertScalar<T, U> {
  input: Ref<T>,
  out: Ref<U>,
}

impl<T, U> MechFunction for ConvertScalar<T, U>
where
  T: Copy + std::fmt::Debug,
  U: Copy + std::fmt::Debug,
  Ref<U>: ToValue
{
  fn solve(&self) {
    let in_value = self.input.borrow();
    let mut out_value = self.out.borrow_mut();
    unsafe {
      *out_value = *(&*in_value as *const T as *const U);
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

macro_rules! generate_conversion_match_arms {
  ($arg:expr, $($input_type:ident => $($value_kind:ident, $target_type:ident),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          (Value::$input_type(arg), ValueKind::$value_kind) => {Ok(Box::new(ConvertScalar {input: arg.clone(),out: new_ref(0 as $target_type)}))},
        )+
      )+
      x => Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
    }
  }
}

fn generate_conversion_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  generate_conversion_match_arms!(
    (source_value, target_kind),
    I8 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I16 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I32 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I64 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    I128 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U8 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U16 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U32 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U64 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
    U128 => I8, i8, I16, i16, I32, i32, I64, i64, I128, i128, U8, u8, U16, u16, U32, u32, U64, u64, U128, u128;
  )
}

pub struct ConvertKind {}

impl NativeFunctionCompiler for ConvertKind {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let target_kind = arguments[1].kind();
    match generate_conversion_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(lhs) => {
            generate_conversion_fxn(lhs.borrow().clone(), target_kind.clone())
          }
          _ => unreachable!(),
        }
      }
    }
  }
}
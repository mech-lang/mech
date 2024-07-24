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

#[macro_export]
macro_rules! generate_urnop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(arg)) => {
              Ok(Box::new([<$lib Scalar>]{arg: arg.clone(), out: new_ref($default) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg))) => {
              Ok(Box::new([<$lib M2>]{arg, out: new_ref(Matrix2::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg))) => {
              Ok(Box::new([<$lib M3>]{arg, out: new_ref(Matrix3::from_element($default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg))) => {
              Ok(Box::new([<$lib R2>]{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg))) => {
              Ok(Box::new([<$lib R3>]{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::RowVector4(arg))) => {
              Ok(Box::new([<$lib R4>]{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) }))},
            (Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg))) => {
              Ok(Box::new([<$lib M2x3>]{arg, out: new_ref(Matrix2x3::from_element($default))}))},          
            (Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg))) => {
              let length = {arg.borrow().len()};
              Ok(Box::new([<$lib RD>]{arg, out: new_ref(RowDVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))) => {
              let length = {arg.borrow().len()};
              Ok(Box::new([<$lib VD>]{arg, out: new_ref(DVector::from_element(length,$default))}))},
            (Value::$matrix_kind(Matrix::<$target_type>::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib MD>]{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
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

#[macro_export]  
macro_rules! impl_convert_op {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $out_type2:ty, $op:ident) => {
    #[derive(Debug)]
    
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name
    where
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr,$out_type2)
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  }
}

macro_rules! convert_op1 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ *$out = *$arg as $out_type }
  };}

macro_rules! convert_op2 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ *$out = (*$arg).0 as $out_type }
  };}

macro_rules! convert_op3 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ (*$out).0 = (*$arg) as $out_type }
  };}

macro_rules! convert_op4 {
  ($arg:expr, $out:expr, $out_type:ty) => {
    unsafe{ (*$out).0 = (*$arg).0 as $out_type }
  };}

macro_rules! impl_convert_op_group {
  ($from:ty, [$($to:ty),*], $func:ident) => {
    paste!{
      $(
        impl_convert_op!([<ConvertScalar $from:upper $to:upper>], $from, $to, [<$to:lower>], $func);
      )*
    }
  };
}

impl_convert_op_group!(i8,   [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i16,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i32,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i64,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(i128, [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);

impl_convert_op_group!(u8,   [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u16,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u32,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u64,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);
impl_convert_op_group!(u128, [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op1);

impl_convert_op_group!(F32,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op2);
impl_convert_op_group!(F64,  [i8, i16, i32, i64, i128, u8, u16, u32, u64, u128], convert_op2);

impl_convert_op_group!(i8,   [F32, F64], convert_op3);
impl_convert_op_group!(i16,  [F32, F64], convert_op3);
impl_convert_op_group!(i32,  [F32, F64], convert_op3);
impl_convert_op_group!(i64,  [F32, F64], convert_op3);
impl_convert_op_group!(i128, [F32, F64], convert_op3);
impl_convert_op_group!(u8,   [F32, F64], convert_op3);
impl_convert_op_group!(u16,  [F32, F64], convert_op3);
impl_convert_op_group!(u32,  [F32, F64], convert_op3);
impl_convert_op_group!(u64,  [F32, F64], convert_op3);
impl_convert_op_group!(u128, [F32, F64], convert_op3);

impl_convert_op_group!(F32,  [F32, F64], convert_op4);
impl_convert_op_group!(F64,  [F32, F64], convert_op4);

macro_rules! generate_conversion_match_arms {
  ($arg:expr, $($input_type:ident => $($target_type:ident),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::[<$input_type:upper>](arg), ValueKind::[<$target_type:upper>]) => {Ok(Box::new([<ConvertScalar $input_type:upper $target_type:upper>]{arg: arg.clone(), out: new_ref($target_type::zero())}))},
          )+
        )+
        x => Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    }
  }
}

fn generate_conversion_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  generate_conversion_match_arms!(
    (source_value, target_kind),
    i8   => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i16  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    i128 => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u8   => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u16  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    u128 => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    F32  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
    F64  => i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, F32, F64;
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
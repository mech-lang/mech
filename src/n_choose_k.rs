use crate::*;
use mech_core::*;

// Combinatorics N Choose K----------------------------------------------------
/*
macro_rules! sum_column_op {
    ($arg:expr, $out:expr) => {
      unsafe { 
        *$out = (*$arg).column_sum();
      }
    };}

#[cfg(feature = "Matrix1")]
impl_stats_urop!(StatsSumColumnM1, Matrix1<T>, Matrix1<T>, sum_column_op);
#[cfg(feature = "Matrix2")]
impl_stats_urop!(StatsSumColumnM2, Matrix2<T>, Vector2<T>, sum_column_op);
#[cfg(feature = "Matrix3")]
impl_stats_urop!(StatsSumColumnM3, Matrix3<T>, Vector3<T>, sum_column_op);
#[cfg(feature = "Matrix4")]
impl_stats_urop!(StatsSumColumnM4, Matrix4<T>, Vector4<T>, sum_column_op);
#[cfg(feature = "Matrix2x3")]
impl_stats_urop!(StatsSumColumnM2x3, Matrix2x3<T>, Vector2<T>, sum_column_op);
#[cfg(feature = "Matrix3x2")]
impl_stats_urop!(StatsSumColumnM3x2, Matrix3x2<T>, Vector3<T>, sum_column_op);
#[cfg(feature = "MatrixD")]
impl_stats_urop!(StatsSumColumnMD, DMatrix<T>, DVector<T>, sum_column_op);
#[cfg(feature = "Vector2")]
impl_stats_urop!(StatsSumColumnV2, Vector2<T>, Vector2<T>, sum_column_op);
#[cfg(feature = "Vector3")]
impl_stats_urop!(StatsSumColumnV3, Vector3<T>, Vector3<T>, sum_column_op);
#[cfg(feature = "Vector4")]
impl_stats_urop!(StatsSumColumnV4, Vector4<T>, Vector4<T>, sum_column_op); 
#[cfg(feature = "VectorD")]
impl_stats_urop!(StatsSumColumnVD, DVector<T>, DVector<T>, sum_column_op);
#[cfg(feature = "Vector2")]
impl_stats_urop!(StatsSumColumnR2, RowVector2<T>, Matrix1<T>, sum_column_op);
#[cfg(feature = "Vector3")]
impl_stats_urop!(StatsSumColumnR3, RowVector3<T>, Matrix1<T>, sum_column_op);
#[cfg(feature = "Vector4")]
impl_stats_urop!(StatsSumColumnR4, RowVector4<T>, Matrix1<T>, sum_column_op); 
#[cfg(feature = "VectorD")]
impl_stats_urop!(StatsSumColumnRD, RowDVector<T>, Matrix1<T>, sum_column_op);

macro_rules! impl_combinatorics_n_choose_k_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{ 
      match $arg {
        $(
          $(
            #[cfg(feature = "Vector4")]
            Value::[<Matrix $input_type>](m) => {
              // determine the size of the output matrix for n choose k
              let out_size = 
              Ok(Box::new([<CombinatoricsNChooseKMatrix $input_type>]{arg: arg.clone(), out: new_ref(Matrix1::from_element($default)) }))
            },
            #[cfg(feature = "Vector3")]
            Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(StatsSumColumnR3{arg: arg.clone(), out: new_ref(Matrix1::from_element($default)) })),
            #[cfg(feature = "Vector2")]
            Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(StatsSumColumnR2{arg: arg.clone(), out: new_ref(Matrix1::from_element($default)) })),
            #[cfg(feature = "Vector4")]
            Value::$matrix_kind(Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(StatsSumColumnV4{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
            #[cfg(feature = "Vector3")]
            Value::$matrix_kind(Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(StatsSumColumnV3{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
            #[cfg(feature = "Vector2")]
            Value::$matrix_kind(Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(StatsSumColumnV2{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
            #[cfg(feature = "Matrix4")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(StatsSumColumnM4{arg: arg.clone(), out: new_ref(Vector4::from_element($default))})),
            #[cfg(feature = "Matrix3")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(StatsSumColumnM3{arg: arg.clone(), out: new_ref(Vector3::from_element($default))})),
            #[cfg(feature = "Matrix2")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(StatsSumColumnM2{arg: arg.clone(), out: new_ref(Vector2::from_element($default))})),
            #[cfg(feature = "Matrix1")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(StatsSumColumnM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($default))})),
            #[cfg(feature = "Matrix2x3")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(StatsSumColumnM2x3{arg: arg.clone(), out: new_ref(Vector2::from_element($default))})),          
            #[cfg(feature = "Matrix3x2")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(StatsSumColumnM3x2{arg: arg.clone(), out: new_ref(Vector3::from_element($default))})),          
            #[cfg(feature = "VectorD")]
            Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(StatsSumColumnVD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(feature = "VectorD")]
            Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(StatsSumColumnRD{arg: arg.clone(), out: new_ref(Matrix1::from_element($default))})),
            #[cfg(feature = "MatrixD")]
            Value::$matrix_kind(Matrix::<$target_type>::DMatrix(arg)) => Ok(Box::new(StatsSumColumnMD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().nrows(),$default))})),
          )+
        )+
        _ => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}*/

use std::fmt::Debug;
use std::ops::{Add, AddAssign, Sub, Div, Mul};
use num_traits::{Zero, One};

#[derive(Debug)]
pub struct NChooseK<T> {
    n: Ref<T>,
    k: Ref<T>,
    out: Ref<T>,
}

impl<T> MechFunction for NChooseK<T>
where
    T: Copy + Debug + Clone + Sync + Send + 'static +
       Add<Output = T> + AddAssign +
       Sub<Output = T> + Mul<Output = T> + Div<Output = T> +
       Zero + One +
       PartialEq + PartialOrd,
    Ref<T>: ToValue,
{
  fn solve(&self) {
    let n_ptr = self.n.as_ptr();
    let k_ptr = self.k.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let n = *n_ptr;
      let k = *k_ptr;
      if k > n {
        *out_ptr = T::zero(); // undefined for k > n
        return;
      }
      let mut result = T::one();
      let mut i = T::zero();
      while i < k {
        let numerator = n - i;
        let denominator = i + T::one();
        result = result * numerator / denominator;
        i = i + T::one();
      }
      *out_ptr = result;
    }
  }
  fn out(&self) -> Value {self.out.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}

fn impl_combinatorics_n_choose_k_fxn(n: Value, k: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (n,k) {
    (Value::U8(n), Value::U8(k)) => Ok(Box::new(NChooseK::<u8>{n: n, k: k, out: new_ref(u8::zero())})),
    (Value::U16(n), Value::U16(k)) => Ok(Box::new(NChooseK::<u16>{n: n, k: k, out: new_ref(u16::zero())})),
    (Value::U32(n), Value::U32(k)) => Ok(Box::new(NChooseK::<u32>{n: n, k: k, out: new_ref(u32::zero())})),
    (Value::U64(n), Value::U64(k)) => Ok(Box::new(NChooseK::<u64>{n: n, k: k, out: new_ref(u64::zero())})),
    (Value::U128(n), Value::U128(k)) => Ok(Box::new(NChooseK::<u128>{n: n, k: k, out: new_ref(u128::zero())})),
    (Value::I8(n), Value::I8(k)) => Ok(Box::new(NChooseK::<i8>{n: n, k: k, out: new_ref(i8::zero())})),
    (Value::I16(n), Value::I16(k)) => Ok(Box::new(NChooseK::<i16>{n: n, k: k, out: new_ref(i16::zero())})),
    (Value::I32(n), Value::I32(k)) => Ok(Box::new(NChooseK::<i32>{n: n, k: k, out: new_ref(i32::zero())})),
    (Value::I64(n), Value::I64(k)) => Ok(Box::new(NChooseK::<i64>{n: n, k: k, out: new_ref(i64::zero())})),
    (Value::I128(n), Value::I128(k)) => Ok(Box::new(NChooseK::<i128>{n: n, k: k, out: new_ref(i128::zero())})),
    (Value::F32(n), Value::F32(k)) => Ok(Box::new(NChooseK::<F32>{n: n, k: k, out: new_ref(F32::zero())})),
    (Value::F64(n), Value::F64(k)) => Ok(Box::new(NChooseK::<F64>{n: n, k: k, out: new_ref(F64::zero())})),
    _ => todo!(),
  }
}
 
pub struct CombinatoricsNChooseK {}

impl NativeFunctionCompiler for CombinatoricsNChooseK {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let n = arguments[0].clone();
    let k = arguments[1].clone();

    match impl_combinatorics_n_choose_k_fxn(n.clone(),k.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (n,k) {
          (Value::MutableReference(n),Value::MutableReference(k)) => {let n_brrw = n.borrow();let k_brrw = k.borrow();impl_combinatorics_n_choose_k_fxn(n_brrw.clone(),k_brrw.clone())}
          (n,Value::MutableReference(k)) => {let k_brrw = k.borrow(); impl_combinatorics_n_choose_k_fxn(n.clone(),k_brrw.clone())}
          (Value::MutableReference(n),k) => {let n_brrw = n.borrow();impl_combinatorics_n_choose_k_fxn(n_brrw.clone(),k.clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
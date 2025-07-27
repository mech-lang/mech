#[macro_use]
use crate::stdlib::*;

use nalgebra::{Scalar, Matrix3, Matrix4, DVector};
use std::fmt::Debug;
use std::ops::{Index, IndexMut};
use std::marker::PhantomData;

pub struct ConvertMatToMat2<TFrom, TTo, FromMat, ToMat> {
    pub arg: Ref<FromMat>,
    pub out: Ref<ToMat>,
    pub elements: usize,
    _marker: PhantomData<(TFrom, TTo)>, 
}

impl<TFrom, TTo, FromMat, ToMat> MechFunction for ConvertMatToMat2<TFrom, TTo, FromMat, ToMat>
where
    Ref<ToMat>: ToValue,
    TFrom: LosslessInto<TTo> + Debug + Scalar + Copy,
    TTo: Debug + Scalar,
    FromMat: Debug + Index<usize, Output = TFrom>,
    ToMat: Debug + IndexMut<usize, Output = TTo>,
{
  fn solve(&self) {
    let arg_ptr = self.arg.as_ptr();
    let out_ptr = self.out.as_ptr();
    unsafe {
      let out_ref: &mut ToMat = &mut *out_ptr;
      let arg_ref: &FromMat = &*arg_ptr;
      for i in 0..self.elements {
        out_ref[i] = arg_ref[i].lossless_into();
      }
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("ConvertMatToMat2 {{ arg: {:?}, out: {:?}, elements: {} }}", self.arg, self.out, self.elements) }
}

macro_rules! impl_conversion_mat_to_mat_match_arms {
  (
    $arg:expr,
    $(
      $input_type:ident => $(
        $target_type:ident, $zero:expr
      ),+ $(,)?
    );+ $(;)?
  ) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::[<Matrix $input_type>](v), ValueKind::Matrix(box ValueKind::$target_type, dims)) => {
              let shape = v.shape();
              if dims.is_empty() || ((shape[0] == dims[0]) && (shape[1] == dims[1])) {
                match v {
                  #[cfg(feature = "Matrix1")]
                  Matrix::Matrix1(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix1::from_element($zero)), elements: 1, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix2")]
                  Matrix::Matrix2(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix2::from_element($zero)), elements: 4, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix3")]
                  Matrix::Matrix3(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix3::from_element($zero)), elements: 9, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix4")]
                  Matrix::Matrix4(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix4::from_element($zero)), elements: 16, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix3x2")]
                  Matrix::Matrix3x2(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix3x2::from_element($zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix2x3")]
                  Matrix::Matrix2x3(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix2x3::from_element($zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "RowVector2")]
                  Matrix::RowVector2(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowVector2::from_element($zero)), elements: 2, _marker: PhantomData })); },
                  #[cfg(feature = "RowVector3")]
                  Matrix::RowVector3(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowVector3::from_element($zero)), elements: 3, _marker: PhantomData })); },
                  #[cfg(feature = "RowVector4")]
                  Matrix::RowVector4(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowVector4::from_element($zero)), elements: 4, _marker: PhantomData })); },
                  #[cfg(feature = "Vector2")]
                  Matrix::Vector2(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Vector2::from_element($zero)), elements: 2, _marker: PhantomData })); },
                  #[cfg(feature = "Vector3")]
                  Matrix::Vector3(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Vector3::from_element($zero)), elements: 3, _marker: PhantomData })); },
                  #[cfg(feature = "Vector4")]
                  Matrix::Vector4(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Vector4::from_element($zero)), elements: 4, _marker: PhantomData })); },
                  #[cfg(feature = "VectorD")]
                  Matrix::DVector(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DVector::from_element(shape[0], $zero)), elements: shape[0], _marker: PhantomData })); },
                  #[cfg(feature = "RowVectorD")]
                  Matrix::RowDVector(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowDVector::from_element(shape[1], $zero)), elements: shape[1], _marker: PhantomData })); },
                  #[cfg(feature = "MatrixD")]
                  Matrix::DMatrix(v) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DMatrix::from_element(shape[0], shape[1], $zero)), elements: shape[0] * shape[1], _marker: PhantomData })); },
                }
              } else if shape[0] * shape[1] == dims[0] * dims[1] {
                match (v,dims[0],dims[1]) {
                  #[cfg(feature = "Matrix2")]
                  (Matrix::Matrix2(v), 1, 4) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: new_ref(RowVector4::from_element($zero)),elements: 4,_marker: PhantomData}));},
                  #[cfg(feature = "Matrix2")]
                  (Matrix::Matrix2(v), 4, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: new_ref(Vector4::from_element($zero)),elements: 4,_marker: PhantomData}));},

                  #[cfg(feature = "Matrix3")]
                  (Matrix::Matrix3(v), 1, 9) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: new_ref(RowDVector::from_element(9, $zero)), elements: 9, _marker: PhantomData}));},
                  #[cfg(feature = "Matrix3")]
                  (Matrix::Matrix3(v), 9, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: new_ref(DVector::from_element(9, $zero)), elements: 9, _marker: PhantomData}));},

                  #[cfg(feature = "Matrix4")]
                  (Matrix::Matrix4(v), 1, 16) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: new_ref(RowDVector::from_element(16, $zero)), elements: 16, _marker: PhantomData}));},
                  #[cfg(feature = "Matrix4")]
                  (Matrix::Matrix4(v), 16, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: new_ref(DVector::from_element(16, $zero)), elements: 16, _marker: PhantomData}));},
                  
                  #[cfg(feature = "Matrix3x2")]
                  (Matrix::Matrix3x2(v), 1, 6) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowDVector::from_element(6, $zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix3x2")]
                  (Matrix::Matrix3x2(v), 6, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DVector::from_element(6, $zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix3x2")]
                  (Matrix::Matrix3x2(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix2x3::from_element($zero)), elements: 6, _marker: PhantomData })); },

                  #[cfg(feature = "Matrix2x3")]
                  (Matrix::Matrix2x3(v), 1, 6) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowDVector::from_element(6, $zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix2x3")]
                  (Matrix::Matrix2x3(v), 6, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DVector::from_element(6, $zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "Matrix2x3")]
                  (Matrix::Matrix2x3(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix3x2::from_element($zero)), elements: 6, _marker: PhantomData })); },

                  #[cfg(feature = "Vector4")]
                  (Matrix::Vector4(v), 2, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix2::from_element($zero)), elements: 4, _marker: PhantomData })); },
                  #[cfg(feature = "Vector2")]
                  (Matrix::Vector2(v), 1, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowVector2::from_element($zero)), elements: 2, _marker: PhantomData })); },
                  #[cfg(feature = "Vector3")]
                  (Matrix::Vector3(v), 1, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowVector3::from_element($zero)), elements: 3, _marker: PhantomData })); },
                  #[cfg(feature = "Vector4")]
                  (Matrix::Vector4(v), 1, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowVector4::from_element($zero)), elements: 4, _marker: PhantomData })); },

                  #[cfg(feature = "RowVector4")]
                  (Matrix::RowVector4(v), 2, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix2::from_element($zero)), elements: 4, _marker: PhantomData })); },
                  #[cfg(feature = "RowVector2")]
                  (Matrix::RowVector2(v), 2, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Vector2::from_element($zero)), elements: 2, _marker: PhantomData })); },
                  #[cfg(feature = "RowVector3")]
                  (Matrix::RowVector3(v), 3, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Vector3::from_element($zero)), elements: 3, _marker: PhantomData })); },
                  #[cfg(feature = "RowVector4")]
                  (Matrix::RowVector4(v), 4, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Vector4::from_element($zero)), elements: 4, _marker: PhantomData })); },
                  #[cfg(feature = "RowVectorD")]
                  (Matrix::RowDVector(v), 3, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix3::from_element($zero)), elements: 9, _marker: PhantomData })); },
                  #[cfg(feature = "RowVectorD")]
                  (Matrix::RowDVector(v), 4, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix4::from_element($zero)), elements: 16, _marker: PhantomData })); },

                  #[cfg(feature = "RowVectorD")]
                  (Matrix::RowDVector(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix2x3::from_element($zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "RowVectorD")]
                  (Matrix::RowDVector(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix3x2::from_element($zero)), elements: 6, _marker: PhantomData })); },

                  #[cfg(feature = "VectorD")]
                  (Matrix::DVector(v), 3, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix3::from_element($zero)), elements: 9, _marker: PhantomData })); },
                  #[cfg(feature = "VectorD")]
                  (Matrix::DVector(v), 4, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix4::from_element($zero)), elements: 16, _marker: PhantomData })); },
                  #[cfg(feature = "VectorD")]
                  (Matrix::DVector(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix3x2::from_element($zero)), elements: 6, _marker: PhantomData })); },
                  #[cfg(feature = "VectorD")]
                  (Matrix::DVector(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(Matrix2x3::from_element($zero)), elements: 6, _marker: PhantomData })); },
                  
                  #[cfg(feature = "VectorD")]
                  (Matrix::DVector(v), 1, n) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowDVector::from_element(n, $zero)), elements: n, _marker: PhantomData })); },
                  #[cfg(feature = "VectorD")]
                  (Matrix::DVector(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DMatrix::from_element(n, m, $zero)), elements: n * m, _marker: PhantomData })); },
                  #[cfg(feature = "RowVectorD")]
                  (Matrix::RowDVector(v), n, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DVector::from_element(n, $zero)), elements: n, _marker: PhantomData })); },
                   #[cfg(feature = "RowVectorD")]
                  (Matrix::RowDVector(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DMatrix::from_element(n, m, $zero)), elements: n, _marker: PhantomData })); },
                  #[cfg(feature = "MatrixD")]
                  (Matrix::DMatrix(v), n, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DVector::from_element(n, $zero)), elements: n, _marker: PhantomData })); },
                  #[cfg(feature = "MatrixD")]
                  (Matrix::DMatrix(v), 1, n) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(RowDVector::from_element(n, $zero)), elements: n, _marker: PhantomData })); },
                  #[cfg(feature = "MatrixD")]
                  (Matrix::DMatrix(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: new_ref(DMatrix::from_element(n, m, $zero)), elements: n * m, _marker: PhantomData })); },
                  _ => {
                    return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Cannot convert {:?} to {:?}", shape, dims), id: line!(), kind: MechErrorKind::None});
                  }
                }
              } else {
                return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Matrix dimensions do not match".to_string(), id: line!(), kind: MechErrorKind::None});
              }
            }
          )+
        )+
        x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{:?}", x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    }
  }
}

fn impl_conversion_mat_to_mat_fxn(source_value: Value, target_kind: ValueKind) -> MResult<Box<dyn MechFunction>>  {
  impl_conversion_mat_to_mat_match_arms!(
    (source_value, target_kind),
    F64 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero();
    F32 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F64, F64::zero(), F32, F32::zero();
    U8 => String, String::new(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), U8, u8::zero();
    U16 => String, String::new(), U8, u8::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), U16, u16::zero();
    U32 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), U32, u32::zero();
    U64 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), U64, u64::zero();
    U128 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), U128, u128::zero();
    I8 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), I8, i8::zero();
    I16 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I32, i32::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), I16, i16::zero();
    I32 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I64, i64::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), I32, i32::zero();
    I64 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I128, i128::zero(), F32, F32::zero(), F64, F64::zero(), I64, i64::zero();
    I128 => String, String::new(), U8, u8::zero(), U16, u16::zero(), U32, u32::zero(), U64, u64::zero(), U128, u128::zero(), I8, i8::zero(), I16, i16::zero(), I32, i32::zero(), I64, i64::zero(), F32, F32::zero(), F64, F64::zero(), I128, i128::zero();
  )
}

pub struct ConvertMatToMat {}

impl NativeFunctionCompiler for ConvertMatToMat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: String::new(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let source_value = arguments[0].clone();
    let source_kind = source_value.kind();
    let target_kind = arguments[1].kind();
    match impl_conversion_mat_to_mat_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_mat_to_mat_fxn(rhs.borrow().clone(), target_kind.clone()),
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
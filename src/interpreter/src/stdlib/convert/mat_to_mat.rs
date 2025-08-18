#[macro_use]
use crate::stdlib::*;

use nalgebra::{Scalar, Matrix3, Matrix4, DVector, ArrayStorage, Const};
use std::fmt::Debug;
use std::ops::{Index, IndexMut};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct ConvertMatToMat2<TFrom, TTo, FromMat, ToMat> {
    pub arg: Ref<FromMat>,
    pub out: Ref<ToMat>,
    _marker: PhantomData<(TFrom, TTo)>, 
}

impl<TFrom, TTo, FromMat, ToMat> MechFunction for ConvertMatToMat2<TFrom, TTo, FromMat, ToMat>
where
    Ref<ToMat>: ToValue,
    TFrom: LosslessInto<TTo> + Debug + Scalar + Clone,
    TTo: Debug + Scalar,
    for<'a> &'a FromMat: IntoIterator<Item = &'a TFrom>,
    for<'a> &'a mut ToMat: IntoIterator<Item = &'a mut TTo>,
    FromMat: Debug,
    ToMat: Debug,
{
    fn solve(&self) {
      let arg_ptr = self.arg.as_ptr();
      let out_ptr = self.out.as_mut_ptr();
      unsafe {
        let arg_ref: &FromMat = &*arg_ptr;
        let out_ref: &mut ToMat = &mut *out_ptr;
        for (dst, src) in (&mut *out_ref).into_iter().zip((&*arg_ref).into_iter()) {
          *dst = src.clone().lossless_into();
        }
      }
    }
    fn out(&self) -> Value {self.out.to_value()}
    fn to_string(&self) -> String { format!("{:#?}",self) }
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
      todo!();
    }
}

fn create_convert_mat_to_mat<TFrom, TTo>(
  v: Matrix<TFrom>,
  shape: &[usize],
) -> MResult<Box<dyn MechFunction>>
where
  Ref<na::Matrix1<TTo>>: ToValue,
  Ref<na::Matrix2<TTo>>: ToValue,
  Ref<na::Matrix3<TTo>>: ToValue,
  Ref<na::Matrix4<TTo>>: ToValue,
  Ref<na::Matrix3x2<TTo>>: ToValue,
  Ref<na::Matrix2x3<TTo>>: ToValue,
  Ref<na::RowVector2<TTo>>: ToValue,
  Ref<na::RowVector3<TTo>>: ToValue,
  Ref<na::RowVector4<TTo>>: ToValue,
  Ref<na::Vector2<TTo>>: ToValue,
  Ref<na::Vector3<TTo>>: ToValue,
  Ref<na::Vector4<TTo>>: ToValue,
  Ref<na::DVector<TTo>>: ToValue,
  Ref<na::RowDVector<TTo>>: ToValue,
  Ref<na::DMatrix<TTo>>: ToValue,
  TFrom: LosslessInto<TTo> + Debug + Scalar + Clone,
  TTo: Debug + Scalar + Default,
{
  let zero = TTo::default();
  match v {
    #[cfg(feature = "matrix1")]
    Matrix::Matrix1(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix1::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "matrix2")]
    Matrix::Matrix2(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "matrix3")]
    Matrix::Matrix3(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "matrix4")]
    Matrix::Matrix4(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix4::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "matrix3x2")]
    Matrix::Matrix3x2(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3x2::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "matrix2x3")]
    Matrix::Matrix2x3(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2x3::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "row_vector2")]
    Matrix::RowVector2(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector2::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "row_vector3")]
    Matrix::RowVector3(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector3::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "row_vector4")]
    Matrix::RowVector4(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector4::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "vector2")]
    Matrix::Vector2(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector2::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "vector3")]
    Matrix::Vector3(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector3::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "vector4")]
    Matrix::Vector4(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector4::from_element(zero)), _marker: PhantomData })),
    #[cfg(feature = "vectord")]
    Matrix::DVector(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(shape[0], zero)), _marker: PhantomData })),
    #[cfg(feature = "row_vectord")]
    Matrix::RowDVector(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(shape[1], zero)), _marker: PhantomData })),
    #[cfg(feature = "matrixd")]
    Matrix::DMatrix(v) => Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DMatrix::from_element(shape[0], shape[1], zero)), _marker: PhantomData })),
    _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Unknown matrix type".to_string(), id: line!(), kind: MechErrorKind::None}),
  }
}

fn create_reshape_mat_to_mat<TFrom, TTo>(
  v: Matrix<TFrom>,
  shape: &[usize],
) -> MResult<Box<dyn MechFunction>>
where
  Ref<na::Matrix1<TTo>>: ToValue,
  Ref<na::Matrix2<TTo>>: ToValue,
  Ref<na::Matrix3<TTo>>: ToValue,
  Ref<na::Matrix4<TTo>>: ToValue,
  Ref<na::Matrix3x2<TTo>>: ToValue,
  Ref<na::Matrix2x3<TTo>>: ToValue,
  Ref<na::RowVector2<TTo>>: ToValue,
  Ref<na::RowVector3<TTo>>: ToValue,
  Ref<na::RowVector4<TTo>>: ToValue,
  Ref<na::Vector2<TTo>>: ToValue,
  Ref<na::Vector3<TTo>>: ToValue,
  Ref<na::Vector4<TTo>>: ToValue,
  Ref<na::DVector<TTo>>: ToValue,
  Ref<na::RowDVector<TTo>>: ToValue,
  Ref<na::DMatrix<TTo>>: ToValue,
  TFrom: LosslessInto<TTo> + Debug + Scalar + Clone,
  TTo: Debug + Scalar + Default,
{
  let zero = TTo::default();
  let dims = v.shape();
  match (v,shape[0],shape[1]) {
    #[cfg(feature = "matrix2")]
    (Matrix::Matrix2(v), 1, 4) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(RowVector4::from_element(zero)), _marker: PhantomData}));},
    #[cfg(feature = "matrix2")]
    (Matrix::Matrix2(v), 4, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(Vector4::from_element(zero)), _marker: PhantomData}));},

    #[cfg(feature = "matrix3")]
    (Matrix::Matrix3(v), 1, 9) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(RowDVector::from_element(9, zero)), _marker: PhantomData}));},
    #[cfg(feature = "matrix3")]
    (Matrix::Matrix3(v), 9, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(DVector::from_element(9, zero)), _marker: PhantomData}));},

    #[cfg(feature = "matrix4")]
    (Matrix::Matrix4(v), 1, 16) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(RowDVector::from_element(16, zero)), _marker: PhantomData}));},
    #[cfg(feature = "matrix4")]
    (Matrix::Matrix4(v), 16, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(DVector::from_element(16, zero)), _marker: PhantomData}));},
    
    #[cfg(feature = "matrix3x2")]
    (Matrix::Matrix3x2(v), 1, 6) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(feature = "matrix3x2")]
    (Matrix::Matrix3x2(v), 6, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(feature = "matrix3x2")]
    (Matrix::Matrix3x2(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2x3::from_element(zero)), _marker: PhantomData })); },

    #[cfg(feature = "matrix2x3")]
    (Matrix::Matrix2x3(v), 1, 6) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(feature = "matrix2x3")]
    (Matrix::Matrix2x3(v), 6, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(feature = "matrix2x3")]
    (Matrix::Matrix2x3(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3x2::from_element(zero)), _marker: PhantomData })); },

    #[cfg(feature = "vector2")]
    (Matrix::Vector2(v), 1, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "vector3")]
    (Matrix::Vector3(v), 1, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector3::from_element(zero)), _marker: PhantomData })); },
    
    #[cfg(feature = "vector4")]
    (Matrix::Vector4(v), 1, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "vector4")]
    (Matrix::Vector4(v), 2, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2::from_element(zero)), _marker: PhantomData })); },

    #[cfg(feature = "row_vector2")]
    (Matrix::RowVector2(v), 2, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "row_vector3")]
    (Matrix::RowVector3(v), 3, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector3::from_element(zero)), _marker: PhantomData })); },
    
    #[cfg(feature = "row_vector4")]
    (Matrix::RowVector4(v), 4, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "row_vector4")]
    (Matrix::RowVector4(v), 2, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2::from_element(zero)), _marker: PhantomData })); },
    
    #[cfg(feature = "row_vectord")]
    (Matrix::RowDVector(v), 3, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "row_vectord")]
    (Matrix::RowDVector(v), 4, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "row_vectord")]
    (Matrix::RowDVector(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2x3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "row_vectord")]
    (Matrix::RowDVector(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3x2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "row_vectord")]
    (Matrix::RowDVector(v), n, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(feature = "row_vectord")]
    (Matrix::RowDVector(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DMatrix::from_element(n, m, zero)), _marker: PhantomData })); },
    
    #[cfg(feature = "vectord")]
    (Matrix::DVector(v), 3, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "vectord")]
    (Matrix::DVector(v), 4, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "vectord")]
    (Matrix::DVector(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3x2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "vectord")]
    (Matrix::DVector(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2x3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(feature = "vectord")]
    (Matrix::DVector(v), 1, n) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(feature = "vectord")]
    (Matrix::DVector(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DMatrix::from_element(n, m, zero)), _marker: PhantomData })); },
    
    #[cfg(feature = "matrixd")]
    (Matrix::DMatrix(v), n, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(feature = "matrixd")]
    (Matrix::DMatrix(v), 1, n) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(feature = "matrixd")]
    (Matrix::DMatrix(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DMatrix::from_element(n, m, zero)), _marker: PhantomData })); },
    _ => {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Cannot convert {:?} to {:?}", shape, dims), id: line!(), kind: MechErrorKind::None});
    }
  }
}

macro_rules! impl_conversion_mat_to_mat_fxn {
  (
    $(
      $src:tt, $src_string:tt => [ $( $dst:tt, $dst_string:tt ),+ $(,)? ]
    );+ $(;)?
  ) => {
    pub fn impl_conversion_mat_to_mat_fxn(
      source_value: Value,
      target_kind: ValueKind
    ) -> MResult<Box<dyn MechFunction>> {
      let shape = source_value.shape();

      paste::paste! {
        match (source_value, target_kind) {
          $(
            $(
              #[cfg(all(feature = "matrix", feature = $src_string, feature = $dst_string))]
              (Value::[<Matrix $src:camel>](v), ValueKind::Matrix(box ValueKind::[<$dst:camel>], dims)) => {
                if dims.is_empty() { 
                  create_convert_mat_to_mat::<$src, $dst>(v, &shape)
                } else if ((shape[0] == dims[0]) && (shape[1] == dims[1])) {
                  create_convert_mat_to_mat::<$src, $dst>(v, &dims)
                } else if shape[0] * shape[1] == dims[0] * dims[1] {
                  create_reshape_mat_to_mat::<$src, $dst>(v, &dims)
                } else {
                  Err(MechError {id: line!(),file: file!().to_string(),tokens: vec![],msg: "Unsupported conversion".to_string(),kind: MechErrorKind::None})
                }
              }
            )+
          )+
          _ => Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Unsupported conversion".to_string(),id: line!(),kind: MechErrorKind::None}),
        }
      }
    }
  };
}

#[cfg(not(target_arch = "wasm32"))]
impl_conversion_mat_to_mat_fxn! {
  F64, "f64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", RationalNumber, "rational"];
  F32, "f32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", RationalNumber, "rational"];
  u8,  "u8"  => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  u16, "u16" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  u32, "u32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  u64, "u64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  u128,"u128"=> [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  i8,  "i8"  => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  i16, "i16" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  i32, "i32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  i64, "i64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  i128,"i128"=> [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128"];
  String, "string" => [String, "string"];
  RationalNumber, "rational" => [String, "string"];
  ComplexNumber, "complex" => [String, "string"];
}


#[cfg(target_arch = "wasm32")]
impl_conversion_mat_to_mat_fxn! {
  F64, "f64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64", RationalNumber, "rational"];
  F32, "f32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64", RationalNumber, "rational"];
  u8,  "u8"  => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  u16, "u16" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  u32, "u32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  u64, "u64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i8,  "i8"  => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i16, "i16" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i32, "i32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i64, "i64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  String, "string" => [String, "string"];
  RationalNumber, "rational" => [String, "string"];
  ComplexNumber, "complex" => [String, "string"];
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
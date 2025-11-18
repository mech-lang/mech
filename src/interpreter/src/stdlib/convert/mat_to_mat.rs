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

impl<TFrom, TTo, FromMat, ToMat> MechFunctionImpl for ConvertMatToMat2<TFrom, TTo, FromMat, ToMat>
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
  }
#[cfg(feature = "compiler")]
impl<TFrom, TTo, FromMat, ToMat> MechFunctionCompiler for ConvertMatToMat2<TFrom, TTo, FromMat, ToMat> 
where
  TFrom: ConstElem + CompileConst + AsValueKind,
  TTo: ConstElem + CompileConst + AsValueKind,
  FromMat: CompileConst + ConstElem + AsValueKind,
  ToMat: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("ConvertMatToMat2<{},{}>", FromMat::as_value_kind(), ToMat::as_value_kind());
    compile_unop!(name, self.out, self.arg, ctx, FeatureFlag::Builtin(FeatureKind::Convert));
  }
}

fn create_convert_mat_to_mat<TFrom, TTo>(
  v: Matrix<TFrom>,
  shape: &[usize],
) -> MResult<Box<dyn MechFunction>>
where
  #[cfg(feature = "matrix1")]
  Ref<na::Matrix1<TTo>>: ToValue,
  #[cfg(feature = "matrix2")]
  Ref<na::Matrix2<TTo>>: ToValue,
  #[cfg(feature = "matrix3")]
  Ref<na::Matrix3<TTo>>: ToValue,
  #[cfg(feature = "matrix4")]
  Ref<na::Matrix4<TTo>>: ToValue,
  #[cfg(feature = "matrix3x2")]
  Ref<na::Matrix3x2<TTo>>: ToValue,
  #[cfg(feature = "matrix2x3")]
  Ref<na::Matrix2x3<TTo>>: ToValue,
  #[cfg(feature = "row_vector2")]
  Ref<na::RowVector2<TTo>>: ToValue,
  #[cfg(feature = "row_vector3")]
  Ref<na::RowVector3<TTo>>: ToValue,
  #[cfg(feature = "row_vector4")]
  Ref<na::RowVector4<TTo>>: ToValue,
  #[cfg(feature = "vector2")]
  Ref<na::Vector2<TTo>>: ToValue,
  #[cfg(feature = "vector3")]
  Ref<na::Vector3<TTo>>: ToValue,
  #[cfg(feature = "vector4")]
  Ref<na::Vector4<TTo>>: ToValue,
  #[cfg(feature = "vectord")]
  Ref<na::DVector<TTo>>: ToValue,
  #[cfg(feature = "row_vectord")]
  Ref<na::RowDVector<TTo>>: ToValue,
  #[cfg(feature = "matrixd")]
  Ref<na::DMatrix<TTo>>: ToValue,
  TFrom: LosslessInto<TTo> + Debug + Scalar + Clone + ConstElem + CompileConst + AsValueKind,
  TTo: Debug + Scalar + Default + ConstElem + CompileConst + AsValueKind,
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
    _ => Err(MechError2::new(
        FeatureNotEnabledError,
        None
      ).with_compiler_loc()
    ),
  }
}

fn create_reshape_mat_to_mat<TFrom, TTo>(
  v: Matrix<TFrom>,
  shape: &[usize],
) -> MResult<Box<dyn MechFunction>>
where
  #[cfg(feature = "matrix1")]
  Ref<na::Matrix1<TTo>>: ToValue,
  #[cfg(feature = "matrix2")]
  Ref<na::Matrix2<TTo>>: ToValue,
  #[cfg(feature = "matrix3")]
  Ref<na::Matrix3<TTo>>: ToValue,
  #[cfg(feature = "matrix4")]
  Ref<na::Matrix4<TTo>>: ToValue,
  #[cfg(feature = "matrix3x2")]
  Ref<na::Matrix3x2<TTo>>: ToValue,
  #[cfg(feature = "matrix2x3")]
  Ref<na::Matrix2x3<TTo>>: ToValue,
  #[cfg(feature = "row_vector2")]
  Ref<na::RowVector2<TTo>>: ToValue,
  #[cfg(feature = "row_vector3")]
  Ref<na::RowVector3<TTo>>: ToValue,
  #[cfg(feature = "row_vector4")]
  Ref<na::RowVector4<TTo>>: ToValue,
  #[cfg(feature = "vector2")]
  Ref<na::Vector2<TTo>>: ToValue,
  #[cfg(feature = "vector3")]
  Ref<na::Vector3<TTo>>: ToValue,
  #[cfg(feature = "vector4")]
  Ref<na::Vector4<TTo>>: ToValue,
  #[cfg(feature = "vectord")]
  Ref<na::DVector<TTo>>: ToValue,
  #[cfg(feature = "row_vectord")]
  Ref<na::RowDVector<TTo>>: ToValue,
  #[cfg(feature = "matrixd")]
  Ref<na::DMatrix<TTo>>: ToValue,
  TFrom: LosslessInto<TTo> + Debug + Scalar + Clone + ConstElem + CompileConst + AsValueKind,
  TTo: Debug + Scalar + Default + ConstElem + CompileConst + AsValueKind,
{
  let zero = TTo::default();
  let dims = v.shape();
  match (v,shape[0],shape[1]) {
    #[cfg(all(feature = "matrix2", feature = "row_vector4"))]
    (Matrix::Matrix2(v), 1, 4) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(RowVector4::from_element(zero)), _marker: PhantomData}));},
    #[cfg(all(feature = "matrix2", feature = "vector4"))]
    (Matrix::Matrix2(v), 4, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(Vector4::from_element(zero)), _marker: PhantomData}));},

    #[cfg(all(feature = "matrix3", feature = "row_vectord"))]
    (Matrix::Matrix3(v), 1, 9) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(RowDVector::from_element(9, zero)), _marker: PhantomData}));},
    #[cfg(all(feature = "matrix3", feature = "vectord"))]
    (Matrix::Matrix3(v), 9, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(DVector::from_element(9, zero)), _marker: PhantomData}));},

    #[cfg(all(feature = "matrix4", feature = "row_vectord"))]
    (Matrix::Matrix4(v), 1, 16) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(RowDVector::from_element(16, zero)), _marker: PhantomData}));},
    #[cfg(all(feature = "matrix4", feature = "vectord"))]
    (Matrix::Matrix4(v), 16, 1) => {return Ok(Box::new(ConvertMatToMat2 {arg: v,out: Ref::new(DVector::from_element(16, zero)), _marker: PhantomData}));},
    
    #[cfg(all(feature = "matrix3x2", feature = "row_vectord"))]
    (Matrix::Matrix3x2(v), 1, 6) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "matrix3x2", feature = "vectord"))]
    (Matrix::Matrix3x2(v), 6, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "matrix3x2", feature = "matrix2x3"))]
    (Matrix::Matrix3x2(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2x3::from_element(zero)), _marker: PhantomData })); },

    #[cfg(all(feature = "matrix2x3", feature = "row_vectord"))]
    (Matrix::Matrix2x3(v), 1, 6) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "matrix2x3", feature = "vectord"))]
    (Matrix::Matrix2x3(v), 6, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(6, zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "matrix2x3", feature = "matrix3x2"))]
    (Matrix::Matrix2x3(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3x2::from_element(zero)), _marker: PhantomData })); },

    #[cfg(all(feature = "vector2", feature = "row_vector2"))]
    (Matrix::Vector2(v), 1, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "vector3", feature = "row_vector3"))]
    (Matrix::Vector3(v), 1, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector3::from_element(zero)), _marker: PhantomData })); },
    
    #[cfg(all(feature = "vector4", feature = "row_vector4"))]
    (Matrix::Vector4(v), 1, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowVector4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "vector4", feature = "matrix2"))]
    (Matrix::Vector4(v), 2, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2::from_element(zero)), _marker: PhantomData })); },

    #[cfg(all(feature = "row_vector2", feature = "vector2"))]
    (Matrix::RowVector2(v), 2, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "row_vector3", feature = "vector3"))]
    (Matrix::RowVector3(v), 3, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector3::from_element(zero)), _marker: PhantomData })); },
    
    #[cfg(all(feature = "row_vector4", feature = "vector4"))]
    (Matrix::RowVector4(v), 4, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Vector4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "row_vector4", feature = "matrix2"))]
    (Matrix::RowVector4(v), 2, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2::from_element(zero)), _marker: PhantomData })); },
    
    #[cfg(all(feature = "row_vectord", feature = "matrix3"))]
    (Matrix::RowDVector(v), 3, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "row_vectord", feature = "matrix4"))]
    (Matrix::RowDVector(v), 4, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "row_vectord", feature = "matrix2x3"))]
    (Matrix::RowDVector(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2x3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "row_vectord", feature = "matrix3x2"))]
    (Matrix::RowDVector(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3x2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "row_vectord", feature = "vectord"))]
    (Matrix::RowDVector(v), n, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "row_vectord", feature = "matrixd"))]
    (Matrix::RowDVector(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DMatrix::from_element(n, m, zero)), _marker: PhantomData })); },
    
    #[cfg(all(feature = "vectord", feature = "matrix3"))]
    (Matrix::DVector(v), 3, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "vectord", feature = "matrix4"))]
    (Matrix::DVector(v), 4, 4) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix4::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "vectord", feature = "matrix3x2"))]
    (Matrix::DVector(v), 3, 2) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix3x2::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "vectord", feature = "matrix2x3"))]
    (Matrix::DVector(v), 2, 3) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(Matrix2x3::from_element(zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "vectord", feature = "row_vectord"))]
    (Matrix::DVector(v), 1, n) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "vectord", feature = "matrixd"))]
    (Matrix::DVector(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DMatrix::from_element(n, m, zero)), _marker: PhantomData })); },
    
    #[cfg(all(feature = "matrixd", feature = "vectord"))]
    (Matrix::DMatrix(v), n, 1) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
    (Matrix::DMatrix(v), 1, n) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(RowDVector::from_element(n, zero)), _marker: PhantomData })); },
    #[cfg(feature = "matrixd")]
    (Matrix::DMatrix(v), n, m) => { return Ok(Box::new(ConvertMatToMat2 { arg: v, out: Ref::new(DMatrix::from_element(n, m, zero)), _marker: PhantomData })); },
    _ => {
      return Err(MechError2::new(
        ReshapeError { original: (dims[0], dims[1]), requested: (shape[0], shape[1]) },
        None
      ).with_compiler_loc());
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
        match (source_value.clone(), target_kind.clone()) {
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
                  Err(MechError2::new(UnsupportedConversionError{from: source_value.kind(), to: target_kind.clone()}, None).with_compiler_loc())
                }
              }
            )+
          )+
          _ => Err(MechError2::new(UnsupportedConversionError{from: source_value.kind(), to: target_kind.clone()}, None).with_compiler_loc()),
        }
      }
    }
  };
}

#[cfg(not(target_arch = "wasm32"))]
impl_conversion_mat_to_mat_fxn! {
  F64, "f64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", R64, "rational"];
  F32, "f32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", R64, "rational"];
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
  R64, "rational" => [String, "string"];
  C64, "complex" => [String, "string"];
}


#[cfg(target_arch = "wasm32")]
impl_conversion_mat_to_mat_fxn! {
  F64, "f64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64", R64, "rational"];
  F32, "f32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64", R64, "rational"];
  u8,  "u8"  => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  u16, "u16" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  u32, "u32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  u64, "u64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i8,  "i8"  => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i16, "i16" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i32, "i32" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  i64, "i64" => [String, "string", F64, "f64", F32, "f32", u8, "u8", u16, "u16", u32, "u32", u64, "u64", i8, "i8", i16, "i16", i32, "i32", i64, "i64"];
  String, "string" => [String, "string"];
  R64, "rational" => [String, "string"];
  C64, "complex" => [String, "string"];
}

pub struct ConvertMatToMat {}

impl NativeFunctionCompiler for ConvertMatToMat {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let source_value = arguments[0].clone();
    let source_kind = source_value.kind();
    let target_kind = arguments[1].kind();
    match impl_conversion_mat_to_mat_fxn(source_value.clone(), target_kind.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match source_value {
          Value::MutableReference(rhs) => impl_conversion_mat_to_mat_fxn(rhs.borrow().clone(), target_kind.clone()),
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (arguments[0].kind(), arguments[1].kind()), fxn_name: "convert/mat-to-mat".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}



#[derive(Debug, Clone)]
pub struct ReshapeError {
  pub requested: (usize, usize),
  pub original: (usize, usize),
}
impl MechErrorKind2 for ReshapeError {
  fn name(&self) -> &str { "ReshapeError" }
  fn message(&self) -> String {
    format!("Cannot reshape matrix of shape {:?} into {:?}",self.original,self.requested)
  }
}
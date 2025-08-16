use crate::*;
use mech_core::*;
use libm::{atan2,atan2f};

// Atan2 ------------------------------------------------------------------------

macro_rules! atan2_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe{(*$out).0 = atan2((*$arg1).0,(*$arg2).0);}
  };}

macro_rules! atan2_vec_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe {
      let arg1_deref = &(*$arg1);
      let arg2_deref = &(*$arg2);
      let mut out_deref = (&mut *$out);
      for i in 0..arg1_deref.len() {
        (out_deref[i]).0 = atan2(arg1_deref[i].0,arg2_deref[i].0);
      }}};}

macro_rules! atan2f_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe{(*$out).0 = atan2f((*$arg1).0,(*$arg2).0);}
  };}

macro_rules! atan2f_vec_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe {
      let arg1_deref = &(*$arg1);
      let arg2_deref = &(*$arg2);
      let mut out_deref = (&mut *$out);
      for i in 0..arg1_deref.len() {
        (out_deref[i]).0 = atan2f(arg1_deref[i].0,arg2_deref[i].0);
      }}};}

macro_rules! impl_two_arg_fxn {
  ($struct_name:ident, $kind1:ty, $kind2:ty, $out_kind:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      arg1: Ref<$kind1>,
      arg2: Ref<$kind2>,
      out: Ref<$out_kind>,
    }
    impl MechFunction for $struct_name {
      fn solve(&self) {
        let arg1_ptr = self.arg1.as_ptr();
        let arg2_ptr = self.arg2.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg1_ptr,arg2_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

impl_two_arg_fxn!(Atan2F32, F32, F32, F32, atan2f_op);
#[cfg(feature = "matrix1")]
impl_two_arg_fxn!(Atan2M1F32, Matrix1<F32>, Matrix1<F32>, Matrix1<F32>, atan2f_vec_op);
#[cfg(feature = "matrix2")]
impl_two_arg_fxn!(Atan2M2F32, Matrix2<F32>, Matrix2<F32>, Matrix2<F32>, atan2f_vec_op);
#[cfg(feature = "matrix3")]
impl_two_arg_fxn!(Atan2M3F32, Matrix3<F32>, Matrix3<F32>, Matrix3<F32>, atan2f_vec_op);
#[cfg(feature = "matrix2x3")]
impl_two_arg_fxn!(Atan2M2x3F32, Matrix2x3<F32>, Matrix2x3<F32>, Matrix2x3<F32>, atan2f_vec_op);
#[cfg(feature = "matrix3")]
impl_two_arg_fxn!(Atan2M3x2F32, Matrix3x2<F32>, Matrix3x2<F32>, Matrix3x2<F32>, atan2f_vec_op);
#[cfg(feature = "matrix4")]
impl_two_arg_fxn!(Atan2M4F32, Matrix4<F32>, Matrix4<F32>, Matrix4<F32>, atan2f_vec_op);
#[cfg(feature = "vector2")]
impl_two_arg_fxn!(Atan2V2F32, Vector2<F32>, Vector2<F32>, Vector2<F32>, atan2f_vec_op);
#[cfg(feature = "vector3")]
impl_two_arg_fxn!(Atan2V3F32, Vector3<F32>, Vector3<F32>, Vector3<F32>, atan2f_vec_op);
#[cfg(feature = "vector4")]
impl_two_arg_fxn!(Atan2V4F32, Vector4<F32>, Vector4<F32>, Vector4<F32>, atan2f_vec_op);
#[cfg(feature = "row_vector2")]
impl_two_arg_fxn!(Atan2R2F32, RowVector2<F32>, RowVector2<F32>, RowVector2<F32>, atan2f_vec_op);
#[cfg(feature = "row_vector3")]
impl_two_arg_fxn!(Atan2R3F32, RowVector3<F32>, RowVector3<F32>, RowVector3<F32>, atan2f_vec_op);
#[cfg(feature = "row_vector4")]
impl_two_arg_fxn!(Atan2R4F32, RowVector4<F32>, RowVector4<F32>, RowVector4<F32>, atan2f_vec_op);
#[cfg(feature = "row_vectord")]
impl_two_arg_fxn!(Atan2RDF32, RowDVector<F32>, RowDVector<F32>, RowDVector<F32>, atan2f_vec_op);
#[cfg(feature = "vectord")]
impl_two_arg_fxn!(Atan2VDF32, DVector<F32>, DVector<F32>, DVector<F32>, atan2f_vec_op);
#[cfg(feature = "matrixd")]
impl_two_arg_fxn!(Atan2MDF32, DMatrix<F32>, DMatrix<F32>, DMatrix<F32>, atan2f_vec_op);

impl_two_arg_fxn!(Atan2F64, F64, F64, F64, atan2_op);
#[cfg(feature = "matrix1")]
impl_two_arg_fxn!(Atan2M1F64, Matrix1<F64>, Matrix1<F64>, Matrix1<F64>, atan2_vec_op);
#[cfg(feature = "matrix2")]
impl_two_arg_fxn!(Atan2M2F64, Matrix2<F64>, Matrix2<F64>, Matrix2<F64>, atan2_vec_op);
#[cfg(feature = "matrix3")]
impl_two_arg_fxn!(Atan2M3F64, Matrix3<F64>, Matrix3<F64>, Matrix3<F64>, atan2_vec_op);
#[cfg(feature = "matrix2x3")]
impl_two_arg_fxn!(Atan2M2x3F64, Matrix2x3<F64>, Matrix2x3<F64>, Matrix2x3<F64>, atan2_vec_op);
#[cfg(feature = "matrix3")]
impl_two_arg_fxn!(Atan2M3x2F64, Matrix3x2<F64>, Matrix3x2<F64>, Matrix3x2<F64>, atan2_vec_op);
#[cfg(feature = "matrix4")]
impl_two_arg_fxn!(Atan2M4F64, Matrix4<F64>, Matrix4<F64>, Matrix4<F64>, atan2_vec_op);
#[cfg(feature = "vector2")]
impl_two_arg_fxn!(Atan2V2F64, Vector2<F64>, Vector2<F64>, Vector2<F64>, atan2_vec_op);
#[cfg(feature = "vector3")]
impl_two_arg_fxn!(Atan2V3F64, Vector3<F64>, Vector3<F64>, Vector3<F64>, atan2_vec_op);
#[cfg(feature = "vector4")]
impl_two_arg_fxn!(Atan2V4F64, Vector4<F64>, Vector4<F64>, Vector4<F64>, atan2_vec_op);
#[cfg(feature = "row_vector2")]
impl_two_arg_fxn!(Atan2R2F64, RowVector2<F64>, RowVector2<F64>, RowVector2<F64>, atan2_vec_op);
#[cfg(feature = "row_vector3")]
impl_two_arg_fxn!(Atan2R3F64, RowVector3<F64>, RowVector3<F64>, RowVector3<F64>, atan2_vec_op);
#[cfg(feature = "row_vector4")]
impl_two_arg_fxn!(Atan2R4F64, RowVector4<F64>, RowVector4<F64>, RowVector4<F64>, atan2_vec_op);
#[cfg(feature = "row_vectord")]
impl_two_arg_fxn!(Atan2RDF64, RowDVector<F64>, RowDVector<F64>, RowDVector<F64>, atan2_vec_op);
#[cfg(feature = "vectord")]
impl_two_arg_fxn!(Atan2VDF64, DVector<F64>, DVector<F64>, DVector<F64>, atan2_vec_op);
#[cfg(feature = "matrixd")]
impl_two_arg_fxn!(Atan2MDF64, DMatrix<F64>, DMatrix<F64>, DMatrix<F64>, atan2_vec_op);

fn impl_atan2_fxn(arg1_value: Value, arg2_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (arg1_value,arg2_value) {
    #[cfg(feature = "f32")]
    (Value::F32(arg1),Value::F32(arg2)) => Ok(Box::new(Atan2F32{arg1, arg2, out: Ref::new(F32::zero())})),
    #[cfg(all(feature = "matrix1", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix1(arg1)),Value::MatrixF32(Matrix::Matrix1(arg2))) => Ok(Box::new(Atan2M1F32{arg1, arg2, out: Ref::new(Matrix1::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix2", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix2(arg1)),Value::MatrixF32(Matrix::Matrix2(arg2))) => Ok(Box::new(Atan2M2F32{arg1, arg2, out: Ref::new(Matrix2::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix3(arg1)),Value::MatrixF32(Matrix::Matrix3(arg2))) => Ok(Box::new(Atan2M3F32{arg1, arg2, out: Ref::new(Matrix3::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix2x3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix2x3(arg1)),Value::MatrixF32(Matrix::Matrix2x3(arg2))) => Ok(Box::new(Atan2M2x3F32{arg1, arg2, out: Ref::new(Matrix2x3::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix3x2(arg1)),Value::MatrixF32(Matrix::Matrix3x2(arg2))) => Ok(Box::new(Atan2M3x2F32{arg1, arg2, out: Ref::new(Matrix3x2::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix4", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix4(arg1)),Value::MatrixF32(Matrix::Matrix4(arg2))) => Ok(Box::new(Atan2M4F32{arg1, arg2, out: Ref::new(Matrix4::from_element(F32::zero()))})),
    #[cfg(all(feature = "vector2", feature = "f32"))]
    (Value::MatrixF32(Matrix::Vector2(arg1)),Value::MatrixF32(Matrix::Vector2(arg2))) => Ok(Box::new(Atan2V2F32{arg1, arg2, out: Ref::new(Vector2::from_element(F32::zero()))})),
    #[cfg(all(feature = "vector3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Vector3(arg1)),Value::MatrixF32(Matrix::Vector3(arg2))) => Ok(Box::new(Atan2V3F32{arg1, arg2, out: Ref::new(Vector3::from_element(F32::zero()))})),
    #[cfg(all(feature = "vector4", feature = "f32"))]
    (Value::MatrixF32(Matrix::Vector4(arg1)),Value::MatrixF32(Matrix::Vector4(arg2))) => Ok(Box::new(Atan2V4F32{arg1, arg2, out: Ref::new(Vector4::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vector2", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowVector2(arg1)),Value::MatrixF32(Matrix::RowVector2(arg2))) => Ok(Box::new(Atan2R2F32{arg1, arg2, out: Ref::new(RowVector2::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vector3", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowVector3(arg1)),Value::MatrixF32(Matrix::RowVector3(arg2))) => Ok(Box::new(Atan2R3F32{arg1, arg2, out: Ref::new(RowVector3::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vector4", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowVector4(arg1)),Value::MatrixF32(Matrix::RowVector4(arg2))) => Ok(Box::new(Atan2R4F32{arg1, arg2, out: Ref::new(RowVector4::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vectord", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowDVector(arg1)),Value::MatrixF32(Matrix::RowDVector(arg2))) => Ok(Box::new(Atan2RDF32{arg1: arg1.clone(), arg2, out: Ref::new(RowDVector::from_element(arg1.borrow().ncols(),F32::zero()))})),
    #[cfg(all(feature = "vectord", feature = "f32"))]
    (Value::MatrixF32(Matrix::DVector(arg1)),Value::MatrixF32(Matrix::DVector(arg2))) => Ok(Box::new(Atan2VDF32{arg1: arg1.clone(), arg2, out: Ref::new(DVector::from_element(arg1.borrow().nrows(),F32::zero()))})),
    #[cfg(all(feature = "matrixd", feature = "f32"))]
    (Value::MatrixF32(Matrix::DMatrix(arg1)),Value::MatrixF32(Matrix::DMatrix(arg2))) => {
      let rows = arg1.borrow().nrows();
      let cols = arg1.borrow().ncols();
      Ok(Box::new(Atan2MDF32{arg1, arg2, out: Ref::new(DMatrix::from_element(rows,cols,F32::zero()))}))
    },
    #[cfg(feature = "f64")]
    (Value::F64(arg1),Value::F64(arg2)) => Ok(Box::new(Atan2F64{arg1, arg2, out: Ref::new(F64::zero())})),
    #[cfg(all(feature = "matrix1", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix1(arg1)),Value::MatrixF64(Matrix::Matrix1(arg2))) => Ok(Box::new(Atan2M1F64{arg1, arg2, out: Ref::new(Matrix1::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix2", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix2(arg1)),Value::MatrixF64(Matrix::Matrix2(arg2))) => Ok(Box::new(Atan2M2F64{arg1, arg2, out: Ref::new(Matrix2::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix3(arg1)),Value::MatrixF64(Matrix::Matrix3(arg2))) => Ok(Box::new(Atan2M3F64{arg1, arg2, out: Ref::new(Matrix3::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix2x3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix2x3(arg1)),Value::MatrixF64(Matrix::Matrix2x3(arg2))) => Ok(Box::new(Atan2M2x3F64{arg1, arg2, out: Ref::new(Matrix2x3::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix3x2(arg1)),Value::MatrixF64(Matrix::Matrix3x2(arg2))) => Ok(Box::new(Atan2M3x2F64{arg1, arg2, out: Ref::new(Matrix3x2::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix4", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix4(arg1)),Value::MatrixF64(Matrix::Matrix4(arg2))) => Ok(Box::new(Atan2M4F64{arg1, arg2, out: Ref::new(Matrix4::from_element(F64::zero()))})),
    #[cfg(all(feature = "vector2", feature = "f64"))]
    (Value::MatrixF64(Matrix::Vector2(arg1)),Value::MatrixF64(Matrix::Vector2(arg2))) => Ok(Box::new(Atan2V2F64{arg1, arg2, out: Ref::new(Vector2::from_element(F64::zero()))})),
    #[cfg(all(feature = "vector3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Vector3(arg1)),Value::MatrixF64(Matrix::Vector3(arg2))) => Ok(Box::new(Atan2V3F64{arg1, arg2, out: Ref::new(Vector3::from_element(F64::zero()))})),
    #[cfg(all(feature = "vector4", feature = "f64"))]
    (Value::MatrixF64(Matrix::Vector4(arg1)),Value::MatrixF64(Matrix::Vector4(arg2))) => Ok(Box::new(Atan2V4F64{arg1, arg2, out: Ref::new(Vector4::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vector2", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowVector2(arg1)),Value::MatrixF64(Matrix::RowVector2(arg2))) => Ok(Box::new(Atan2R2F64{arg1, arg2, out: Ref::new(RowVector2::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vector3", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowVector3(arg1)),Value::MatrixF64(Matrix::RowVector3(arg2))) => Ok(Box::new(Atan2R3F64{arg1, arg2, out: Ref::new(RowVector3::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vector4", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowVector4(arg1)),Value::MatrixF64(Matrix::RowVector4(arg2))) => Ok(Box::new(Atan2R4F64{arg1, arg2, out: Ref::new(RowVector4::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vectord", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowDVector(arg1)),Value::MatrixF64(Matrix::RowDVector(arg2))) => Ok(Box::new(Atan2RDF64{arg1: arg1.clone(), arg2, out: Ref::new(RowDVector::from_element(arg1.borrow().ncols(),F64::zero()))})),
    #[cfg(all(feature = "vectord", feature = "f64"))]
    (Value::MatrixF64(Matrix::DVector(arg1)),Value::MatrixF64(Matrix::DVector(arg2))) => Ok(Box::new(Atan2VDF64{arg1: arg1.clone(), arg2, out: Ref::new(DVector::from_element(arg1.borrow().nrows(),F64::zero()))})),
    #[cfg(all(feature = "matrixd", feature = "f64"))]
    (Value::MatrixF64(Matrix::DMatrix(arg1)),Value::MatrixF64(Matrix::DMatrix(arg2))) => {
      let rows = arg1.borrow().nrows();
      let cols = arg1.borrow().ncols();
      Ok(Box::new(Atan2MDF64{arg1, arg2, out: Ref::new(DMatrix::from_element(rows,cols,F64::zero()))}))
    },
    x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}

pub struct MathAtan2 {}

impl NativeFunctionCompiler for MathAtan2 {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let arg1 = arguments[0].clone();
    let arg2 = arguments[1].clone();
    match impl_atan2_fxn(arg1.clone(), arg2.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (arg1,arg2) {
          (Value::MutableReference(arg1),Value::MutableReference(arg2)) => {impl_atan2_fxn(arg1.borrow().clone(),arg2.borrow().clone())}
          (Value::MutableReference(arg1),arg2) => {impl_atan2_fxn(arg1.borrow().clone(),arg2.clone())}
          (arg1,Value::MutableReference(arg2)) => {impl_atan2_fxn(arg1.clone(),arg2.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}
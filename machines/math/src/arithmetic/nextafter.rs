use crate::*;
use mech_core::*;
use libm::{nextafter,nextafterf};
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Nextafter ------------------------------------------------------------------------

macro_rules! nextafter_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe{(*$out).0 = nextafter((*$arg1).0,(*$arg2).0);}
  };}

macro_rules! nextafter_vec_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe {
      let arg1_deref = &(*$arg1);
      let arg2_deref = &(*$arg2);
      let mut out_deref = (&mut *$out);
      for i in 0..arg1_deref.len() {
        (out_deref[i]).0 = nextafter(arg1_deref[i].0,arg2_deref[i].0);
      }}};}

macro_rules! nextafterf_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe{(*$out).0 = nextafterf((*$arg1).0,(*$arg2).0);}
  };}

macro_rules! nextafterf_vec_op {
  ($arg1:expr, $arg2:expr, $out:expr) => {
    unsafe {
      let arg1_deref = &(*$arg1);
      let arg2_deref = &(*$arg2);
      let mut out_deref = (&mut *$out);
      for i in 0..arg1_deref.len() {
        (out_deref[i]).0 = nextafterf(arg1_deref[i].0,arg2_deref[i].0);
      }}};}

macro_rules! impl_two_arg_fxn {
  ($struct_name:ident, $kind1:ty, $kind2:ty, $out_kind:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      arg1: Ref<$kind1>,
      arg2: Ref<$kind2>,
      out: Ref<$out_kind>,
    }
    impl MechFunctionImpl for $struct_name {
      fn solve(&self) {
        let arg1_ptr = self.arg1.as_ptr();
        let arg2_ptr = self.arg2.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg1_ptr,arg2_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }
    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for $struct_name {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
    }};}

#[cfg(all(feature = "f32", feature = "matrix1"))]
impl_two_arg_fxn!(NextafterM1F32, Matrix1<F32>, Matrix1<F32>, Matrix1<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "matrix2"))]
impl_two_arg_fxn!(NextafterM2F32, Matrix2<F32>, Matrix2<F32>, Matrix2<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(NextafterM3F32, Matrix3<F32>, Matrix3<F32>, Matrix3<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "matrix2x3"))]
impl_two_arg_fxn!(NextafterM2x3F32, Matrix2x3<F32>, Matrix2x3<F32>, Matrix2x3<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "matrix3"))]
impl_two_arg_fxn!(NextafterM3x2F32, Matrix3x2<F32>, Matrix3x2<F32>, Matrix3x2<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "matrix4"))]
impl_two_arg_fxn!(NextafterM4F32, Matrix4<F32>, Matrix4<F32>, Matrix4<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "vector2"))]
impl_two_arg_fxn!(NextafterV2F32, Vector2<F32>, Vector2<F32>, Vector2<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "vector3"))]
impl_two_arg_fxn!(NextafterV3F32, Vector3<F32>, Vector3<F32>, Vector3<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "vector4"))]
impl_two_arg_fxn!(NextafterV4F32, Vector4<F32>, Vector4<F32>, Vector4<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "row_vector2"))]
impl_two_arg_fxn!(NextafterR2F32, RowVector2<F32>, RowVector2<F32>, RowVector2<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "row_vector3"))]
impl_two_arg_fxn!(NextafterR3F32, RowVector3<F32>, RowVector3<F32>, RowVector3<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "row_vector4"))]
impl_two_arg_fxn!(NextafterR4F32, RowVector4<F32>, RowVector4<F32>, RowVector4<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "row_vectord"))]
impl_two_arg_fxn!(NextafterRDF32, RowDVector<F32>, RowDVector<F32>, RowDVector<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "vectord"))]
impl_two_arg_fxn!(NextafterVDF32, DVector<F32>, DVector<F32>, DVector<F32>, nextafterf_vec_op);
#[cfg(all(feature = "f32", feature = "matrixd"))]
impl_two_arg_fxn!(NextafterMDF32, DMatrix<F32>, DMatrix<F32>, DMatrix<F32>, nextafterf_vec_op);

#[cfg(feature = "f32")]
impl_two_arg_fxn!(NextafterF32, F32, F32, F32, nextafterf_op);

#[cfg(all(feature = "f64", feature = "matrix1"))]
impl_two_arg_fxn!(NextafterM1F64, Matrix1<F64>, Matrix1<F64>, Matrix1<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "matrix2"))]
impl_two_arg_fxn!(NextafterM2F64, Matrix2<F64>, Matrix2<F64>, Matrix2<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(NextafterM3F64, Matrix3<F64>, Matrix3<F64>, Matrix3<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "matrix2x3"))]
impl_two_arg_fxn!(NextafterM2x3F64, Matrix2x3<F64>, Matrix2x3<F64>, Matrix2x3<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "matrix3"))]
impl_two_arg_fxn!(NextafterM3x2F64, Matrix3x2<F64>, Matrix3x2<F64>, Matrix3x2<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "matrix4"))]
impl_two_arg_fxn!(NextafterM4F64, Matrix4<F64>, Matrix4<F64>, Matrix4<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "vector2"))]
impl_two_arg_fxn!(NextafterV2F64, Vector2<F64>, Vector2<F64>, Vector2<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "vector3"))]
impl_two_arg_fxn!(NextafterV3F64, Vector3<F64>, Vector3<F64>, Vector3<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "vector4"))]
impl_two_arg_fxn!(NextafterV4F64, Vector4<F64>, Vector4<F64>, Vector4<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "row_vector2"))]
impl_two_arg_fxn!(NextafterR2F64, RowVector2<F64>, RowVector2<F64>, RowVector2<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "row_vector3"))]
impl_two_arg_fxn!(NextafterR3F64, RowVector3<F64>, RowVector3<F64>, RowVector3<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "row_vector4"))]
impl_two_arg_fxn!(NextafterR4F64, RowVector4<F64>, RowVector4<F64>, RowVector4<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "row_vectord"))]
impl_two_arg_fxn!(NextafterRDF64, RowDVector<F64>, RowDVector<F64>, RowDVector<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "vectord"))]
impl_two_arg_fxn!(NextafterVDF64, DVector<F64>, DVector<F64>, DVector<F64>, nextafter_vec_op);
#[cfg(all(feature = "f64", feature = "matrixd"))]
impl_two_arg_fxn!(NextafterMDF64, DMatrix<F64>, DMatrix<F64>, DMatrix<F64>, nextafter_vec_op);

#[cfg(feature = "f64")]
impl_two_arg_fxn!(NextafterF64, F64, F64, F64, nextafter_op);

fn impl_nextafter_fxn(arg1_value: Value, arg2_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (arg1_value,arg2_value) {
    #[cfg(feature = "f32")]
    (Value::F32(arg1),Value::F32(arg2)) => Ok(Box::new(NextafterF32{arg1, arg2, out: Ref::new(F32::zero())})),
    #[cfg(all(feature = "matrix1", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix1(arg1)),Value::MatrixF32(Matrix::Matrix1(arg2))) => Ok(Box::new(NextafterM1F32{arg1, arg2, out: Ref::new(Matrix1::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix2", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix2(arg1)),Value::MatrixF32(Matrix::Matrix2(arg2))) => Ok(Box::new(NextafterM2F32{arg1, arg2, out: Ref::new(Matrix2::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix3(arg1)),Value::MatrixF32(Matrix::Matrix3(arg2))) => Ok(Box::new(NextafterM3F32{arg1, arg2, out: Ref::new(Matrix3::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix2x3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix2x3(arg1)),Value::MatrixF32(Matrix::Matrix2x3(arg2))) => Ok(Box::new(NextafterM2x3F32{arg1, arg2, out: Ref::new(Matrix2x3::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix3x2(arg1)),Value::MatrixF32(Matrix::Matrix3x2(arg2))) => Ok(Box::new(NextafterM3x2F32{arg1, arg2, out: Ref::new(Matrix3x2::from_element(F32::zero()))})),
    #[cfg(all(feature = "matrix4", feature = "f32"))]
    (Value::MatrixF32(Matrix::Matrix4(arg1)),Value::MatrixF32(Matrix::Matrix4(arg2))) => Ok(Box::new(NextafterM4F32{arg1, arg2, out: Ref::new(Matrix4::from_element(F32::zero()))})),
    #[cfg(all(feature = "vector2", feature = "f32"))]
    (Value::MatrixF32(Matrix::Vector2(arg1)),Value::MatrixF32(Matrix::Vector2(arg2))) => Ok(Box::new(NextafterV2F32{arg1, arg2, out: Ref::new(Vector2::from_element(F32::zero()))})),
    #[cfg(all(feature = "vector3", feature = "f32"))]
    (Value::MatrixF32(Matrix::Vector3(arg1)),Value::MatrixF32(Matrix::Vector3(arg2))) => Ok(Box::new(NextafterV3F32{arg1, arg2, out: Ref::new(Vector3::from_element(F32::zero()))})),
    #[cfg(all(feature = "vector4", feature = "f32"))]
    (Value::MatrixF32(Matrix::Vector4(arg1)),Value::MatrixF32(Matrix::Vector4(arg2))) => Ok(Box::new(NextafterV4F32{arg1, arg2, out: Ref::new(Vector4::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vector2", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowVector2(arg1)),Value::MatrixF32(Matrix::RowVector2(arg2))) => Ok(Box::new(NextafterR2F32{arg1, arg2, out: Ref::new(RowVector2::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vector3", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowVector3(arg1)),Value::MatrixF32(Matrix::RowVector3(arg2))) => Ok(Box::new(NextafterR3F32{arg1, arg2, out: Ref::new(RowVector3::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vector4", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowVector4(arg1)),Value::MatrixF32(Matrix::RowVector4(arg2))) => Ok(Box::new(NextafterR4F32{arg1, arg2, out: Ref::new(RowVector4::from_element(F32::zero()))})),
    #[cfg(all(feature = "row_vectord", feature = "f32"))]
    (Value::MatrixF32(Matrix::RowDVector(arg1)),Value::MatrixF32(Matrix::RowDVector(arg2))) => Ok(Box::new(NextafterRDF32{arg1: arg1.clone(), arg2, out: Ref::new(RowDVector::from_element(arg1.borrow().ncols(),F32::zero()))})),
    #[cfg(all(feature = "vectord", feature = "f32"))]
    (Value::MatrixF32(Matrix::DVector(arg1)),Value::MatrixF32(Matrix::DVector(arg2))) => Ok(Box::new(NextafterVDF32{arg1: arg1.clone(), arg2, out: Ref::new(DVector::from_element(arg1.borrow().nrows(),F32::zero()))})),
    #[cfg(all(feature = "matrixd", feature = "f32"))]
    (Value::MatrixF32(Matrix::DMatrix(arg1)),Value::MatrixF32(Matrix::DMatrix(arg2))) => {
      let rows = arg1.borrow().nrows();
      let cols = arg1.borrow().ncols();
      Ok(Box::new(NextafterMDF32{arg1, arg2, out: Ref::new(DMatrix::from_element(rows,cols,F32::zero()))}))
    },
    #[cfg(feature = "f64")]
    (Value::F64(arg1),Value::F64(arg2)) => Ok(Box::new(NextafterF64{arg1, arg2, out: Ref::new(F64::zero())})),
    #[cfg(all(feature = "matrix1", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix1(arg1)),Value::MatrixF64(Matrix::Matrix1(arg2))) => Ok(Box::new(NextafterM1F64{arg1, arg2, out: Ref::new(Matrix1::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix2", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix2(arg1)),Value::MatrixF64(Matrix::Matrix2(arg2))) => Ok(Box::new(NextafterM2F64{arg1, arg2, out: Ref::new(Matrix2::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix3(arg1)),Value::MatrixF64(Matrix::Matrix3(arg2))) => Ok(Box::new(NextafterM3F64{arg1, arg2, out: Ref::new(Matrix3::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix2x3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix2x3(arg1)),Value::MatrixF64(Matrix::Matrix2x3(arg2))) => Ok(Box::new(NextafterM2x3F64{arg1, arg2, out: Ref::new(Matrix2x3::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix3x2(arg1)),Value::MatrixF64(Matrix::Matrix3x2(arg2))) => Ok(Box::new(NextafterM3x2F64{arg1, arg2, out: Ref::new(Matrix3x2::from_element(F64::zero()))})),
    #[cfg(all(feature = "matrix4", feature = "f64"))]
    (Value::MatrixF64(Matrix::Matrix4(arg1)),Value::MatrixF64(Matrix::Matrix4(arg2))) => Ok(Box::new(NextafterM4F64{arg1, arg2, out: Ref::new(Matrix4::from_element(F64::zero()))})),
    #[cfg(all(feature = "vector2", feature = "f64"))]
    (Value::MatrixF64(Matrix::Vector2(arg1)),Value::MatrixF64(Matrix::Vector2(arg2))) => Ok(Box::new(NextafterV2F64{arg1, arg2, out: Ref::new(Vector2::from_element(F64::zero()))})),
    #[cfg(all(feature = "vector3", feature = "f64"))]
    (Value::MatrixF64(Matrix::Vector3(arg1)),Value::MatrixF64(Matrix::Vector3(arg2))) => Ok(Box::new(NextafterV3F64{arg1, arg2, out: Ref::new(Vector3::from_element(F64::zero()))})),
    #[cfg(all(feature = "vector4", feature = "f64"))]
    (Value::MatrixF64(Matrix::Vector4(arg1)),Value::MatrixF64(Matrix::Vector4(arg2))) => Ok(Box::new(NextafterV4F64{arg1, arg2, out: Ref::new(Vector4::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vector2", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowVector2(arg1)),Value::MatrixF64(Matrix::RowVector2(arg2))) => Ok(Box::new(NextafterR2F64{arg1, arg2, out: Ref::new(RowVector2::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vector3", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowVector3(arg1)),Value::MatrixF64(Matrix::RowVector3(arg2))) => Ok(Box::new(NextafterR3F64{arg1, arg2, out: Ref::new(RowVector3::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vector4", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowVector4(arg1)),Value::MatrixF64(Matrix::RowVector4(arg2))) => Ok(Box::new(NextafterR4F64{arg1, arg2, out: Ref::new(RowVector4::from_element(F64::zero()))})),
    #[cfg(all(feature = "row_vectord", feature = "f64"))]
    (Value::MatrixF64(Matrix::RowDVector(arg1)),Value::MatrixF64(Matrix::RowDVector(arg2))) => Ok(Box::new(NextafterRDF64{arg1: arg1.clone(), arg2, out: Ref::new(RowDVector::from_element(arg1.borrow().ncols(),F64::zero()))})),
    #[cfg(all(feature = "vectord", feature = "f64"))]
    (Value::MatrixF64(Matrix::DVector(arg1)),Value::MatrixF64(Matrix::DVector(arg2))) => Ok(Box::new(NextafterVDF64{arg1: arg1.clone(), arg2, out: Ref::new(DVector::from_element(arg1.borrow().nrows(),F64::zero()))})),
    #[cfg(all(feature = "matrixd", feature = "f64"))]
    (Value::MatrixF64(Matrix::DMatrix(arg1)),Value::MatrixF64(Matrix::DMatrix(arg2))) => {
      let rows = arg1.borrow().nrows();
      let cols = arg1.borrow().ncols();
      Ok(Box::new(NextafterMDF64{arg1, arg2, out: Ref::new(DMatrix::from_element(rows,cols,F64::zero()))}))
    },
    x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}

pub struct MathNextafter {}

impl NativeFunctionCompiler for MathNextafter {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let arg1 = arguments[0].clone();
    let arg2 = arguments[1].clone();
    match impl_nextafter_fxn(arg1.clone(), arg2.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (arg1,arg2) {
          (Value::MutableReference(arg1),Value::MutableReference(arg2)) => {impl_nextafter_fxn(arg1.borrow().clone(),arg2.borrow().clone())}
          (Value::MutableReference(arg1),arg2) => {impl_nextafter_fxn(arg1.borrow().clone(),arg2.clone())}
          (arg1,Value::MutableReference(arg2)) => {impl_nextafter_fxn(arg1.clone(),arg2.borrow().clone())}
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

register_descriptor! {
  FunctionCompilerDescriptor {
    name: "math/nextafter",
    ptr: &MathNextafter{},
  }
}
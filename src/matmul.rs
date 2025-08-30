use crate::*;
use mech_core::*;

// MatMul ---------------------------------------------------------------------

macro_rules! mul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs * *$rhs; }
  };}

macro_rules! matmul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).mul_to(&*$rhs,&mut *$out); }
  };}

impl_binop!(MatMulScalar, T,T,T,mul_op, FeatureFlag::Builtin(FeatureKind::Mul));
#[cfg(all(feature = "row_vector4", feature = "vector4", feature = "matrix1"))]
impl_binop!(MatMulR4V4, RowVector4<T>, Vector4<T>, Matrix1<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector4", feature = "matrix4"))]
impl_binop!(MatMulR4M4, RowVector4<T>, Matrix4<T>, RowVector4<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vectord"))]
impl_binop!(MatMulR4MD, RowVector4<T>, DMatrix<T>, RowDVector<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "row_vector3", feature = "vector3", feature = "matrix1"))]
impl_binop!(MatMulR3V3, RowVector3<T>, Vector3<T>, Matrix1<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector3", feature = "matrix3"))]
impl_binop!(MatMulR3M3, RowVector3<T>, Matrix3<T>, RowVector3<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector3", feature = "matrix3x2"))]
impl_binop!(MatMulR3M3x2, RowVector3<T>, Matrix3x2<T>, RowVector2<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
impl_binop!(MatMulR3MD, RowVector3<T>, DMatrix<T>, RowDVector<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
impl_binop!(MatMulR2V2, RowVector2<T>, Vector2<T>, Matrix1<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "row_vector2"))]
impl_binop!(MatMulR2M2, RowVector2<T>, Matrix2<T>, RowVector2<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector2", feature = "matrix2x3", feature = "row_vector3"))]
impl_binop!(MatMulR2M2x3, RowVector2<T>, Matrix2x3<T>, RowVector3<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
impl_binop!(MatMulR2MD, RowVector2<T>, DMatrix<T>, RowDVector<T>, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "row_vectord", feature = "vectord", feature = "matrix1"))]
impl_binop!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "row_vectord", feature = "matrixd"))]
impl_binop!(MatMulRDMD, RowDVector<T>, DMatrix<T>, RowDVector<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
impl_binop!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
impl_binop!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
impl_binop!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
impl_binop!(MatMulVDRD, DVector<T>,RowDVector<T>,DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "matrix4", feature = "vector4"))]
impl_binop!(MatMulM4V4, Matrix4<T>, Vector4<T>, Vector4<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix4"))]
impl_binop!(MatMulM4M4, Matrix4<T>, Matrix4<T>, Matrix4<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix4", feature = "matrixd"))]
impl_binop!(MatMulM4MD, Matrix4<T>, DMatrix<T>, DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
impl_binop!(MatMulM2M2x3, Matrix2<T>, Matrix2x3<T>, Matrix2x3<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix2", feature = "matrix2"))]
impl_binop!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix2", feature = "vector2"))]
impl_binop!(MatMulM2V2, Matrix2<T>, Vector2<T>, Vector2<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix2", feature = "matrixd"))]
impl_binop!(MatMulM2MD, Matrix2<T>, DMatrix<T>, DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(feature = "matrix3")]
impl_binop!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix3", feature = "matrix3x2"))]
impl_binop!(MatMulM2M3x2, Matrix3<T>, Matrix3x2<T>, Matrix3x2<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix3", feature = "vector3"))]
impl_binop!(MatMulM3V3, Matrix3<T>, Vector3<T>, Vector3<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix3", feature = "matrixd"))]
impl_binop!(MatMulM3MD, Matrix3<T>, DMatrix<T>, DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "matrix1"))]
impl_binop!(MatMulM1M1, Matrix1<T>, Matrix1<T>, Matrix1<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
impl_binop!(MatMulM2x3V2, Matrix2x3<T>, Vector3<T>, Vector2<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix2x3", feature = "matrix3"))]
impl_binop!(MatMulM2x3M3, Matrix2x3<T>, Matrix3<T>, Matrix2x3<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = "matrix2"))]
impl_binop!(MatMulM2x3M3x2, Matrix2x3<T>, Matrix3x2<T>, Matrix2<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix2x3", feature = "matrixd"))]
impl_binop!(MatMulM2x3MD, Matrix2x3<T>, DMatrix<T>, DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(all(feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
impl_binop!(MatMulM3x2V2, Matrix3x2<T>, Vector2<T>, Vector3<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix3x2", feature = "matrix2"))]
impl_binop!(MatMulM3x2M2, Matrix3x2<T>, Matrix2<T>, Matrix3x2<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = "matrix3"))]
impl_binop!(MatMulM3x2M2x3, Matrix3x2<T>, Matrix2x3<T>, Matrix3<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrix3x2", feature = "matrixd"))]
impl_binop!(MatMulM3x2MD, Matrix3x2<T>, DMatrix<T>, DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

#[cfg(feature = "matrixd")]
impl_binop!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrixd", feature = "matrix3x2"))]
impl_binop!(MatMulMDM3x2, DMatrix<T>,Matrix3x2<T>,DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrixd", feature = "vectord"))]
impl_binop!(MatMulMDVD, DMatrix<T>,DVector<T>,DVector<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
#[cfg(all(feature = "matrixd", feature = "row_vectord"))]
impl_binop!(MatMulMDRD, DMatrix<T>,RowDVector<T>,DMatrix<T>,matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));

macro_rules! impl_matmul_match_arms {
  ($arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          #[cfg(feature = $value_string)]
          (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($target_type::zero()) })),
          #[cfg(feature = "Vector4")]
          (Value::$matrix_kind(Matrix::Vector4(lhs)),    Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new(MatMulV4R4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix4::from_element($target_type::zero())) })),
          #[cfg(feature = "Vector3")]
          (Value::$matrix_kind(Matrix::Vector3(lhs)),    Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new(MatMulV3R3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix3::from_element($target_type::zero())) })),
          #[cfg(feature = "Vector2")]
          (Value::$matrix_kind(Matrix::Vector2(lhs)),    Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new(MatMulV2R2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix2::from_element($target_type::zero())) })),
          #[cfg(feature = "RowVector4")]
          (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new(MatMulR4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          #[cfg(feature = "RowVector3")]
          (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulR3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          #[cfg(feature = "RowVector2")]
          (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulR2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          #[cfg(feature = "Matrix4")]
          (Value::$matrix_kind(Matrix::Matrix4(lhs)),    Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new(MatMulM4M4{lhs, rhs, out: new_ref(Matrix4::from_element($target_type::zero()))})),
          #[cfg(feature = "Matrix3")]
          (Value::$matrix_kind(Matrix::Matrix3(lhs)),    Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulM3M3{lhs, rhs, out: new_ref(Matrix3::from_element($target_type::zero()))})),
          #[cfg(feature = "Matrix2")]
          (Value::$matrix_kind(Matrix::Matrix2(lhs)),    Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulM2M2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))})),
          #[cfg(feature = "Matrix1")]
          (Value::$matrix_kind(Matrix::Matrix1(lhs)),    Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new(MatMulM1M1{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))})),
          #[cfg(feature = "Matrix2x3")]
          (Value::$matrix_kind(Matrix::Matrix2x3(lhs)),  Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulM2x3M3x2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))})),          
          #[cfg(feature = "RowVectorD")]
          (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new(MatMulRDVD{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))})),
          #[cfg(feature = "VectorD")]
          (Value::$matrix_kind(Matrix::DVector(lhs)),    Value::$matrix_kind(Matrix::RowDVector(rhs))) => {
            let rows = {lhs.borrow().len()};
            let cols = {rhs.borrow().len()};
            Ok(Box::new(MatMulVDRD{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
          },
          #[cfg(feature = "MatrixD")]
          (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
            let (rows,_) = {lhs.borrow().shape()};
            let (_,cols) = {rhs.borrow().shape()};
            Ok(Box::new(MatMulMDMD{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
          },
        )+
      )+
      x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

fn impl_matmul_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_matmul_match_arms!(
    (lhs_value, rhs_value),
    I8,   I8   => MatrixI8,   i8, "I8";
    I16,  I16  => MatrixI16,  i16, "I16";
    I32,  I32  => MatrixI32,  i32, "I32";
    I64,  I64  => MatrixI64,  i64, "I64";
    I128, I128 => MatrixI128, i128, "I128";
    U8,   U8   => MatrixU8,   u8, "U8";
    U16,  U16  => MatrixU16,  u16, "U16";
    U32,  U32  => MatrixU32,  u32, "U32";
    U64,  U64  => MatrixU64,  u64, "U64";
    U128, U128 => MatrixU128, u128, "U128";
    F32,  F32  => MatrixF32,  F32, "F32";
    F64,  F64  => MatrixF64,  F64, "F64";
  )
}

impl_mech_binop_fxn!(MatrixMatMul,impl_matmul_fxn);
use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// MatMul ---------------------------------------------------------------------

macro_rules! mul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs * *$rhs; }
  };}

macro_rules! matmul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { (*$lhs).mul_to(&*$rhs,&mut *$out); }
  };}

macro_rules! impl_matmul {
  ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
    impl_binop!($name, $type1, $type2, $out_type, matmul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
    register_fxn_descriptor!($name, u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", F32, "f32", F64, "f64");
  };
}

impl_binop!(MatMulScalar, T,T,T,mul_op, FeatureFlag::Builtin(FeatureKind::MatMul));
register_fxn_descriptor!(MatMulScalar, u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", F32, "f32", F64, "f64");

#[cfg(all(feature = "row_vector4", feature = "vector4", feature = "matrix1"))]
impl_matmul!(MatMulR4V4, RowVector4<T>, Vector4<T>, Matrix1<T>);
#[cfg(all(feature = "row_vector4", feature = "matrix4"))]
impl_matmul!(MatMulR4M4, RowVector4<T>, Matrix4<T>, RowVector4<T>);
#[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR4MD, RowVector4<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "row_vector3", feature = "vector3", feature = "matrix1"))]
impl_matmul!(MatMulR3V3, RowVector3<T>, Vector3<T>, Matrix1<T>);
#[cfg(all(feature = "row_vector3", feature = "matrix3"))]
impl_matmul!(MatMulR3M3, RowVector3<T>, Matrix3<T>, RowVector3<T>);
#[cfg(all(feature = "row_vector3", feature = "matrix3x2"))]
impl_matmul!(MatMulR3M3x2, RowVector3<T>, Matrix3x2<T>, RowVector2<T>);
#[cfg(all(feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR3MD, RowVector3<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
impl_matmul!(MatMulR2V2, RowVector2<T>, Vector2<T>, Matrix1<T>);
#[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "row_vector2"))]
impl_matmul!(MatMulR2M2, RowVector2<T>, Matrix2<T>, RowVector2<T>);
#[cfg(all(feature = "row_vector2", feature = "matrix2x3", feature = "row_vector3"))]
impl_matmul!(MatMulR2M2x3, RowVector2<T>, Matrix2x3<T>, RowVector3<T>);
#[cfg(all(feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR2MD, RowVector2<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "row_vectord", feature = "vectord", feature = "matrix1"))]
impl_matmul!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>);
#[cfg(all(feature = "row_vectord", feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
impl_matmul!(MatMulRDVD, RowDVector<T>, DVector<T>, DMatrix<T>);
#[cfg(all(feature = "row_vectord", feature = "matrixd"))]
impl_matmul!(MatMulRDMD, RowDVector<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
impl_matmul!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>);
#[cfg(all(feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
impl_matmul!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>);
#[cfg(all(feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
impl_matmul!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>);

#[cfg(all(feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
impl_matmul!(MatMulVDRD, DVector<T>,RowDVector<T>,DMatrix<T>);

#[cfg(all(feature = "matrix4", feature = "vector4"))]
impl_matmul!(MatMulM4V4, Matrix4<T>, Vector4<T>, Vector4<T>);
#[cfg(all(feature = "matrix4"))]
impl_matmul!(MatMulM4M4, Matrix4<T>, Matrix4<T>, Matrix4<T>);
#[cfg(all(feature = "matrix4", feature = "matrixd"))]
impl_matmul!(MatMulM4MD, Matrix4<T>, DMatrix<T>, DMatrix<T>);

#[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
impl_matmul!(MatMulM2M2x3, Matrix2<T>, Matrix2x3<T>, Matrix2x3<T>);
#[cfg(all(feature = "matrix2", feature = "matrix2"))]
impl_matmul!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>);
#[cfg(all(feature = "matrix2", feature = "vector2"))]
impl_matmul!(MatMulM2V2, Matrix2<T>, Vector2<T>, Vector2<T>);
#[cfg(all(feature = "matrix2", feature = "matrixd"))]
impl_matmul!(MatMulM2MD, Matrix2<T>, DMatrix<T>, DMatrix<T>);

#[cfg(feature = "matrix3")]
impl_matmul!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>);
#[cfg(all(feature = "matrix3", feature = "matrix3x2"))]
impl_matmul!(MatMulM2M3x2, Matrix3<T>, Matrix3x2<T>, Matrix3x2<T>);
#[cfg(all(feature = "matrix3", feature = "vector3"))]
impl_matmul!(MatMulM3V3, Matrix3<T>, Vector3<T>, Vector3<T>);
#[cfg(all(feature = "matrix3", feature = "matrixd"))]
impl_matmul!(MatMulM3MD, Matrix3<T>, DMatrix<T>, DMatrix<T>);

#[cfg(all(feature = "matrix1"))]
impl_matmul!(MatMulM1M1, Matrix1<T>, Matrix1<T>, Matrix1<T>);

#[cfg(all(feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
impl_matmul!(MatMulM2x3V2, Matrix2x3<T>, Vector3<T>, Vector2<T>);
#[cfg(all(feature = "matrix2x3", feature = "matrix3"))]
impl_matmul!(MatMulM2x3M3, Matrix2x3<T>, Matrix3<T>, Matrix2x3<T>);
#[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = "matrix2"))]
impl_matmul!(MatMulM2x3M3x2, Matrix2x3<T>, Matrix3x2<T>, Matrix2<T>);
#[cfg(all(feature = "matrix2x3", feature = "matrixd"))]
impl_matmul!(MatMulM2x3MD, Matrix2x3<T>, DMatrix<T>, DMatrix<T>);

#[cfg(all(feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
impl_matmul!(MatMulM3x2V2, Matrix3x2<T>, Vector2<T>, Vector3<T>);
#[cfg(all(feature = "matrix3x2", feature = "matrix2"))]
impl_matmul!(MatMulM3x2M2, Matrix3x2<T>, Matrix2<T>, Matrix3x2<T>);
#[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = "matrix3"))]
impl_matmul!(MatMulM3x2M2x3, Matrix3x2<T>, Matrix2x3<T>, Matrix3<T>);
#[cfg(all(feature = "matrix3x2", feature = "matrixd"))]
impl_matmul!(MatMulM3x2MD, Matrix3x2<T>, DMatrix<T>, DMatrix<T>);

#[cfg(feature = "matrixd")]
impl_matmul!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>);
#[cfg(all(feature = "matrixd", feature = "matrix3x2"))]
impl_matmul!(MatMulMDM3x2, DMatrix<T>,Matrix3x2<T>,DMatrix<T>);
#[cfg(all(feature = "matrixd", feature = "vectord"))]
impl_matmul!(MatMulMDVD, DMatrix<T>,DVector<T>,DVector<T>);
#[cfg(all(feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulMDRD, DMatrix<T>,RowDVector<T>,DMatrix<T>);

macro_rules! impl_matmul_match_arms {
  ($arg:expr, $($lhs_type:tt, $($matrix_kind:tt, $target_type:tt, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          // Scalar multiplication
          #[cfg(feature = $value_string)]
          (Value::$lhs_type(lhs), Value::$lhs_type(rhs)) => Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::zero()) })),

          // Row Vector 4
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "vector4"))]
          (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new(MatMulR4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
          (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new(MatMulR4M4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulR4MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(),$target_type::zero())) })),

          // Row Vector 3
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "vector3", feature = "matrix1"))]
          (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulR3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3", feature = "row_vector3"))]
          (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulR3M3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3x2", feature = "row_vector2"))]
          (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulR3M3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
          (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulR3MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(), $target_type::zero())) })),

          // Row Vector 2
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
          (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulR2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2", feature = "row_vector2"))]
          (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulR2M2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2x3", feature = "row_vector3"))]
          (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new(MatMulR2M2x3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
          (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulR2MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(), $target_type::zero())) })),

          // Row Vector D
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord", feature = "matrix1"))]
          (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new(MatMulRDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
          (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new(MatMulRDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(1,1,$target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulRDMD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(), $target_type::zero())) })),

          // Vector 4
          #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
          (Value::$matrix_kind(Matrix::Vector4(lhs)), Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new(MatMulV4R4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
          (Value::$matrix_kind(Matrix::Vector3(lhs)), Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new(MatMulV3R3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
          (Value::$matrix_kind(Matrix::Vector2(lhs)), Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new(MatMulV2R2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2::from_element($target_type::zero())) })),

          // Vector D
          #[cfg(all(feature = $value_string, feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::DVector(lhs)), Value::$matrix_kind(Matrix::RowDVector(rhs))) => Ok(Box::new(MatMulVDRD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 4
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
          (Value::$matrix_kind(Matrix::Matrix4(lhs)), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new(MatMulM4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::$matrix_kind(Matrix::Matrix4(lhs)), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new(MatMulM4M4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::Matrix4(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM4MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 2
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrix2x3"))]
          (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new(MatMulM2M2x3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2x3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulM2M2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
          (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulM2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM2MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 3
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::$matrix_kind(Matrix::Matrix3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulM3M3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrix3x2"))]
          (Value::$matrix_kind(Matrix::Matrix3(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulM2M3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3x2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
          (Value::$matrix_kind(Matrix::Matrix3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulM3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::Matrix3(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM3MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 1
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::$matrix_kind(Matrix::Matrix1(lhs)), Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new(MatMulM1M1 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),

          // Matrix 2x3
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3"))]
          (Value::$matrix_kind(Matrix::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulM2x3V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrix3"))]
          (Value::$matrix_kind(Matrix::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulM2x3M3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2x3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrix3x2"))]
          (Value::$matrix_kind(Matrix::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulM2x3M3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM2x3MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 3x2
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2"))]
          (Value::$matrix_kind(Matrix::Matrix3x2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulM3x2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrix2"))]
          (Value::$matrix_kind(Matrix::Matrix3x2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulM3x2M2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3x2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrix2x3"))]
          (Value::$matrix_kind(Matrix::Matrix3x2(lhs)), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new(MatMulM3x2M2x3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::Matrix3x2(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM3x2MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix D
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(
                MechError2::new(
                  DimensionMismatch { dims: vec![lhs_rows, lhs_cols, rhs_rows, rhs_cols] },
                  None
                ).with_compiler_loc()
              );
            }
            Ok(Box::new(MatMulMDMD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs_rows, rhs_cols, $target_type::zero())) }))
          },
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
          (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(MechError2::new(
                DimensionMismatch { dims: vec![lhs_rows, lhs_cols, rhs_rows, rhs_cols] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(MatMulMDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DVector::from_element(lhs_rows, $target_type::zero())) }))
          },
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "row_vectord"))]
          (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::RowDVector(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(MechError2::new(
                DimensionMismatch { dims: vec![lhs_rows, rhs_cols, lhs_cols, rhs_rows] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(MatMulMDRD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs_rows, rhs_cols, $target_type::zero())) }))
          },
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix3x2"))]
          (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(MechError2::new(
                DimensionMismatch { dims: vec![lhs_rows, rhs_cols, lhs_cols, rhs_rows] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(MatMulMDM3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs_rows, rhs_cols, $target_type::zero())) }))
          },
          #[cfg(feature = $value_string)]
          (Value::$matrix_kind(lhs), Value::$matrix_kind(rhs)) => {
            let lhs_shape = lhs.shape();
            let rhs_shape = rhs.shape();
            return Err(MechError2::new(
              DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rhs_shape[0], rhs_shape[1]] },
              None
            ).with_compiler_loc());
          }
        )+
      )+
      (arg1,arg2) => Err(MechError2::new(
        UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: stringify!($fxn).to_string() },
        None
      ).with_compiler_loc()),
    }
  }
}

fn impl_matmul_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_matmul_match_arms!(
    (lhs_value, rhs_value),
    I8,   MatrixI8,   i8,   "i8";
    I16,  MatrixI16,  i16,  "i16";
    I32,  MatrixI32,  i32,  "i32";
    I64,  MatrixI64,  i64,  "i64";
    I128, MatrixI128, i128, "i128";
    U8,   MatrixU8,   u8,   "u8";
    U16,  MatrixU16,  u16,  "u16";
    U32,  MatrixU32,  u32,  "u32";
    U64,  MatrixU64,  u64,  "u64";
    U128, MatrixU128, u128, "u128";
    F32,  MatrixF32,  F32,  "f32";
    F64,  MatrixF64,  F64,  "f64";
    R64, MatrixR64, R64, "rational";
    C64, MatrixC64, C64, "complex";
  )
}

impl_mech_binop_fxn!(MatrixMatMul,impl_matmul_fxn,"matrix/matmul");
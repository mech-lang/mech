use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;

// MatMul ---------------------------------------------------------------------

macro_rules! checked_mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_matrix_mul(*$lhs, *$rhs, "scalar matrix product")?;
            *$out = next;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! checked_matmul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let lhs = &*$lhs;
            let rhs = &*$rhs;
            let current = &*$out;
            if lhs.ncols() != rhs.nrows()
                || current.nrows() != lhs.nrows()
                || current.ncols() != rhs.ncols()
            {
                return Err(MechError::new(
                    DimensionMismatch {
                        dims: vec![
                            lhs.nrows(),
                            lhs.ncols(),
                            rhs.nrows(),
                            rhs.ncols(),
                            current.nrows(),
                            current.ncols(),
                        ],
                    },
                    None,
                )
                .with_compiler_loc());
            }

            let mut next = current.clone();
            for row in 0..lhs.nrows() {
                for column in 0..rhs.ncols() {
                    let mut sum = Zero::zero();
                    for inner in 0..lhs.ncols() {
                        let product = checked_matrix_mul(
                            lhs[(row, inner)],
                            rhs[(inner, column)],
                            "matrix-product multiplication",
                        )?;
                        sum = checked_matrix_add(sum, product, "matrix-product accumulation")?;
                    }
                    next[(row, column)] = sum;
                }
            }
            *$out = next;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_matmul {
    ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
        impl_checked_matrix_binop!(
            $name,
            $type1,
            $type2,
            $out_type,
            checked_matmul_op,
            crate::product_contract
        );
    };
}

impl_checked_matrix_binop!(
    MatMulScalar,
    T,
    T,
    T,
    checked_mul_op,
    crate::product_contract
);
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
#[cfg(all(
    feature = "row_vector3",
    feature = "matrix3x2",
    feature = "row_vector2"
))]
impl_matmul!(MatMulR3M3x2, RowVector3<T>, Matrix3x2<T>, RowVector2<T>);
#[cfg(all(feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR3MD, RowVector3<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
impl_matmul!(MatMulR2V2, RowVector2<T>, Vector2<T>, Matrix1<T>);
#[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "row_vector2"))]
impl_matmul!(MatMulR2M2, RowVector2<T>, Matrix2<T>, RowVector2<T>);
#[cfg(all(
    feature = "row_vector2",
    feature = "matrix2x3",
    feature = "row_vector3"
))]
impl_matmul!(MatMulR2M2x3, RowVector2<T>, Matrix2x3<T>, RowVector3<T>);
#[cfg(all(feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR2MD, RowVector2<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(
    feature = "row_vectord",
    feature = "vectord",
    feature = "matrix1"
))]
impl_matmul!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>);
#[cfg(all(
    feature = "row_vectord",
    feature = "vectord",
    feature = "matrixd",
    not(feature = "matrix1")
))]
impl_matmul!(MatMulRDVDMD, RowDVector<T>, DVector<T>, DMatrix<T>);
#[cfg(all(feature = "row_vectord", feature = "matrixd"))]
impl_matmul!(MatMulRDMD, RowDVector<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
impl_matmul!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>);
#[cfg(all(feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
impl_matmul!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>);
#[cfg(all(feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
impl_matmul!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>);

#[cfg(all(feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
impl_matmul!(MatMulVDRD, DVector<T>, RowDVector<T>, DMatrix<T>);

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
impl_matmul!(MatMulMDMD, DMatrix<T>, DMatrix<T>, DMatrix<T>);
#[cfg(all(feature = "matrixd", feature = "matrix3x2"))]
impl_matmul!(MatMulMDM3x2, DMatrix<T>, Matrix3x2<T>, DMatrix<T>);
#[cfg(all(feature = "matrixd", feature = "vectord"))]
impl_matmul!(MatMulMDVD, DMatrix<T>, DVector<T>, DVector<T>);
#[cfg(all(feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulMDRD, DMatrix<T>, RowDVector<T>, DMatrix<T>);

#[cfg(feature = "source")]
macro_rules! impl_matmul_match_arms {
  ($arg:expr, $($lhs_type:tt, $($matrix_kind:tt, $target_type:tt, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          // Scalar multiplication
          #[cfg(feature = $value_string)]
          (LegacyValue::$lhs_type(lhs), LegacyValue::$lhs_type(rhs)) => Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::zero()) })),

          // Row Vector 4
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "vector4"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector4(lhs)), LegacyValue::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new(MatMulR4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector4(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new(MatMulR4M4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector4(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulR4MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(),$target_type::zero())) })),

          // Row Vector 3
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "vector3", feature = "matrix1"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector3(lhs)), LegacyValue::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulR3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3", feature = "row_vector3"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector3(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulR3M3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3x2", feature = "row_vector2"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector3(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulR3M3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector3(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulR3MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(), $target_type::zero())) })),

          // Row Vector 2
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector2(lhs)), LegacyValue::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulR2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2", feature = "row_vector2"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector2(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulR2M2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2x3", feature = "row_vector3"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector2(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new(MatMulR2M2x3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
          (LegacyValue::$matrix_kind(Matrix::RowVector2(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulR2MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(), $target_type::zero())) })),

          // Row Vector D
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord", feature = "matrix1"))]
          (LegacyValue::$matrix_kind(Matrix::RowDVector(lhs)), LegacyValue::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new(MatMulRDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
          (LegacyValue::$matrix_kind(Matrix::RowDVector(lhs)), LegacyValue::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new(MatMulRDVDMD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(1, 1, $target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::RowDVector(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulRDMD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().ncols(), $target_type::zero())) })),

          // Vector 4
          #[cfg(all(feature = $value_string, feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
          (LegacyValue::$matrix_kind(Matrix::Vector4(lhs)), LegacyValue::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new(MatMulV4R4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
          (LegacyValue::$matrix_kind(Matrix::Vector3(lhs)), LegacyValue::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new(MatMulV3R3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
          (LegacyValue::$matrix_kind(Matrix::Vector2(lhs)), LegacyValue::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new(MatMulV2R2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2::from_element($target_type::zero())) })),

          // Vector D
          #[cfg(all(feature = $value_string, feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::DVector(lhs)), LegacyValue::$matrix_kind(Matrix::RowDVector(rhs))) => Ok(Box::new(MatMulVDRD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 4
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix4(lhs)), LegacyValue::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new(MatMulM4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix4(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new(MatMulM4M4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix4(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM4MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 2
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrix2x3"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new(MatMulM2M2x3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2x3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulM2M2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2(lhs)), LegacyValue::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulM2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM2MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 3
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulM3M3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrix3x2"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulM2M3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3x2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3(lhs)), LegacyValue::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulM3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM3MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 1
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix1(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new(MatMulM1M1 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix1::from_element($target_type::zero())) })),

          // Matrix 2x3
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector3"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2x3(lhs)), LegacyValue::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulM2x3V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrix3"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2x3(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulM2x3M3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2x3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrix3x2"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2x3(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulM2x3M3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2x3(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM2x3MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix 3x2
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector2"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3x2(lhs)), LegacyValue::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulM3x2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrix2"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3x2(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulM3x2M2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3x2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrix2x3"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3x2(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new(MatMulM3x2M2x3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Matrix3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix3x2(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => Ok(Box::new(MatMulM3x2MD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs.borrow().nrows(), rhs.borrow().ncols(), $target_type::zero())) })),

          // Matrix D
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::DMatrix(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(
                MechError::new(
                  DimensionMismatch { dims: vec![lhs_rows, lhs_cols, rhs_rows, rhs_cols] },
                  None
                ).with_compiler_loc()
              );
            }
            Ok(Box::new(MatMulMDMD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs_rows, rhs_cols, $target_type::zero())) }))
          },
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
          (LegacyValue::$matrix_kind(Matrix::DMatrix(lhs)), LegacyValue::$matrix_kind(Matrix::DVector(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![lhs_rows, lhs_cols, rhs_rows, rhs_cols] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(MatMulMDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DVector::from_element(lhs_rows, $target_type::zero())) }))
          },
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "row_vectord"))]
          (LegacyValue::$matrix_kind(Matrix::DMatrix(lhs)), LegacyValue::$matrix_kind(Matrix::RowDVector(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![lhs_rows, rhs_cols, lhs_cols, rhs_rows] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(MatMulMDRD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs_rows, rhs_cols, $target_type::zero())) }))
          },
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix3x2"))]
          (LegacyValue::$matrix_kind(Matrix::DMatrix(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix3x2(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_cols != rhs_rows {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![lhs_rows, rhs_cols, lhs_cols, rhs_rows] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(MatMulMDM3x2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DMatrix::from_element(lhs_rows, rhs_cols, $target_type::zero())) }))
          },
          #[cfg(all(feature = $value_string, feature = "matrix"))]
          (LegacyValue::$matrix_kind(lhs), LegacyValue::$matrix_kind(rhs)) => {
            let lhs_shape = lhs.shape();
            let rhs_shape = rhs.shape();
            return Err(MechError::new(
              DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rhs_shape[0], rhs_shape[1]] },
              None
            ).with_compiler_loc());
          }
        )+
      )+
      (arg1,arg2) => Err(MechError::new(
        UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: stringify!($fxn).to_string() },
        None
      ).with_compiler_loc()),
    }
  }
}

#[cfg(feature = "source")]
fn impl_matmul_fxn(lhs_value: LegacyValue, rhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
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
      F32,  MatrixF32,  f32,  "f32";
      F64,  MatrixF64,  f64,  "f64";
      R64, MatrixR64, R64, "rational";
      C64, MatrixC64, C64, "complex";
    )
}

#[cfg(feature = "source")]
impl_mech_binop_fxn!(MatrixMatMul, impl_matmul_fxn, "matrix/matmul");

#[cfg(all(test, feature = "u8", feature = "matrixd"))]
mod checked_matmul_tests {
    use super::*;

    #[test]
    fn integer_matrix_product_rejects_overflow_and_retains_output() {
        let lhs = Ref::new(DMatrix::from_row_slice(1, 2, &[200_u8, 200]));
        let rhs = Ref::new(DMatrix::from_column_slice(2, 1, &[1_u8, 0]));
        let out = Ref::new(DMatrix::from_element(1, 1, 17_u8));
        let function = MatMulMDMD {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(out.borrow()[(0, 0)], 200);
        *rhs.borrow_mut() = DMatrix::from_column_slice(2, 1, &[2, 2]);

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MatrixArithmeticOverflow");
        assert_eq!(out.borrow()[(0, 0)], 200);
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "matrix2",
    feature = "vector2",
    feature = "matrixd",
    feature = "vectord"
))]
mod invocation_port_tests {
    use super::*;

    fn binary_args<L, R, O>(out: &Ref<O>, lhs: &Ref<L>, rhs: &Ref<R>) -> FunctionArgs
    where
        Ref<L>: ToValue,
        Ref<R>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
    }

    #[test]
    fn scalar_fixed_and_dynamic_products_use_exact_invocation_ports() {
        let scalar_lhs = Ref::new(2.5_f64);
        let scalar_rhs = Ref::new(4.0_f64);
        let legacy_out = Ref::new(0.0_f64);
        let invocation_out = Ref::new(0.0_f64);
        let legacy =
            MatMulScalar::<f64>::new(binary_args(&legacy_out, &scalar_lhs, &scalar_rhs)).unwrap();
        let invocation = MatMulScalar::<f64>::new_invocation(
            binary_args(&invocation_out, &scalar_lhs, &scalar_rhs).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), 10.0);
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());

        let fixed_lhs = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let fixed_rhs = Ref::new(Matrix2::new(5.0_f64, 6.0, 7.0, 8.0));
        let fixed_out = Ref::new(Matrix2::zeros());
        MatMulM2M2::<f64>::new_invocation(binary_args(&fixed_out, &fixed_lhs, &fixed_rhs).into())
            .unwrap()
            .solve_result()
            .unwrap();
        assert_eq!(*fixed_out.borrow(), Matrix2::new(19.0, 22.0, 43.0, 50.0));

        let vector = Ref::new(Vector2::new(2.0_f64, 3.0));
        let vector_out = Ref::new(Vector2::zeros());
        MatMulM2V2::<f64>::new_invocation(binary_args(&vector_out, &fixed_lhs, &vector).into())
            .unwrap()
            .solve_result()
            .unwrap();
        assert_eq!(*vector_out.borrow(), Vector2::new(8.0, 18.0));

        let dynamic_lhs = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_rhs = Ref::new(DMatrix::from_row_slice(
            3,
            2,
            &[7.0_f64, 8.0, 9.0, 10.0, 11.0, 12.0],
        ));
        let dynamic_out = Ref::new(DMatrix::zeros(2, 2));
        MatMulMDMD::<f64>::new_invocation(
            binary_args(&dynamic_out, &dynamic_lhs, &dynamic_rhs).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            DMatrix::from_row_slice(2, 2, &[58.0, 64.0, 139.0, 154.0])
        );

        let dynamic_vector = Ref::new(DVector::from_vec(vec![1.0_f64, 2.0, 3.0]));
        let dynamic_vector_out = Ref::new(DVector::zeros(2));
        MatMulMDVD::<f64>::new_invocation(
            binary_args(&dynamic_vector_out, &dynamic_lhs, &dynamic_vector).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *dynamic_vector_out.borrow(),
            DVector::from_vec(vec![14.0, 32.0])
        );
    }

    #[test]
    fn dimension_failure_does_not_publish_a_partial_product() {
        let lhs = Ref::new(DMatrix::from_element(2, 3, 2.0_f64));
        let rhs = Ref::new(DMatrix::from_element(2, 2, 3.0_f64));
        let original = DMatrix::from_element(2, 2, 17.0_f64);
        let out = Ref::new(original.clone());
        let function =
            MatMulMDMD::<f64>::new_invocation(binary_args(&out, &lhs, &rhs).into()).unwrap();

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "DimensionMismatch");
        assert_eq!(*out.borrow(), original);
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "row_vectord",
    feature = "vectord",
    feature = "matrixd",
    not(feature = "matrix1")
))]
mod dynamic_fallback_invocation_tests {
    use super::*;

    #[test]
    fn row_vector_product_uses_dynamic_matrix_fallback_without_matrix1() {
        let lhs = Ref::new(RowDVector::from_vec(vec![1.0_f64, 2.0]));
        let rhs = Ref::new(DVector::from_vec(vec![3.0_f64, 4.0]));
        let out = Ref::new(DMatrix::zeros(1, 1));
        let function = MatMulRDVDMD::<f64>::new_invocation(
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), DMatrix::from_element(1, 1, 11.0));
    }
}

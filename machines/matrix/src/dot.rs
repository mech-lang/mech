use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;

// MatMul ---------------------------------------------------------------------

macro_rules! checked_mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_matrix_mul(*$lhs, *$rhs, "scalar product")?;
            *$out = next;
        }
    };
}

macro_rules! checked_dot_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let lhs = &*$lhs;
            let rhs = &*$rhs;
            if lhs.nrows() != rhs.nrows() || lhs.ncols() != rhs.ncols() {
                return Err(MechError::new(
                    DimensionMismatch {
                        dims: vec![lhs.nrows(), lhs.ncols(), rhs.nrows(), rhs.ncols()],
                    },
                    None,
                )
                .with_compiler_loc());
            }
            let mut next = Zero::zero();
            for (lhs, rhs) in lhs.iter().zip(rhs.iter()) {
                let product = checked_matrix_mul(*lhs, *rhs, "dot-product multiplication")?;
                next = checked_matrix_add(next, product, "dot-product accumulation")?;
            }
            *$out = next;
        }
    };
}

macro_rules! impl_dot {
    ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
        impl_checked_matrix_binop!($name, $type1, $type2, $out_type, checked_dot_op);
    };
}

impl_checked_matrix_binop!(DotScalar, T, T, T, checked_mul_op);
#[cfg(all(feature = "row_vector2", feature = "row_vector2"))]
impl_dot!(DotR2R2, RowVector2<T>, RowVector2<T>, T);
#[cfg(all(feature = "vector2", feature = "vector2"))]
impl_dot!(DotV2V2, Vector2<T>, Vector2<T>, T);

#[cfg(all(feature = "row_vector3", feature = "row_vector3"))]
impl_dot!(DotR3R3, RowVector3<T>, RowVector3<T>, T);
#[cfg(all(feature = "vector3", feature = "vector3"))]
impl_dot!(DotV3V3, Vector3<T>, Vector3<T>, T);

#[cfg(all(feature = "row_vector4", feature = "row_vector4"))]
impl_dot!(DotR4R4, RowVector4<T>, RowVector4<T>, T);
#[cfg(all(feature = "vector4", feature = "vector4"))]
impl_dot!(DotV4V4, Vector4<T>, Vector4<T>, T);

#[cfg(all(feature = "matrix1", feature = "matrix1"))]
impl_dot!(DotM1M1, Matrix1<T>, Matrix1<T>, T);
#[cfg(all(feature = "matrix2", feature = "matrix2"))]
impl_dot!(DotM2M2, Matrix2<T>, Matrix2<T>, T);
#[cfg(all(feature = "matrix3", feature = "matrix3"))]
impl_dot!(DotM3M3, Matrix3<T>, Matrix3<T>, T);
#[cfg(all(feature = "matrix4", feature = "matrix4"))]
impl_dot!(DotM4M4, Matrix4<T>, Matrix4<T>, T);

#[cfg(all(feature = "matrixd", feature = "matrixd"))]
impl_dot!(DotMDMD, DMatrix<T>, DMatrix<T>, T);
#[cfg(all(feature = "vectord", feature = "vectord"))]
impl_dot!(DotVDVD, DVector<T>, DVector<T>, T);
#[cfg(all(feature = "row_vectord", feature = "row_vectord"))]
impl_dot!(DotRDRD, RowDVector<T>, RowDVector<T>, T);

#[cfg(feature = "source")]
macro_rules! impl_dot_match_arms {
  ($arg:expr, $($lhs_type:tt, $($matrix_kind:tt, $target_type:tt, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          #[cfg(feature = $value_string)]
          (Value::$lhs_type(lhs), Value::$lhs_type(rhs)) => Ok(Box::new(DotScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),

          #[cfg(all(feature = $value_string, feature = "vector2", feature = "vector2"))]
          (Value::$matrix_kind(Matrix::Vector2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(DotV2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "row_vector2"))]
          (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new(DotR2R2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),

          #[cfg(all(feature = $value_string, feature = "vector3", feature = "vector3"))]
          (Value::$matrix_kind(Matrix::Vector3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(DotV3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "row_vector3"))]
          (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new(DotR3R3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),

          #[cfg(all(feature = $value_string, feature = "vector4", feature = "vector4"))]
          (Value::$matrix_kind(Matrix::Vector4(lhs)), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new(DotV4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "row_vector4"))]
          (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new(DotR4R4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),

          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrix1"))]
          (Value::$matrix_kind(Matrix::Matrix1(lhs)), Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new(DotM1M1 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrix2"))]
          (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(DotM2M2 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrix3"))]
          (Value::$matrix_kind(Matrix::Matrix3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(DotM3M3 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "matrix4"))]
          (Value::$matrix_kind(Matrix::Matrix4(lhs)), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new(DotM4M4 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrixd"))]
          (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
            let (lhs_rows,lhs_cols) = {lhs.borrow().shape()};
            let (rhs_rows,rhs_cols) = {rhs.borrow().shape()};
            if lhs_rows != rhs_rows || lhs_cols != rhs_cols {
              return Err(
                MechError::new(
                  DimensionMismatch { dims: vec![lhs_rows, lhs_cols, rhs_rows, rhs_cols] },
                  None
                ).with_compiler_loc()
              );
            }
            Ok(Box::new(DotMDMD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) }))
          },
          #[cfg(all(feature = $value_string, feature = "vectord", feature = "vectord"))]
          (Value::$matrix_kind(Matrix::DVector(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => {
            let lhs_len = {lhs.borrow().len()};
            let rhs_len = {rhs.borrow().len()};
            if lhs_len != rhs_len {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![lhs_len, rhs_len] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(DotVDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) }))
          },
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "row_vectord"))]
          (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::RowDVector(rhs))) => {
            let lhs_len = {lhs.borrow().len()};
            let rhs_len = {rhs.borrow().len()};
            if lhs_len != rhs_len {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![lhs_len, rhs_len] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(DotRDRD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) }))
          },
        )+
      )+
      (arg1,arg2) => Err(MechError::new(
          UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: stringify!($fxn).to_string() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}

#[cfg(feature = "source")]
fn impl_dot_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
    impl_dot_match_arms!(
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
impl_mech_binop_fxn!(MatrixDot, impl_dot_fxn, "matrix/dot");

#[cfg(all(test, feature = "u8", feature = "vector2"))]
mod checked_dot_tests {
    use super::*;

    #[test]
    fn integer_dot_rejects_overflow_and_retains_output() {
        let lhs = Ref::new(Vector2::new(200_u8, 200));
        let rhs = Ref::new(Vector2::new(1_u8, 0));
        let out = Ref::new(17_u8);
        let function = DotV2V2 {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 200);
        *rhs.borrow_mut() = Vector2::new(2, 2);

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MatrixArithmeticOverflow");
        assert_eq!(*out.borrow(), 200);
    }
}

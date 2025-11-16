use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// MatMul ---------------------------------------------------------------------

macro_rules! mul_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = *$lhs * *$rhs; }
  };}

macro_rules! dot_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { *$out = (*$lhs).dot(&*$rhs); }
  };}

macro_rules! impl_dot {
  ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
    impl_binop!($name, $type1, $type2, $out_type, dot_op, FeatureFlag::Builtin(FeatureKind::Dot));
    register_fxn_descriptor!($name, u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", F32, "f32", F64, "f64");
  };
}

impl_binop!(DotScalar, T, T, T, mul_op, FeatureFlag::Builtin(FeatureKind::Dot));
register_fxn_descriptor!(DotScalar, u8, "u8", u16, "u16", u32, "u32", u64, "u64", u128, "u128", i8, "i8", i16, "i16", i32, "i32", i64, "i64", i128, "i128", F32, "f32", F64, "f64");

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
impl_dot!(DotM1M1, Matrix2<T>, Matrix2<T>, T);
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
          (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(DotM1M1 { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
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
                MechError2::new(
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
              return Err(MechError2::new(
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
              return Err(MechError2::new(
                DimensionMismatch { dims: vec![lhs_len, rhs_len] },
                None
              ).with_compiler_loc());
            }
            Ok(Box::new(DotRDRD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) }))
          },
        )+
      )+
      x => Err(MechError2::new(
          UnhandledFunctionArgumentKind2 { arg: x, fxn_name: stringify!($fxn).to_string() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}

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
    F32,  MatrixF32,  F32,  "f32";
    F64,  MatrixF64,  F64,  "f64";
    R64, MatrixR64, R64, "rational";
    C64, MatrixC64, C64, "complex";
  )
}

impl_mech_binop_fxn!(MatrixDot,impl_dot_fxn,"matrix/dot");
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

impl_binop!(MatMulScalar, T,T,T,mul_op);
#[cfg(feature = "RowVector4")]
impl_binop!(MatMulR4V4, RowVector4<T>,Vector4<T>,Matrix1<T>,matmul_op);
#[cfg(feature = "RowVector3")]
impl_binop!(MatMulR3V3, RowVector3<T>,Vector3<T>,Matrix1<T>,matmul_op);
#[cfg(feature = "RowVector2")]
impl_binop!(MatMulR2V2, RowVector2<T>,Vector2<T>,Matrix1<T>,matmul_op);
#[cfg(feature = "RowVectorD")]
impl_binop!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>,matmul_op);
#[cfg(feature = "Vector4")]
impl_binop!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>,matmul_op);
#[cfg(feature = "Vector3")]
impl_binop!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>,matmul_op);
#[cfg(feature = "Vector2")]
impl_binop!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>,matmul_op);
#[cfg(feature = "RowVectorD")]
impl_binop!(MatMulVDRD, DVector<T>,RowDVector<T>,DMatrix<T>,matmul_op);
#[cfg(feature = "Matrix4")]
impl_binop!(MatMulM4M4, Matrix4<T>, Matrix4<T>, Matrix4<T>,matmul_op);
#[cfg(feature = "Matrix3")]
impl_binop!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>,matmul_op);
#[cfg(feature = "Matrix2")]
impl_binop!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>,matmul_op);
#[cfg(feature = "Matrix1")]
impl_binop!(MatMulM1M1, Matrix1<T>, Matrix1<T>, Matrix1<T>,matmul_op);
#[cfg(feature = "Matrix2x3")]
impl_binop!(MatMulM2x3M3x2, Matrix2x3<T>, Matrix3x2<T>, Matrix2<T>,matmul_op);
#[cfg(feature = "MatrixD")]
impl_binop!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,matmul_op);

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
#[macro_use]
use crate::stdlib::*;

pub mod access;
pub mod set;

// ----------------------------------------------------------------------------
// Matrix Library
// ----------------------------------------------------------------------------

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
impl_binop!(MatMulM2x3M3x2, Matrix2x3<T>, Matrix3x2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulM1M1, Matrix1<T>, Matrix1<T>, Matrix1<T>,matmul_op);
impl_binop!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>,matmul_op);
impl_binop!(MatMulM4M4, Matrix4<T>, Matrix4<T>, Matrix4<T>,matmul_op);
impl_binop!(MatMulR2V2, RowVector2<T>,Vector2<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulR3V3, RowVector3<T>,Vector3<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulR4V4, RowVector4<T>,Vector4<T>,Matrix1<T>,matmul_op);
impl_binop!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>,matmul_op);
impl_binop!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>,matmul_op);
impl_binop!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>,matmul_op);
impl_binop!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>,matmul_op);
impl_binop!(MatMulVDRD, DVector<T>,RowDVector<T>,DMatrix<T>,matmul_op);
impl_binop!(MatMulMDMD, DMatrix<T>,DMatrix<T>,DMatrix<T>,matmul_op);

macro_rules! impl_matmul_match_arms {
  ($arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          #[cfg(all(feature = $value_string))]
          (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => Ok(Box::new(MatMulScalar { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($target_type::zero()) })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          (Value::$matrix_kind(Matrix::Vector4(lhs)),    Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new(MatMulV4R4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix4::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          (Value::$matrix_kind(Matrix::Vector3(lhs)),    Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new(MatMulV3R3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix3::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          (Value::$matrix_kind(Matrix::Vector2(lhs)),    Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new(MatMulV2R2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix2::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new(MatMulR4V4 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new(MatMulR3V3 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new(MatMulR2V2 { lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Matrix1::from_element($target_type::zero())) })),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          (Value::$matrix_kind(Matrix::Matrix4(lhs)),    Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new(MatMulM4M4{lhs, rhs, out: new_ref(Matrix4::from_element($target_type::zero()))})),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          (Value::$matrix_kind(Matrix::Matrix3(lhs)),    Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new(MatMulM3M3{lhs, rhs, out: new_ref(Matrix3::from_element($target_type::zero()))})),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          (Value::$matrix_kind(Matrix::Matrix2(lhs)),    Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new(MatMulM2M2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))})),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          (Value::$matrix_kind(Matrix::Matrix1(lhs)),    Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new(MatMulM1M1{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))})),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          (Value::$matrix_kind(Matrix::Matrix2x3(lhs)),  Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new(MatMulM2x3M3x2{lhs, rhs, out: new_ref(Matrix2::from_element($target_type::zero()))})),          
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new(MatMulRDVD{lhs, rhs, out: new_ref(Matrix1::from_element($target_type::zero()))})),
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
          (Value::$matrix_kind(Matrix::DVector(lhs)),    Value::$matrix_kind(Matrix::RowDVector(rhs))) => {
            let rows = {lhs.borrow().len()};
            let cols = {rhs.borrow().len()};
            Ok(Box::new(MatMulVDRD{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$target_type::zero()))}))
          },
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
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

// Transpose ------------------------------------------------------------------

macro_rules! transpose_op {
  ($arg:expr, $out:expr) => {
    unsafe { *$out = (*$arg).transpose(); }
  };}

impl_bool_urop!(TransposeM1, Matrix1<T>, Matrix1<T>, transpose_op);
impl_bool_urop!(TransposeM2, Matrix2<T>, Matrix2<T>, transpose_op);
impl_bool_urop!(TransposeM3, Matrix3<T>, Matrix3<T>, transpose_op);
impl_bool_urop!(TransposeM4, Matrix4<T>, Matrix4<T>, transpose_op);
impl_bool_urop!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>, transpose_op);
impl_bool_urop!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>, transpose_op);
impl_bool_urop!(TransposeV2, Vector2<T>, RowVector2<T>, transpose_op);
impl_bool_urop!(TransposeV3, Vector3<T>, RowVector3<T>, transpose_op);
impl_bool_urop!(TransposeV4, Vector4<T>, RowVector4<T>, transpose_op); 
impl_bool_urop!(TransposeR2, RowVector2<T>, Vector2<T>, transpose_op);
impl_bool_urop!(TransposeR3, RowVector3<T>, Vector3<T>, transpose_op);
impl_bool_urop!(TransposeR4, RowVector4<T>, Vector4<T>, transpose_op); 
impl_bool_urop!(TransposeRD, RowDVector<T>, DVector<T>, transpose_op);
impl_bool_urop!(TransposeVD, DVector<T>, RowDVector<T>, transpose_op);
impl_bool_urop!(TransposeMD, DMatrix<T>, DMatrix<T>, transpose_op);

macro_rules! impl_transpose_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          #[cfg(all(feature = $value_string, feature = "RowVector4"))]
          Value::$matrix_kind(Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(TransposeR4{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
          #[cfg(all(feature = $value_string, feature = "RowVector3"))]
          Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(TransposeR3{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
          #[cfg(all(feature = $value_string, feature = "RowVector2"))]
          Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(TransposeR2{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
          #[cfg(all(feature = $value_string, feature = "Vector4"))]
          Value::$matrix_kind(Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(TransposeV4{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) })),
          #[cfg(all(feature = $value_string, feature = "Vector3"))]
          Value::$matrix_kind(Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(TransposeV3{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) })),
          #[cfg(all(feature = $value_string, feature = "Vector2"))]
          Value::$matrix_kind(Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(TransposeV2{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) })),
          #[cfg(all(feature = $value_string, feature = "Matrix4"))]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(TransposeM4{arg: arg.clone(), out: new_ref(Matrix4::from_element($default))})),
          #[cfg(all(feature = $value_string, feature = "Matrix3"))]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(TransposeM3{arg: arg.clone(), out: new_ref(Matrix3::from_element($default))})),
          #[cfg(all(feature = $value_string, feature = "Matrix2"))]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(TransposeM2{arg: arg.clone(), out: new_ref(Matrix2::from_element($default))})),
          #[cfg(all(feature = $value_string, feature = "Matrix1"))]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(TransposeM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($default))})),
          #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(TransposeM2x3{arg: arg.clone(), out: new_ref(Matrix3x2::from_element($default))})),          
          #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(TransposeM3x2{arg: arg.clone(), out: new_ref(Matrix2x3::from_element($default))})),          
          #[cfg(all(feature = $value_string, feature = "VectorD"))]
          Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(TransposeVD{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
          #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
          Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(TransposeRD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
          #[cfg(all(feature = $value_string, feature = "MatrixD"))]
          Value::$matrix_kind(Matrix::<$target_type>::DMatrix(arg)) => {
            let (rows,cols) = {arg.borrow().shape()};
            Ok(Box::new(TransposeMD{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))
          },
        )+
      )+
      x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

fn impl_transpose_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_transpose_match_arms!(
    (lhs_value),
    Bool => MatrixBool, bool, false, "Bool";
    I8   => MatrixI8,   i8,   i8::zero(), "I8";
    I16  => MatrixI16,  i16,  i16::zero(), "I16";
    I32  => MatrixI32,  i32,  i32::zero(), "I32";
    I64  => MatrixI64,  i64,  i64::zero(), "I64";
    I128 => MatrixI128, i128, i128::zero(), "I128";
    U8   => MatrixU8,   u8,   u8::zero(), "U8";
    U16  => MatrixU16,  u16,  u16::zero(), "U16";
    U32  => MatrixU32,  u32,  u32::zero(), "U32";
    U64  => MatrixU64,  u64,  u64::zero(), "U64";
    U128 => MatrixU128, u128, u128::zero(), "U128";
    F32  => MatrixF32,  F32,  F32::zero(), "F32";
    F64  => MatrixF64,  F64,  F64::zero(), "F64";
  )
}
  
impl_mech_urnop_fxn!(MatrixTranspose,impl_transpose_fxn);
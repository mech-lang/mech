use crate::*;
use mech_core::*;

// Transpose ------------------------------------------------------------------

macro_rules! transpose_op {
  ($arg:expr, $out:expr) => {
    unsafe { *$out = (*$arg).transpose(); }
  };}

#[cfg(feature = "Matrix1")]
impl_bool_urop!(TransposeM1, Matrix1<T>, Matrix1<T>, transpose_op);
#[cfg(feature = "Matrix2")]
impl_bool_urop!(TransposeM2, Matrix2<T>, Matrix2<T>, transpose_op);
#[cfg(feature = "Matrix3")]
impl_bool_urop!(TransposeM3, Matrix3<T>, Matrix3<T>, transpose_op);
#[cfg(feature = "Matrix4")]
impl_bool_urop!(TransposeM4, Matrix4<T>, Matrix4<T>, transpose_op);
#[cfg(feature = "Matrix2x3")]
impl_bool_urop!(TransposeM2x3, Matrix2x3<T>, Matrix3x2<T>, transpose_op);
#[cfg(feature = "Matrix3x2")]
impl_bool_urop!(TransposeM3x2, Matrix3x2<T>, Matrix2x3<T>, transpose_op);
#[cfg(feature = "MatrixD")]
impl_bool_urop!(TransposeMD, DMatrix<T>, DMatrix<T>, transpose_op);
#[cfg(feature = "Vector2")]
impl_bool_urop!(TransposeV2, Vector2<T>, RowVector2<T>, transpose_op);
#[cfg(feature = "Vector3")]
impl_bool_urop!(TransposeV3, Vector3<T>, RowVector3<T>, transpose_op);
#[cfg(feature = "Vector4")]
impl_bool_urop!(TransposeV4, Vector4<T>, RowVector4<T>, transpose_op); 
#[cfg(feature = "VectorD")]
impl_bool_urop!(TransposeVD, DVector<T>, RowDVector<T>, transpose_op);
#[cfg(feature = "RowVector2")]
impl_bool_urop!(TransposeR2, RowVector2<T>, Vector2<T>, transpose_op);
#[cfg(feature = "RowVector3")]
impl_bool_urop!(TransposeR3, RowVector3<T>, Vector3<T>, transpose_op);
#[cfg(feature = "RowVector4")]
impl_bool_urop!(TransposeR4, RowVector4<T>, Vector4<T>, transpose_op); 
#[cfg(feature = "RowVectorD")]
impl_bool_urop!(TransposeRD, RowDVector<T>, DVector<T>, transpose_op);

macro_rules! impl_transpose_match_arms {
  ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          #[cfg(feature = "RowVector4")]
          Value::$matrix_kind(Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(TransposeR4{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
          #[cfg(feature = "RowVector3")]
          Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(TransposeR3{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
          #[cfg(feature = "RowVector2")]
          Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(TransposeR2{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
          #[cfg(feature = "Vector4")]
          Value::$matrix_kind(Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(TransposeV4{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) })),
          #[cfg(feature = "Vector3")]
          Value::$matrix_kind(Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(TransposeV3{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) })),
          #[cfg(feature = "Vector2")]
          Value::$matrix_kind(Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(TransposeV2{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) })),
          #[cfg(feature = "Matrix4")]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(TransposeM4{arg: arg.clone(), out: new_ref(Matrix4::from_element($default))})),
          #[cfg(feature = "Matrix3")]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(TransposeM3{arg: arg.clone(), out: new_ref(Matrix3::from_element($default))})),
          #[cfg(feature = "Matrix2")]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(TransposeM2{arg: arg.clone(), out: new_ref(Matrix2::from_element($default))})),
          #[cfg(feature = "Matrix1")]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(TransposeM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($default))})),
          #[cfg(feature = "Matrix2x3")]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(TransposeM2x3{arg: arg.clone(), out: new_ref(Matrix3x2::from_element($default))})),          
          #[cfg(feature = "Matrix3x2")]
          Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(TransposeM3x2{arg: arg.clone(), out: new_ref(Matrix2x3::from_element($default))})),          
          #[cfg(feature = "VectorD")]
          Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(TransposeVD{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
          #[cfg(feature = "RowVectorD")]
          Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(TransposeRD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
          #[cfg(feature = "MatrixD")]
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
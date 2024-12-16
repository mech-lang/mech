use crate::*;
use mech_core::*;

// Stats Sum Column -----------------------------------------------------------

macro_rules! sum_column_op {
    ($arg:expr, $out:expr) => {
      unsafe { 
        *$out = (*$arg).column_sum();
      }
    };}
  
  #[cfg(feature = "Matrix1")]
  impl_stats_urop!(StatsSumColumnM1, Matrix1<T>, Matrix1<T>, sum_column_op);
  #[cfg(feature = "Matrix2")]
  impl_stats_urop!(StatsSumColumnM2, Matrix2<T>, Vector2<T>, sum_column_op);
  #[cfg(feature = "Matrix3")]
  impl_stats_urop!(StatsSumColumnM3, Matrix3<T>, Vector3<T>, sum_column_op);
  #[cfg(feature = "Matrix4")]
  impl_stats_urop!(StatsSumColumnM4, Matrix4<T>, Vector4<T>, sum_column_op);
  #[cfg(feature = "Matrix2x3")]
  impl_stats_urop!(StatsSumColumnM2x3, Matrix2x3<T>, Vector2<T>, sum_column_op);
  #[cfg(feature = "Matrix3x2")]
  impl_stats_urop!(StatsSumColumnM3x2, Matrix3x2<T>, Vector3<T>, sum_column_op);
  #[cfg(feature = "MatrixD")]
  impl_stats_urop!(StatsSumColumnMD, DMatrix<T>, DVector<T>, sum_column_op);
  #[cfg(feature = "Vector2")]
  impl_stats_urop!(StatsSumColumnV2, Vector2<T>, Vector2<T>, sum_column_op);
  #[cfg(feature = "Vector3")]
  impl_stats_urop!(StatsSumColumnV3, Vector3<T>, Vector3<T>, sum_column_op);
  #[cfg(feature = "Vector4")]
  impl_stats_urop!(StatsSumColumnV4, Vector4<T>, Vector4<T>, sum_column_op); 
  #[cfg(feature = "VectorD")]
  impl_stats_urop!(StatsSumColumnVD, DVector<T>, DVector<T>, sum_column_op);
  #[cfg(feature = "Vector2")]
  impl_stats_urop!(StatsSumColumnR2, RowVector2<T>, Matrix1<T>, sum_column_op);
  #[cfg(feature = "Vector3")]
  impl_stats_urop!(StatsSumColumnR3, RowVector3<T>, Matrix1<T>, sum_column_op);
  #[cfg(feature = "Vector4")]
  impl_stats_urop!(StatsSumColumnR4, RowVector4<T>, Matrix1<T>, sum_column_op); 
  #[cfg(feature = "VectorD")]
  impl_stats_urop!(StatsSumColumnRD, RowDVector<T>, Matrix1<T>, sum_column_op);
  
  macro_rules! impl_stats_sum_column_match_arms {
    ($arg:expr, $($input_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
      match $arg {
        $(
          $(
            #[cfg(feature = "Vector4")]
            Value::$matrix_kind(Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(StatsSumColumnR4{arg: arg.clone(), out: new_ref(Matrix1::from_element($default)) })),
            #[cfg(feature = "Vector3")]
            Value::$matrix_kind(Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(StatsSumColumnR3{arg: arg.clone(), out: new_ref(Matrix1::from_element($default)) })),
            #[cfg(feature = "Vector2")]
            Value::$matrix_kind(Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(StatsSumColumnR2{arg: arg.clone(), out: new_ref(Matrix1::from_element($default)) })),
            #[cfg(feature = "Vector4")]
            Value::$matrix_kind(Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(StatsSumColumnV4{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
            #[cfg(feature = "Vector3")]
            Value::$matrix_kind(Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(StatsSumColumnV3{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
            #[cfg(feature = "Vector2")]
            Value::$matrix_kind(Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(StatsSumColumnV2{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
            #[cfg(feature = "Matrix4")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(StatsSumColumnM4{arg: arg.clone(), out: new_ref(Vector4::from_element($default))})),
            #[cfg(feature = "Matrix3")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(StatsSumColumnM3{arg: arg.clone(), out: new_ref(Vector3::from_element($default))})),
            #[cfg(feature = "Matrix2")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(StatsSumColumnM2{arg: arg.clone(), out: new_ref(Vector2::from_element($default))})),
            #[cfg(feature = "Matrix1")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(StatsSumColumnM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($default))})),
            #[cfg(feature = "Matrix2x3")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(StatsSumColumnM2x3{arg: arg.clone(), out: new_ref(Vector2::from_element($default))})),          
            #[cfg(feature = "Matrix3x2")]
            Value::$matrix_kind(Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(StatsSumColumnM3x2{arg: arg.clone(), out: new_ref(Vector3::from_element($default))})),          
            #[cfg(feature = "VectorD")]
            Value::$matrix_kind(Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(StatsSumColumnVD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(feature = "VectorD")]
            Value::$matrix_kind(Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(StatsSumColumnRD{arg: arg.clone(), out: new_ref(Matrix1::from_element($default))})),
            #[cfg(feature = "MatrixD")]
            Value::$matrix_kind(Matrix::<$target_type>::DMatrix(arg)) => Ok(Box::new(StatsSumColumnMD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().nrows(),$default))})),
          )+
        )+
        _ => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
  
  fn impl_stats_sum_column_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    impl_stats_sum_column_match_arms!(
      lhs_value,
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
    
  impl_mech_urnop_fxn!(StatsSumColumn,impl_stats_sum_column_fxn); 
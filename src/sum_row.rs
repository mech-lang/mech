use crate::*;
use mech_core::*;

// Stats Sum Row -----------------------------------------------------------

macro_rules! sum_row_op {
    ($arg:expr, $out:expr) => {
      unsafe { 
        *$out = (*$arg).row_sum();
      }
    };}
  
  #[cfg(feature = "Matrix1")]
  impl_stats_urop!(StatsSumRowM1, Matrix1<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "Matrix2")]
  impl_stats_urop!(StatsSumRowM2, Matrix2<T>, RowVector2<T>, sum_row_op);
  #[cfg(feature = "Matrix3")]
  impl_stats_urop!(StatsSumRowM3, Matrix3<T>, RowVector3<T>, sum_row_op);
  #[cfg(feature = "Matrix4")]
  impl_stats_urop!(StatsSumRowM4, Matrix4<T>, RowVector4<T>, sum_row_op);
  #[cfg(feature = "Matrix2x3")]
  impl_stats_urop!(StatsSumRowM2x3, Matrix2x3<T>, RowVector3<T>, sum_row_op);
  #[cfg(feature = "Matrix3x2")]
  impl_stats_urop!(StatsSumRowM3x2, Matrix3x2<T>, RowVector2<T>, sum_row_op);
  #[cfg(feature = "MatrixD")]
  impl_stats_urop!(StatsSumRowMD, DMatrix<T>, RowDVector<T>, sum_row_op);
  #[cfg(feature = "Vector2")]
  impl_stats_urop!(StatsSumRowV2, Vector2<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "Vector3")]
  impl_stats_urop!(StatsSumRowV3, Vector3<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "Vector4")]
  impl_stats_urop!(StatsSumRowV4, Vector4<T>, Matrix1<T>, sum_row_op); 
  #[cfg(feature = "VectorD")]
  impl_stats_urop!(StatsSumRowVD, DVector<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "RowVector2")]
  impl_stats_urop!(StatsSumRowR2, RowVector2<T>, RowVector2<T>, sum_row_op);
  #[cfg(feature = "RowVector3")]
  impl_stats_urop!(StatsSumRowR3, RowVector3<T>, RowVector3<T>, sum_row_op);
  #[cfg(feature = "RowVector4")]
  impl_stats_urop!(StatsSumRowR4, RowVector4<T>, RowVector4<T>, sum_row_op); 
  #[cfg(feature = "RowVectorD")]
  impl_stats_urop!(StatsSumRowRD, RowDVector<T>, RowDVector<T>, sum_row_op);
  
  macro_rules! impl_stats_sum_row_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) =>Ok(Box::new(StatsSumRowR4{arg: arg.clone(), out: new_ref(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) =>Ok(Box::new(StatsSumRowR3{arg: arg.clone(), out: new_ref(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) =>Ok(Box::new(StatsSumRowR2{arg: arg.clone(), out: new_ref(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg)) =>Ok(Box::new(StatsSumRowV4{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg)) =>Ok(Box::new(StatsSumRowV3{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg)) =>Ok(Box::new(StatsSumRowV2{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg)) =>Ok(Box::new(StatsSumRowM4{arg: arg.clone(), out: new_ref(RowVector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg)) =>Ok(Box::new(StatsSumRowM3{arg: arg.clone(), out: new_ref(RowVector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg)) =>Ok(Box::new(StatsSumRowM2{arg: arg.clone(), out: new_ref(RowVector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg)) =>Ok(Box::new(StatsSumRowM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg)) => Ok(Box::new(StatsSumRowM2x3{arg: arg.clone(), out: new_ref(RowVector3::from_element($target_type::default()))})),          
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg)) => Ok(Box::new(StatsSumRowM3x2{arg: arg.clone(), out: new_ref(RowVector2::from_element($target_type::default()))})),          
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg)) =>Ok(Box::new(StatsSumRowVD{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) =>Ok(Box::new(StatsSumRowRD{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(), $target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DMatrix(arg)) =>Ok(Box::new(StatsSumRowMD{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().ncols(), $target_type::default()))})),
          )+
        )+
        _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

  
  fn impl_stats_sum_row_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
    impl_stats_sum_row_match_arms!(
      lhs_value,
      I8,   i8,   "I8";
      I16,  i16,  "I16";
      I32,  i32,  "I32";
      I64,  i64,  "I64";
      I128, i128, "I128";
      U8,   u8,   "U8";
      U16,  u16,  "U16";
      U32,  u32,  "U32";
      U64,  u64,  "U64";
      U128, u128, "U128";
      F32,  F32,  "F32";
      F64,  F64,  "F64";
      ComplexNumber, ComplexNumber, "ComplexNumber";
      RationalNumber, RationalNumber, "RationalNumber";
    )
  }
    
  impl_mech_urnop_fxn!(StatsSumRow,impl_stats_sum_row_fxn);
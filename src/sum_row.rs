use crate::*;
use mech_core::*;
use num_traits::*;

// Stats Sum Row -----------------------------------------------------------

macro_rules! sum_row_op {
    ($arg:expr, $out:expr) => {
      unsafe { 
        *$out = (*$arg).row_sum();
      }
    };}
  
  #[cfg(feature = "matrix1")]
  impl_stats_urop!(StatsSumRowM1, Matrix1<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "matrix2")]
  impl_stats_urop!(StatsSumRowM2, Matrix2<T>, RowVector2<T>, sum_row_op);
  #[cfg(feature = "matrix3")]
  impl_stats_urop!(StatsSumRowM3, Matrix3<T>, RowVector3<T>, sum_row_op);
  #[cfg(feature = "matrix4")]
  impl_stats_urop!(StatsSumRowM4, Matrix4<T>, RowVector4<T>, sum_row_op);
  #[cfg(feature = "matrix2x3")]
  impl_stats_urop!(StatsSumRowM2x3, Matrix2x3<T>, RowVector3<T>, sum_row_op);
  #[cfg(feature = "matrix3x2")]
  impl_stats_urop!(StatsSumRowM3x2, Matrix3x2<T>, RowVector2<T>, sum_row_op);
  #[cfg(feature = "matrixd")]
  impl_stats_urop!(StatsSumRowMD, DMatrix<T>, RowDVector<T>, sum_row_op);
  #[cfg(feature = "vector2")]
  impl_stats_urop!(StatsSumRowV2, Vector2<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "vector3")]
  impl_stats_urop!(StatsSumRowV3, Vector3<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "vector4")]
  impl_stats_urop!(StatsSumRowV4, Vector4<T>, Matrix1<T>, sum_row_op); 
  #[cfg(feature = "vectord")]
  impl_stats_urop!(StatsSumRowVD, DVector<T>, Matrix1<T>, sum_row_op);
  #[cfg(feature = "row_vector2")]
  impl_stats_urop!(StatsSumRowR2, RowVector2<T>, RowVector2<T>, sum_row_op);
  #[cfg(feature = "row_vector3")]
  impl_stats_urop!(StatsSumRowR3, RowVector3<T>, RowVector3<T>, sum_row_op);
  #[cfg(feature = "row_vector4")]
  impl_stats_urop!(StatsSumRowR4, RowVector4<T>, RowVector4<T>, sum_row_op); 
  #[cfg(feature = "row_vectord")]
  impl_stats_urop!(StatsSumRowRD, RowDVector<T>, RowDVector<T>, sum_row_op);
  
  macro_rules! impl_stats_sum_row_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) =>Ok(Box::new(StatsSumRowR4{arg: arg.clone(), out: new_ref(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) =>Ok(Box::new(StatsSumRowR3{arg: arg.clone(), out: new_ref(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) =>Ok(Box::new(StatsSumRowR2{arg: arg.clone(), out: new_ref(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg)) =>Ok(Box::new(StatsSumRowV4{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg)) =>Ok(Box::new(StatsSumRowV3{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg)) =>Ok(Box::new(StatsSumRowV2{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg)) =>Ok(Box::new(StatsSumRowM4{arg: arg.clone(), out: new_ref(RowVector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg)) =>Ok(Box::new(StatsSumRowM3{arg: arg.clone(), out: new_ref(RowVector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg)) =>Ok(Box::new(StatsSumRowM2{arg: arg.clone(), out: new_ref(RowVector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg)) =>Ok(Box::new(StatsSumRowM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg)) => Ok(Box::new(StatsSumRowM2x3{arg: arg.clone(), out: new_ref(RowVector3::from_element($target_type::default()))})),          
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg)) => Ok(Box::new(StatsSumRowM3x2{arg: arg.clone(), out: new_ref(RowVector2::from_element($target_type::default()))})),          
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg)) =>Ok(Box::new(StatsSumRowVD{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) =>Ok(Box::new(StatsSumRowRD{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(), $target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
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
      I8,   i8,   "i8";
      I16,  i16,  "i16";
      I32,  i32,  "i32";
      I64,  i64,  "i64";
      I128, i128, "i128";
      U8,   u8,   "u8";
      U16,  u16,  "u16";
      U32,  u32,  "u32";
      U64,  u64,  "u64";
      U128, u128, "u128";
      F32,  F32,  "f32";
      F64,  F64,  "f64";
      ComplexNumber, ComplexNumber, "complex";
      RationalNumber, RationalNumber, "rational";
    )
  }
    
  impl_mech_urnop_fxn!(StatsSumRow,impl_stats_sum_row_fxn);
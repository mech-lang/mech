use crate::*;
use mech_core::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Stats Sum Row -----------------------------------------------------------

macro_rules! sum_row_op {
    ($arg:expr, $out:expr) => {
      unsafe { 
        *$out = (*$arg).row_sum();
      }
    };}


macro_rules! sum_row_op2 {
  ($arg:expr, $out:expr) => {
    unsafe {
      let mut sum = T::zero();
      for i in 0..(*$arg).len() {
        sum += (&(*$arg))[i];
      }
      (&mut (*$out))[(0, 0)] = sum;
    }
  };}
  
  #[cfg(all(feature = "matrix1", feature = "matrix1"))]
  impls_stas!(StatsSumRowM1, Matrix1<T>, Matrix1<T>, sum_row_op);
  #[cfg(all(feature = "matrix2", feature = "row_vector2"))]
  impls_stas!(StatsSumRowM2, Matrix2<T>, RowVector2<T>, sum_row_op);
  #[cfg(all(feature = "matrix3", feature = "row_vector3"))]
  impls_stas!(StatsSumRowM3, Matrix3<T>, RowVector3<T>, sum_row_op);
  #[cfg(all(feature = "matrix4", feature = "row_vector4"))]
  impls_stas!(StatsSumRowM4, Matrix4<T>, RowVector4<T>, sum_row_op);
  #[cfg(all(feature = "matrix2x3", feature = "row_vector3"))]
  impls_stas!(StatsSumRowM2x3, Matrix2x3<T>, RowVector3<T>, sum_row_op);
  #[cfg(all(feature = "matrix3x2", feature = "row_vector2"))]
  impls_stas!(StatsSumRowM3x2, Matrix3x2<T>, RowVector2<T>, sum_row_op);
  #[cfg(all(feature = "matrixd", feature = "row_vectord"))]
  impls_stas!(StatsSumRowMD, DMatrix<T>, RowDVector<T>, sum_row_op);
  #[cfg(all(feature = "vector2", feature = "matrix1"))]
  impls_stas!(StatsSumRowV2, Vector2<T>, Matrix1<T>, sum_row_op);
  #[cfg(all(feature = "vector3", feature = "matrix1"))]
  impls_stas!(StatsSumRowV3, Vector3<T>, Matrix1<T>, sum_row_op);
  #[cfg(all(feature = "vector4", feature = "matrix1"))]
  impls_stas!(StatsSumRowV4, Vector4<T>, Matrix1<T>, sum_row_op); 
  #[cfg(all(feature = "vectord", feature = "matrix1"))]
  impls_stas!(StatsSumRowVD, DVector<T>, Matrix1<T>, sum_row_op);
  #[cfg(all(feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
  impls_stas!(StatsSumRowVDMD, DVector<T>, DMatrix<T>, sum_row_op2);
  #[cfg(all(feature = "row_vector2", feature = "row_vector2"))]
  impls_stas!(StatsSumRowR2, RowVector2<T>, RowVector2<T>, sum_row_op);
  #[cfg(all(feature = "row_vector3", feature = "row_vector3"))]
  impls_stas!(StatsSumRowR3, RowVector3<T>, RowVector3<T>, sum_row_op);
  #[cfg(all(feature = "row_vector4", feature = "row_vector4"))]
  impls_stas!(StatsSumRowR4, RowVector4<T>, RowVector4<T>, sum_row_op); 
  #[cfg(all(feature = "row_vectord", feature = "row_vectord"))]
  impls_stas!(StatsSumRowRD, RowDVector<T>, RowDVector<T>, sum_row_op);
  
  macro_rules! impl_stats_sum_row_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "row_vector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(StatsSumRowR4{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "row_vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(StatsSumRowR3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "row_vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(StatsSumRowR2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg)) => Ok(Box::new(StatsSumRowV4{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg)) => Ok(Box::new(StatsSumRowV3{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg)) => Ok(Box::new(StatsSumRowV2{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "row_vector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg)) => Ok(Box::new(StatsSumRowM4{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg)) => Ok(Box::new(StatsSumRowM3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg)) => Ok(Box::new(StatsSumRowM2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg)) => Ok(Box::new(StatsSumRowM1{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg)) => Ok(Box::new(StatsSumRowM2x3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg)) => Ok(Box::new(StatsSumRowM3x2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg)) => Ok(Box::new(StatsSumRowVD{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg)) => Ok(Box::new(StatsSumRowVDMD{arg: arg.clone(), out: Ref::new(DMatrix::from_element(1,1,$target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "row_vectord"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(StatsSumRowRD{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(), $target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "row_vectord"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DMatrix(arg)) => Ok(Box::new(StatsSumRowMD{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().ncols(), $target_type::default())) })),
          )+
        )+
        x => Err(MechError2::new(
            UnhandledFunctionArgumentKind1 {arg: x.kind(), fxn_name: stringify!(StatsSumRow).to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  }
}

  
  fn impl_stats_sum_row_fxn(lhs_value: Value) -> MResult<Box<dyn MechFunction>> {
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
      F32,  f32,  "f32";
      F64,  f64,  "f64";
      C64, C64, "complex";
      R64, R64, "rational";
    )
  }
    
  impl_mech_urnop_fxn!(StatsSumRow,impl_stats_sum_row_fxn,"stats/sum/row");
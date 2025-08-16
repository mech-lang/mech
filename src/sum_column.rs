use crate::*;
use mech_core::*;
use num_traits::*;

// Stats Sum Column -----------------------------------------------------------

macro_rules! sum_column_op {
    ($arg:expr, $out:expr) => {
      unsafe { 
        *$out = (*$arg).column_sum();
      }
    };}

#[cfg(feature = "matrix1")]
impl_stats_urop!(StatsSumColumnM1, Matrix1<T>, Matrix1<T>, sum_column_op);
#[cfg(feature = "matrix2")]
impl_stats_urop!(StatsSumColumnM2, Matrix2<T>, Vector2<T>, sum_column_op);
#[cfg(feature = "matrix3")]
impl_stats_urop!(StatsSumColumnM3, Matrix3<T>, Vector3<T>, sum_column_op);
#[cfg(feature = "matrix4")]
impl_stats_urop!(StatsSumColumnM4, Matrix4<T>, Vector4<T>, sum_column_op);
#[cfg(feature = "matrix2x3")]
impl_stats_urop!(StatsSumColumnM2x3, Matrix2x3<T>, Vector2<T>, sum_column_op);
#[cfg(feature = "matrix3x2")]
impl_stats_urop!(StatsSumColumnM3x2, Matrix3x2<T>, Vector3<T>, sum_column_op);
#[cfg(feature = "matrixd")]
impl_stats_urop!(StatsSumColumnMD, DMatrix<T>, DVector<T>, sum_column_op);
#[cfg(feature = "vector2")]
impl_stats_urop!(StatsSumColumnV2, Vector2<T>, Vector2<T>, sum_column_op);
#[cfg(feature = "vector3")]
impl_stats_urop!(StatsSumColumnV3, Vector3<T>, Vector3<T>, sum_column_op);
#[cfg(feature = "vector4")]
impl_stats_urop!(StatsSumColumnV4, Vector4<T>, Vector4<T>, sum_column_op); 
#[cfg(feature = "vectord")]
impl_stats_urop!(StatsSumColumnVD, DVector<T>, DVector<T>, sum_column_op);
#[cfg(feature = "row_vector2")]
impl_stats_urop!(StatsSumColumnR2, RowVector2<T>, Matrix1<T>, sum_column_op);
#[cfg(feature = "row_vector3")]
impl_stats_urop!(StatsSumColumnR3, RowVector3<T>, Matrix1<T>, sum_column_op);
#[cfg(feature = "row_vector4")]
impl_stats_urop!(StatsSumColumnR4, RowVector4<T>, Matrix1<T>, sum_column_op); 
#[cfg(feature = "row_vectord")]
impl_stats_urop!(StatsSumColumnRD, RowDVector<T>, Matrix1<T>, sum_column_op);

macro_rules! impl_stats_sum_column_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(StatsSumColumnR4{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(StatsSumColumnR3{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(StatsSumColumnR2{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg))    => Ok(Box::new(StatsSumColumnV4{arg: arg.clone(), out: new_ref(Vector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg))    => Ok(Box::new(StatsSumColumnV3{arg: arg.clone(), out: new_ref(Vector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg))    => Ok(Box::new(StatsSumColumnV2{arg: arg.clone(), out: new_ref(Vector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg))    => Ok(Box::new(StatsSumColumnM4{arg: arg.clone(), out: new_ref(Vector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg))    => Ok(Box::new(StatsSumColumnM3{arg: arg.clone(), out: new_ref(Vector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg))    => Ok(Box::new(StatsSumColumnM2{arg: arg.clone(), out: new_ref(Vector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg))    => Ok(Box::new(StatsSumColumnM1{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg))  => Ok(Box::new(StatsSumColumnM2x3{arg: arg.clone(), out: new_ref(Vector2::from_element($target_type::default()))})),          
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg))  => Ok(Box::new(StatsSumColumnM3x2{arg: arg.clone(), out: new_ref(Vector3::from_element($target_type::default()))})),          
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg))    => Ok(Box::new(StatsSumColumnVD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(StatsSumColumnRD{arg: arg.clone(), out: new_ref(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            Value::[<Matrix $input_type>](Matrix::<$target_type>::DMatrix(arg)) => Ok(Box::new(StatsSumColumnMD{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().nrows(),$target_type::default()))})),
          )+
        )+
        _ => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_stats_sum_column_fxn(lhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_stats_sum_column_match_arms!(
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
    RationalNumber, RationalNumber, "rational"
  )
}
  
impl_mech_urnop_fxn!(StatsSumColumn,impl_stats_sum_column_fxn); 
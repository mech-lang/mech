use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;
use num_traits::*;

// Stats Sum Row -----------------------------------------------------------

macro_rules! sum_row_op {
    ($arg:expr, $out:expr) => {
        {
            for column in 0..($arg).ncols() {
                let mut sum = T::zero();
                for row in 0..($arg).nrows() {
                    sum = checked_sum_add(sum, ($arg)[(row, column)])?;
                }
                ($out)[column] = sum;
            }
            Ok::<(), MechError>(())
        }
    };
}

#[cfg(all(feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
macro_rules! sum_row_op2 {
    ($arg:expr, $out:expr) => {
        {
            let mut sum = T::zero();
            for value in ($arg).iter().copied() {
                sum = checked_sum_add(sum, value)?;
            }
            ($out)[(0, 0)] = sum;
            Ok::<(), MechError>(())
        }
    };
}

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

#[cfg(feature = "source")]
macro_rules! impl_stats_sum_row_match_arms {
  ($arg:expr, $($input_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "row_vector4"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector4(arg)) => Ok(Box::new(StatsSumRowR4{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "row_vector3"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector3(arg)) => Ok(Box::new(StatsSumRowR3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "row_vector2"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowVector2(arg)) => Ok(Box::new(StatsSumRowR2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector4", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector4(arg)) => Ok(Box::new(StatsSumRowV4{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector3", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector3(arg)) => Ok(Box::new(StatsSumRowV3{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector2", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Vector2(arg)) => Ok(Box::new(StatsSumRowV2{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "row_vector4"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix4(arg)) => Ok(Box::new(StatsSumRowM4{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3(arg)) => Ok(Box::new(StatsSumRowM3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2(arg)) => Ok(Box::new(StatsSumRowM2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix1", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix1(arg)) => Ok(Box::new(StatsSumRowM1{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix2x3(arg)) => Ok(Box::new(StatsSumRowM2x3{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::Matrix3x2(arg)) => Ok(Box::new(StatsSumRowM3x2{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrix1"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg)) => Ok(Box::new(StatsSumRowVD{arg: arg.clone(), out: Ref::new(Matrix1::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::DVector(arg)) => Ok(Box::new(StatsSumRowVDMD{arg: arg.clone(), out: Ref::new(DMatrix::from_element(1,1,$target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "row_vectord"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::RowDVector(arg)) => Ok(Box::new(StatsSumRowRD{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(), $target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "matrixd", feature = "row_vectord"))]
            LegacyValue::[<Matrix $input_type>](Matrix::<$target_type>::DMatrix(arg)) => Ok(Box::new(StatsSumRowMD{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().ncols(), $target_type::default())) })),
          )+
        )+
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind1 {arg: x.kind(), fxn_name: stringify!(StatsSumRow).to_string() },
            None
          ).with_compiler_loc()
        ),
      }
    }
  }
}

#[cfg(feature = "source")]
fn impl_stats_sum_row_fxn(lhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
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

#[cfg(feature = "source")]
impl_mech_urnop_fxn!(StatsSumRow, impl_stats_sum_row_fxn, "stats/sum/row");

#[cfg(all(test, feature = "u8"))]
mod checked_sum_tests {
    use super::*;

    #[cfg(feature = "u8")]
    #[test]
    fn integer_row_sum_rejects_reactive_overflow_and_retains_output() {
        let arg = Ref::new(DMatrix::from_row_slice(2, 1, &[1u8, 2]));
        let out = Ref::new(RowDVector::from_vec(vec![99u8]));
        let function = StatsSumRowMD::<u8> {
            arg: arg.clone(),
            out: out.clone(),
        };
        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[3]);
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&function)?;
            *arg.borrow_mut() = DMatrix::from_row_slice(2, 1, &[u8::MAX, 1]);
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "StatsArithmeticOverflow");
            assert_eq!(out.borrow().as_slice(), &[3]);
            *out.borrow_mut() = RowDVector::from_vec(vec![17, 18]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(out.borrow().as_slice(), &[3]);
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "matrix2",
    feature = "row_vector2",
    feature = "matrixd",
    feature = "row_vectord"
))]
mod invocation_port_tests {
    use super::*;

    fn unary_args<I, O>(out: &Ref<O>, arg: &Ref<I>) -> FunctionArgs
    where
        Ref<I>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Unary(out.to_value(), arg.to_value())
    }

    #[test]
    fn fixed_and_dynamic_row_sums_preserve_factory_behavior_and_state() {
        let fixed_arg = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let legacy_out = Ref::new(RowVector2::zeros());
        let invocation_out = Ref::new(RowVector2::zeros());
        let legacy = StatsSumRowM2::<f64>::new(unary_args(&legacy_out, &fixed_arg)).unwrap();
        let invocation = StatsSumRowM2::<f64>::new_invocation(
            unary_args(&invocation_out, &fixed_arg).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), RowVector2::new(4.0, 6.0));
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());

        let dynamic_arg = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_out = Ref::new(RowDVector::zeros(3));
        let dynamic = StatsSumRowMD::<f64>::new_invocation(
            unary_args(&dynamic_out, &dynamic_arg).into(),
        )
        .unwrap();
        dynamic.solve_result().unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            RowDVector::from_vec(vec![5.0, 7.0, 9.0])
        );
        assert_eq!(
            dynamic.reactive_output_cell_ids(),
            dynamic.out().reactive_root_cell_ids(),
        );
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*dynamic)?;
            *dynamic_out.borrow_mut() = RowDVector::from_vec(vec![-1.0]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(
            *dynamic_out.borrow(),
            RowDVector::from_vec(vec![5.0, 7.0, 9.0])
        );
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "vectord",
    feature = "matrixd",
    not(feature = "matrix1")
))]
mod dynamic_fallback_invocation_tests {
    use super::*;

    #[test]
    fn dynamic_vector_row_sum_uses_matrix_fallback_without_matrix1() {
        let arg = Ref::new(DVector::from_vec(vec![1.0_f64, 2.0, 3.0]));
        let out = Ref::new(DMatrix::zeros(1, 1));
        let function = StatsSumRowVDMD::<f64>::new_invocation(
            FunctionArgs::Unary(out.to_value(), arg.to_value()).into(),
        )
        .unwrap();

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), DMatrix::from_element(1, 1, 6.0));
    }
}

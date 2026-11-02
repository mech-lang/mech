#[cfg(any(
    feature = "matrix1",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord",
    all(feature = "matrixd", feature = "row_vectord")
))]
use crate::*;
#[cfg(any(
    feature = "matrix1",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord",
    all(feature = "matrixd", feature = "row_vectord")
))]
use num_traits::*;

// Stats Sum Column -----------------------------------------------------------

#[cfg(any(
    feature = "matrix1",
    feature = "vector2",
    feature = "vector3",
    feature = "vector4",
    feature = "vectord",
    all(feature = "matrixd", feature = "row_vectord")
))]
macro_rules! sum_column_op {
    ($arg:expr, $out:expr) => {{
        if ($out).len() != ($arg).rows() {
            return Err(function_shape_contract_violation(
                "stats/sum/column",
                "column reduction output cardinality disagrees with the input rows",
            ));
        }
        ($out).try_fill_column_major(|row| {
            let mut sum = T::zero();
            for column in 0..($arg).columns() {
                sum = checked_sum_add(
                    sum,
                    ($arg)
                        .get(row, column)
                        .expect("validated column-reduction input lane"),
                )?;
            }
            Ok(sum)
        })
    }};
}

#[cfg(all(feature = "matrix1", feature = "matrix1"))]
impls_stas!(StatsSumColumnM1, Matrix1<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "matrix2", feature = "vector2"))]
impls_stas!(StatsSumColumnM2, Matrix2<T>, Vector2<T>, sum_column_op);
#[cfg(all(feature = "matrix3", feature = "vector3"))]
impls_stas!(StatsSumColumnM3, Matrix3<T>, Vector3<T>, sum_column_op);
#[cfg(all(feature = "matrix4", feature = "vector4"))]
impls_stas!(StatsSumColumnM4, Matrix4<T>, Vector4<T>, sum_column_op);
#[cfg(all(feature = "matrix2x3", feature = "vector2"))]
impls_stas!(StatsSumColumnM2x3, Matrix2x3<T>, Vector2<T>, sum_column_op);
#[cfg(all(feature = "matrix3x2", feature = "vector3"))]
impls_stas!(StatsSumColumnM3x2, Matrix3x2<T>, Vector3<T>, sum_column_op);
#[cfg(all(feature = "matrixd", feature = "vectord"))]
impls_stas!(StatsSumColumnMD, DMatrix<T>, DVector<T>, sum_column_op);
#[cfg(all(feature = "vector2", feature = "vector2"))]
impls_stas!(StatsSumColumnV2, Vector2<T>, Vector2<T>, sum_column_op);
#[cfg(all(feature = "vector3", feature = "vector3"))]
impls_stas!(StatsSumColumnV3, Vector3<T>, Vector3<T>, sum_column_op);
#[cfg(all(feature = "vector4", feature = "vector4"))]
impls_stas!(StatsSumColumnV4, Vector4<T>, Vector4<T>, sum_column_op);
#[cfg(all(feature = "vectord", feature = "vectord"))]
impls_stas!(StatsSumColumnVD, DVector<T>, DVector<T>, sum_column_op);
#[cfg(all(feature = "row_vector2", feature = "matrix1"))]
impls_stas!(StatsSumColumnR2, RowVector2<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "row_vector3", feature = "matrix1"))]
impls_stas!(StatsSumColumnR3, RowVector3<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "row_vector4", feature = "matrix1"))]
impls_stas!(StatsSumColumnR4, RowVector4<T>, Matrix1<T>, sum_column_op);
#[cfg(all(feature = "row_vectord", feature = "matrix1"))]
impls_stas!(StatsSumColumnRD, RowDVector<T>, Matrix1<T>, sum_column_op);

#[cfg(all(feature = "row_vectord", feature = "matrixd", not(feature = "matrix1")))]
impls_stas!(StatsSumColumnRD2, RowDVector<T>, DMatrix<T>, sum_column_op);
#[cfg(feature = "source")]
pub struct StatsSumColumn;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for StatsSumColumn {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input = invocation.input(0).expect("validated column-sum input");
        let shape = input.matrix_descriptor()?.ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role: FunctionArgumentRole::Input(0),
                    expected: "matrix input".into(),
                    found: format!("{:?}", input.representation()),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![vec![shape.rows as u64, 1_u64].into_boxed_slice()].into_boxed_slice(),
            &[input],
        )
    }
}

#[cfg(all(test, any(feature = "u8", feature = "rational")))]
mod checked_sum_tests {
    use super::*;

    #[cfg(feature = "u8")]
    #[test]
    fn integer_column_sum_rejects_reactive_overflow_and_retains_output() {
        let arg = ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[1u8, 2])).unwrap();
        let out = ValueCell::from_exact(DVector::from_vec(vec![99u8])).unwrap();
        let function = crate::test_managed_factory::<StatsSumColumnMD<u8>>(
            FunctionInvocation::unary(out.clone(), arg.clone()),
            "test/stats-sum-column",
        );
        function.instance().solve_result().unwrap();
        crate::assert_test_value(
            &out,
            ValueCell::from_exact(DVector::from_vec(vec![3u8])).unwrap(),
        );
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_instance(function.instance())?;
            arg.replace(
                &ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[u8::MAX, 1]))?.snapshot()?,
            )?;
            let error = function.instance().solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "StatsArithmeticOverflow");
            crate::assert_test_value(
                &out,
                ValueCell::from_exact(DVector::from_vec(vec![3u8])).unwrap(),
            );
            out.replace(&ValueCell::from_exact(DVector::from_vec(vec![17u8, 18]))?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        crate::assert_test_value(
            &out,
            ValueCell::from_exact(DVector::from_vec(vec![3u8])).unwrap(),
        );
    }

    #[cfg(feature = "rational")]
    #[test]
    fn bounded_rational_column_sum_is_checked() {
        let arg = ValueCell::from_exact(DMatrix::from_row_slice(
            1,
            2,
            &[R64::new(i64::MAX, 1), R64::new(1, 1)],
        ))
        .unwrap();
        let out = ValueCell::from_exact(DVector::from_vec(vec![R64::new(7, 1)])).unwrap();
        let function = crate::test_managed_factory::<StatsSumColumnMD<R64>>(
            FunctionInvocation::unary(out.clone(), arg),
            "test/stats-sum-column",
        );
        let error = function.instance().solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "StatsArithmeticOverflow");
        crate::assert_test_value(
            &out,
            ValueCell::from_exact(DVector::from_vec(vec![R64::new(7, 1)])).unwrap(),
        );
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "matrix2",
    feature = "vector2",
    feature = "matrixd",
    feature = "vectord"
))]
mod canonical_port_tests {
    use super::*;

    #[test]
    fn column_sum_preserves_exact_storage_identity_and_dynamic_state() {
        let fixed_arg = ValueCell::from_exact(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0)).unwrap();
        let fixed_out = ValueCell::from_exact(Vector2::<f64>::zeros()).unwrap();
        let fixed_alias = fixed_out.clone();
        crate::test_managed_factory::<StatsSumColumnM2<f64>>(
            FunctionInvocation::unary(fixed_out.clone(), fixed_arg),
            "test/stats-sum-column",
        )
        .instance()
        .solve_result()
        .unwrap();
        assert!(fixed_out.same_cell(&fixed_alias));
        crate::assert_test_value(
            &fixed_out,
            ValueCell::from_exact(Vector2::new(3.0, 7.0)).unwrap(),
        );

        let dynamic_arg = ValueCell::from_exact(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ))
        .unwrap();
        let output = ValueCell::from_exact(DVector::<f64>::zeros(2)).unwrap();
        let function = crate::test_managed_factory::<StatsSumColumnMD<f64>>(
            FunctionInvocation::unary(output.clone(), dynamic_arg),
            "test/stats-sum-column",
        );
        function.instance().solve_result().unwrap();
        assert_eq!(
            function.instance().reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            output.replace(
                &ValueCell::from_exact(DVector::from_vec(vec![-1.0, -2.0, -3.0]))?.snapshot()?,
            )?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        crate::assert_test_value(
            &output,
            ValueCell::from_exact(DVector::from_vec(vec![6.0, 15.0])).unwrap(),
        );
    }

    #[test]
    fn column_sum_rejects_wrong_exact_storage_and_binary_layout() {
        let out = ValueCell::from_exact(Vector2::<f64>::zeros()).unwrap();
        let wrong_arg = ValueCell::from_exact(DMatrix::<f64>::zeros(2, 2)).unwrap();
        assert!(
            StatsSumColumnM2::<f64>::new_invocation(FunctionInvocation::unary(
                out.clone(),
                wrong_arg,
            ))
            .is_err()
        );

        let input = ValueCell::from_exact(Matrix2::<f64>::zeros()).unwrap();
        let error = StatsSumColumnM2::<f64>::new_invocation(FunctionInvocation::binary(
            out,
            input.clone(),
            input,
        ))
        .err()
        .expect("binary layout must be rejected");
        let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((arity.expected, arity.found), (1, 2));
    }
}

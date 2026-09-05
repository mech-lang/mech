use crate::*;
use num_traits::*;

// Stats Sum Row -----------------------------------------------------------

macro_rules! sum_row_op {
    ($arg:expr, $out:expr) => {{
        for column in 0..($arg).ncols() {
            let mut sum = T::zero();
            for row in 0..($arg).nrows() {
                sum = checked_sum_add(sum, ($arg)[(row, column)])?;
            }
            ($out)[column] = sum;
        }
        Ok::<(), MechError>(())
    }};
}

#[cfg(all(feature = "vectord", feature = "matrixd", not(feature = "matrix1")))]
macro_rules! sum_row_op2 {
    ($arg:expr, $out:expr) => {{
        let mut sum = T::zero();
        for value in ($arg).iter().copied() {
            sum = checked_sum_add(sum, value)?;
        }
        ($out)[(0, 0)] = sum;
        Ok::<(), MechError>(())
    }};
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
pub struct StatsSumRow;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for StatsSumRow {
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
        let input = invocation.input(0).expect("validated row-sum input");
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
            vec![vec![1_u64, shape.cols as u64].into_boxed_slice()].into_boxed_slice(),
            &[input],
        )
    }
}

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
mod canonical_port_tests {
    use super::*;

    #[test]
    fn row_sum_preserves_exact_storage_identity_and_dynamic_state() {
        let fixed_arg = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let fixed_out = Ref::new(RowVector2::zeros());
        let fixed_alias = fixed_out.clone();
        StatsSumRowM2::<f64>::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact_matrix_ref(fixed_out.clone(), 1, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(fixed_arg, 2, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert!(fixed_out.same_handle(&fixed_alias));
        assert_eq!(*fixed_out.borrow(), RowVector2::new(4.0, 6.0));

        let dynamic_arg = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let dynamic_out = Ref::new(RowDVector::zeros(3));
        let output = ValueCell::from_exact_matrix_ref(dynamic_out.clone(), 1, 3).unwrap();
        let function = StatsSumRowMD::<f64>::new_invocation(FunctionInvocation::unary(
            output.clone(),
            ValueCell::from_exact_matrix_ref(dynamic_arg, 2, 3).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(
            function.reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );
        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
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

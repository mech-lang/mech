#[cfg(all(
    feature = "f64",
    feature = "matrixd",
    feature = "row_vectord",
    feature = "inclusive",
    feature = "exclusive",
    feature = "inclusive_increment",
    feature = "exclusive_increment"
))]
mod canonical_ranges {
    use crate::{
        RangeExclusiveScalar, RangeIncrementExclusiveScalar, RangeIncrementInclusiveScalar,
        RangeInclusiveScalar,
    };
    use mech_core::{
        FunctionInvocation, IncorrectNumberOfArguments, MResult, MechError, MechFunction,
        MechFunctionFactory, Ref, ValueCell, with_reactive_journal_participant,
    };
    use nalgebra::{DMatrix, RowDVector};

    fn output(length: usize) -> (Ref<DMatrix<f64>>, ValueCell) {
        let reference = Ref::new(DMatrix::from_element(1, length, 0.0));
        let cell = ValueCell::from_exact_matrix_ref(reference.clone(), 1, length).unwrap();
        (reference, cell)
    }

    fn factory_error(result: Result<Box<dyn MechFunction>, MechError>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    #[test]
    fn all_four_range_families_use_canonical_invocations() {
        let (inclusive_out, inclusive_cell) = output(3);
        let inclusive = RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionInvocation::binary(
                inclusive_cell,
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(3.0).unwrap(),
            ),
        )
        .unwrap();
        inclusive.solve_result().unwrap();
        assert_eq!(inclusive_out.borrow().as_slice(), &[1.0, 2.0, 3.0]);

        let (exclusive_out, exclusive_cell) = output(3);
        let exclusive = RangeExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionInvocation::binary(
                exclusive_cell,
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(4.0).unwrap(),
            ),
        )
        .unwrap();
        exclusive.solve_result().unwrap();
        assert_eq!(exclusive_out.borrow().as_slice(), &[1.0, 2.0, 3.0]);

        let (inclusive_step_out, inclusive_step_cell) = output(3);
        let inclusive_step =
            RangeIncrementInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::ternary(
                    inclusive_step_cell,
                    ValueCell::from_exact(1.0).unwrap(),
                    ValueCell::from_exact(2.0).unwrap(),
                    ValueCell::from_exact(5.0).unwrap(),
                ),
            )
            .unwrap();
        inclusive_step.solve_result().unwrap();
        assert_eq!(inclusive_step_out.borrow().as_slice(), &[1.0, 3.0, 5.0]);

        let (exclusive_step_out, exclusive_step_cell) = output(3);
        let exclusive_step =
            RangeIncrementExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::ternary(
                    exclusive_step_cell,
                    ValueCell::from_exact(1.0).unwrap(),
                    ValueCell::from_exact(2.0).unwrap(),
                    ValueCell::from_exact(7.0).unwrap(),
                ),
            )
            .unwrap();
        exclusive_step.solve_result().unwrap();
        assert_eq!(exclusive_step_out.borrow().as_slice(), &[1.0, 3.0, 5.0]);
    }

    #[test]
    fn range_ports_reject_wrong_types_and_storage() {
        let (_, output) = output(3);
        assert!(
            RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::binary(
                    output.clone(),
                    ValueCell::from_exact(1_usize).unwrap(),
                    ValueCell::from_exact(3.0).unwrap(),
                ),
            )
            .is_err()
        );

        let wrong = Ref::new(RowDVector::from_element(3, 0.0));
        let wrong = ValueCell::from_exact_matrix_ref(wrong, 1, 3).unwrap();
        assert!(
            RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::binary(
                    wrong,
                    ValueCell::from_exact(1.0).unwrap(),
                    ValueCell::from_exact(3.0).unwrap(),
                ),
            )
            .is_err()
        );
    }

    #[test]
    fn range_layout_errors_report_canonical_arities() {
        let (_, output) = output(1);
        let scalar = ValueCell::from_exact(1.0).unwrap();
        for error in [
            factory_error(RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::unary(output.clone(), scalar.clone()),
            )),
            factory_error(RangeExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::unary(output.clone(), scalar.clone()),
            )),
        ] {
            let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
            assert_eq!((arity.expected, arity.found), (2, 1));
        }
        for error in [
            factory_error(
                RangeIncrementInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                    FunctionInvocation::binary(output.clone(), scalar.clone(), scalar.clone()),
                ),
            ),
            factory_error(
                RangeIncrementExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                    FunctionInvocation::binary(output, scalar.clone(), scalar),
                ),
            ),
        ] {
            let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
            assert_eq!((arity.expected, arity.found), (3, 2));
        }
    }

    #[test]
    fn stepped_range_checkpoint_restores_dynamic_extent_and_identity() {
        let terminal = ValueCell::from_exact(5.0).unwrap();
        let (output_ref, output_cell) = output(3);
        let output_alias = output_cell.clone();
        let reference_alias = output_ref.clone();
        let schema = output_cell.schema_key();
        let function = RangeIncrementInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionInvocation::ternary(
                output_cell.clone(),
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(2.0).unwrap(),
                terminal.clone(),
            ),
        )
        .unwrap();
        function.solve_result().unwrap();

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            terminal.replace(&ValueCell::from_exact(7.0)?.snapshot()?)?;
            function.solve_result()?;
            assert_eq!(output_ref.borrow().as_slice(), &[1.0, 3.0, 5.0, 7.0]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(output_ref.same_handle(&reference_alias));
        assert!(output_cell.same_cell(&output_alias));
        assert_eq!(output_cell.schema_key(), schema);
        assert_eq!(output_ref.borrow().as_slice(), &[1.0, 3.0, 5.0]);
    }
}

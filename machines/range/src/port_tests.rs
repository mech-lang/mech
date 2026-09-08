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
        RangeExclusiveScalar, RangeInclusiveScalar, RangeIncrementExclusiveScalar,
        RangeIncrementInclusiveScalar,
    };
    use mech_core::{
        ExecutionTarget, FunctionInvocation, IncorrectNumberOfArguments, MResult, MechError,
        MechFunction, MechFunctionFactory, ResolvedOperationDescriptor, RuntimeFunctionId,
        SpecializedFunction, ValueCell, with_reactive_journal_participant,
    };
    use nalgebra::{DMatrix, RowDVector};

    fn output(length: usize) -> ValueCell {
        ValueCell::from_exact(DMatrix::from_element(1, length, 0.0)).unwrap()
    }

    fn managed<F: MechFunctionFactory>(
        invocation: FunctionInvocation,
        operation: &'static str,
    ) -> SpecializedFunction {
        let implementation = F::new_invocation(invocation.clone()).unwrap();
        let contract = F::declared_operation_contract().unwrap().clone();
        SpecializedFunction::syntax_directed(
            (implementation, invocation),
            ResolvedOperationDescriptor::from_name(operation, contract).unwrap(),
            RuntimeFunctionId::from_name(operation),
            ExecutionTarget::DirectRuntime,
            F::implementation_memory_class(),
        )
        .unwrap()
    }

    fn assert_output(actual: &ValueCell, expected: &[f64]) {
        let expected = ValueCell::from_exact(DMatrix::from_row_slice(1, expected.len(), expected))
            .unwrap()
            .snapshot()
            .unwrap();
        let actual = actual.snapshot().unwrap();
        let actual_schemas = actual.schemas().unwrap();
        let expected_schemas = expected.schemas().unwrap();
        assert!(
            actual
                .language_eq(&actual_schemas, &expected, &expected_schemas)
                .unwrap()
        );
    }

    fn factory_error(result: Result<Box<dyn MechFunction>, MechError>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    #[test]
    fn all_four_range_families_use_canonical_invocations() {
        let inclusive_out = output(3);
        let inclusive = managed::<RangeInclusiveScalar<f64, DMatrix<f64>>>(
            FunctionInvocation::binary(
                inclusive_out.clone(),
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(3.0).unwrap(),
            ),
            "range/inclusive",
        );
        inclusive.instance().solve_result().unwrap();
        assert_output(&inclusive_out, &[1.0, 2.0, 3.0]);

        let exclusive_out = output(3);
        let exclusive = managed::<RangeExclusiveScalar<f64, DMatrix<f64>>>(
            FunctionInvocation::binary(
                exclusive_out.clone(),
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(4.0).unwrap(),
            ),
            "range/exclusive",
        );
        exclusive.instance().solve_result().unwrap();
        assert_output(&exclusive_out, &[1.0, 2.0, 3.0]);

        let inclusive_step_out = output(3);
        let inclusive_step = managed::<RangeIncrementInclusiveScalar<f64, DMatrix<f64>>>(
            FunctionInvocation::ternary(
                inclusive_step_out.clone(),
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(2.0).unwrap(),
                ValueCell::from_exact(5.0).unwrap(),
            ),
            "range/inclusive-increment",
        );
        inclusive_step.instance().solve_result().unwrap();
        assert_output(&inclusive_step_out, &[1.0, 3.0, 5.0]);

        let exclusive_step_out = output(3);
        let exclusive_step = managed::<RangeIncrementExclusiveScalar<f64, DMatrix<f64>>>(
            FunctionInvocation::ternary(
                exclusive_step_out.clone(),
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(2.0).unwrap(),
                ValueCell::from_exact(7.0).unwrap(),
            ),
            "range/exclusive-increment",
        );
        exclusive_step.instance().solve_result().unwrap();
        assert_output(&exclusive_step_out, &[1.0, 3.0, 5.0]);
    }

    #[test]
    fn range_ports_reject_wrong_types_and_storage() {
        let output = output(3);
        assert!(
            RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact(1_usize).unwrap(),
                ValueCell::from_exact(3.0).unwrap(),
            ),)
            .is_err()
        );

        let wrong = ValueCell::from_exact(RowDVector::from_element(3, 0.0)).unwrap();
        assert!(
            RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(FunctionInvocation::binary(
                wrong,
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(3.0).unwrap(),
            ),)
            .is_err()
        );
    }

    #[test]
    fn range_layout_errors_report_canonical_arities() {
        let output = output(1);
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
        let output_cell = output(3);
        let output_alias = output_cell.clone();
        let schema = output_cell.schema_key();
        let function = managed::<RangeIncrementInclusiveScalar<f64, DMatrix<f64>>>(
            FunctionInvocation::ternary(
                output_cell.clone(),
                ValueCell::from_exact(1.0).unwrap(),
                ValueCell::from_exact(2.0).unwrap(),
                terminal.clone(),
            ),
            "range/inclusive-increment",
        );
        function.instance().solve_result().unwrap();

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            terminal.replace(&ValueCell::from_exact(7.0)?.snapshot()?)?;
            function.instance().solve_result()?;
            assert_output(&output_cell, &[1.0, 3.0, 5.0, 7.0]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(output_cell.same_logical_cell(&output_alias));
        assert_eq!(output_cell.schema_key(), schema);
        assert_output(&output_cell, &[1.0, 3.0, 5.0]);
    }
}

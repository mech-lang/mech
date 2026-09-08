use mech_core::{
    ExecutionTarget, FunctionInvocation, MResult, MechFunctionFactory, ResolvedOperationDescriptor,
    RuntimeFunctionId, SpecializedFunction,
};

fn managed_factory_instance<F: MechFunctionFactory>(
    invocation: FunctionInvocation,
    operation: &'static str,
) -> MResult<SpecializedFunction> {
    let implementation = F::new_invocation(invocation.clone())?;
    let contract = F::declared_operation_contract()
        .or_else(|| implementation.semantic_operation_contract())
        .expect("managed assignment fixture requires an operation contract");
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name(operation, contract.clone())?,
        RuntimeFunctionId::from_name(operation),
        ExecutionTarget::DirectRuntime,
        F::implementation_memory_class(),
    )
}

fn assert_value_eq(actual: &mech_core::ValueCell, expected: mech_core::ValueCell) {
    let actual = actual.snapshot().unwrap();
    let expected = expected.snapshot().unwrap();
    let actual_schemas = actual.schemas().unwrap();
    let expected_schemas = expected.schemas().unwrap();
    assert!(
        actual
            .language_eq(&actual_schemas, &expected, &expected_schemas)
            .unwrap()
    );
}

#[cfg(all(
    feature = "f64",
    feature = "add_assign",
    feature = "sub_assign",
    feature = "mul_assign",
    feature = "div_assign"
))]
mod scalar {
    use super::super::{
        add_assign::AddAssignSS, div_assign::DivAssignSS, mul_assign::MulAssignSS,
        sub_assign::SubAssignSS,
    };
    use mech_core::{
        FunctionInvocation, IncorrectNumberOfArguments, MResult, MechError, MechFunction,
        MechFunctionFactory, ReactiveNodeKind, ValueCell, ValueData,
        with_reactive_journal_participant,
    };

    fn value(cell: &ValueCell) -> f64 {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::F64(value) = snapshot.data() else {
            panic!("expected f64 cell")
        };
        value.to_f64()
    }

    fn factory_error(result: Result<Box<dyn MechFunction>, MechError>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    macro_rules! assert_assignment {
        ($factory:ident, $expected:expr) => {{
            let sink = ValueCell::from_exact(12.0_f64).unwrap();
            let alias = sink.clone();
            let function = super::managed_factory_instance::<$factory<f64>>(
                FunctionInvocation::unary(sink.clone(), ValueCell::from_exact(3.0_f64).unwrap()),
                "test/math-assignment",
            )
            .unwrap();
            function.instance().solve_result().unwrap();
            assert_eq!(value(&sink), $expected);
            assert!(sink.same_cell(&alias));
            assert_eq!(
                function.instance().reactive_output_cell_ids(),
                vec![sink.reactive_cell_id()]
            );
        }};
    }

    #[test]
    fn all_scalar_assignment_families_use_canonical_invocations() {
        assert_assignment!(AddAssignSS, 15.0);
        assert_assignment!(SubAssignSS, 9.0);
        assert_assignment!(MulAssignSS, 36.0);
        assert_assignment!(DivAssignSS, 4.0);
    }

    #[test]
    fn scalar_assignment_ports_are_exact_and_unary() {
        let sink = ValueCell::from_exact(4.0_f64).unwrap();
        assert!(
            AddAssignSS::<f64>::new_invocation(FunctionInvocation::unary(
                sink.clone(),
                ValueCell::from_exact(2_usize).unwrap(),
            ))
            .is_err()
        );
        let error = factory_error(AddAssignSS::<f64>::new_invocation(
            FunctionInvocation::binary(
                sink,
                ValueCell::from_exact(2.0_f64).unwrap(),
                ValueCell::from_exact(3.0_f64).unwrap(),
            ),
        ));
        let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((arity.expected, arity.found), (1, 2));
    }

    #[test]
    fn scalar_assignment_staging_and_rollback_preserve_identity() {
        let sink = ValueCell::from_exact(2.0_f64).unwrap();
        let alias = sink.clone();
        let function = super::managed_factory_instance::<AddAssignSS<f64>>(
            FunctionInvocation::unary(sink.clone(), ValueCell::from_exact(3.0_f64).unwrap()),
            "test/math-add-assignment",
        )
        .unwrap();
        assert_eq!(
            function.instance().implementation().reactive_node_kind(),
            ReactiveNodeKind::Register
        );
        assert_eq!(
            function
                .instance()
                .implementation()
                .transaction_state_ports()
                .unwrap()
                .unwrap()
                .len(),
            1
        );

        assert_eq!(value(&sink), 2.0);
        function.instance().solve_result().unwrap();
        assert_eq!(value(&sink), 5.0);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            sink.replace(&ValueCell::from_exact(99.0)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(sink.same_cell(&alias));
        assert_eq!(value(&sink), 5.0);
    }
}

#[cfg(all(feature = "f64", feature = "matrix2", feature = "add_assign"))]
mod fixed_matrix {
    use super::super::add_assign::AddAssignVV;
    use mech_core::{FunctionInvocation, ValueCell};
    use nalgebra::Matrix2;

    #[test]
    fn fixed_matrix_assignment_preserves_exact_backing() {
        let source = ValueCell::from_exact(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0)).unwrap();
        let sink = ValueCell::from_exact(Matrix2::from_element(10.0_f64)).unwrap();
        let alias = sink.clone();
        let function =
            super::managed_factory_instance::<AddAssignVV<f64, Matrix2<f64>, Matrix2<f64>>>(
                FunctionInvocation::unary(sink.clone(), source),
                "test/math-add-assignment",
            )
            .unwrap();
        function.instance().solve_result().unwrap();
        assert!(sink.same_cell(&alias));
        super::assert_value_eq(
            &sink,
            ValueCell::from_exact(Matrix2::new(11.0, 12.0, 13.0, 14.0)).unwrap(),
        );
    }
}

#[cfg(all(
    feature = "f64",
    feature = "vectord",
    feature = "add_assign",
    feature = "matrix"
))]
mod indexed {
    use super::super::add_assign::AddAssign1DRS;
    use mech_core::{FunctionInvocation, MResult, ValueCell, with_reactive_journal_participant};
    use nalgebra::DVector;

    #[test]
    fn indexed_assignment_is_atomic_and_checkpointed() {
        let output = ValueCell::from_exact(DVector::from_vec(vec![1.0_f64, 2.0, 3.0])).unwrap();
        let alias = output.clone();
        let indexes = ValueCell::from_exact(DVector::from_vec(vec![1_usize, 3])).unwrap();
        let function =
            super::managed_factory_instance::<AddAssign1DRS<f64, DVector<f64>, DVector<usize>>>(
                FunctionInvocation::binary(
                    output.clone(),
                    ValueCell::from_exact(10.0_f64).unwrap(),
                    indexes.clone(),
                ),
                "test/math-add-indexed-assignment",
            )
            .unwrap();
        function.instance().solve_result().unwrap();
        super::assert_value_eq(
            &output,
            ValueCell::from_exact(DVector::from_vec(vec![11.0, 2.0, 13.0])).unwrap(),
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            indexes.replace(
                &ValueCell::from_exact(DVector::from_vec(vec![4_usize, 2]))?.snapshot()?,
            )?;
            assert!(function.instance().solve_result().is_err());
            super::assert_value_eq(
                &output,
                ValueCell::from_exact(DVector::from_vec(vec![11.0, 2.0, 13.0])).unwrap(),
            );
            output.replace(
                &ValueCell::from_exact(DVector::from_vec(vec![99.0, 98.0]))?.snapshot()?,
            )?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(output.same_cell(&alias));
        super::assert_value_eq(
            &output,
            ValueCell::from_exact(DVector::from_vec(vec![11.0, 2.0, 13.0])).unwrap(),
        );
        assert_eq!(output.shape().parameter_values(), &[3]);
    }
}

#[cfg(all(
    feature = "f64",
    feature = "vectord",
    feature = "add_assign",
    feature = "sub_assign",
    feature = "mul_assign",
    feature = "div_assign",
    feature = "matrix"
))]
mod indexed_all_ops {
    use super::super::{
        add_assign::AddAssign1DRS, div_assign::DivAssign1DRS, mul_assign::MulAssign1DRS,
        sub_assign::SubAssign1DRS,
    };
    use mech_core::{FunctionInvocation, ValueCell};
    use nalgebra::DVector;

    macro_rules! assert_indexed {
        ($factory:ident, $expected:expr) => {{
            let output =
                ValueCell::from_exact(DVector::from_vec(vec![12.0_f64, 12.0, 12.0])).unwrap();
            super::managed_factory_instance::<$factory<f64, DVector<f64>, DVector<usize>>>(
                FunctionInvocation::binary(
                    output.clone(),
                    ValueCell::from_exact(3.0_f64).unwrap(),
                    ValueCell::from_exact(DVector::from_vec(vec![1_usize, 3])).unwrap(),
                ),
                "test/math-indexed-assignment",
            )
            .unwrap()
            .instance()
            .solve_result()
            .unwrap();
            super::assert_value_eq(
                &output,
                ValueCell::from_exact(DVector::from_vec($expected.to_vec())).unwrap(),
            );
        }};
    }

    #[test]
    fn all_indexed_scalar_operation_families_use_canonical_ports() {
        assert_indexed!(AddAssign1DRS, &[15.0, 12.0, 15.0]);
        assert_indexed!(SubAssign1DRS, &[9.0, 12.0, 9.0]);
        assert_indexed!(MulAssign1DRS, &[36.0, 12.0, 36.0]);
        assert_indexed!(DivAssign1DRS, &[4.0, 12.0, 4.0]);
    }
}

#[cfg(all(
    feature = "f64",
    feature = "matrixd",
    feature = "add_assign",
    feature = "matrix"
))]
mod dynamic_whole_matrix {
    use super::super::add_assign::{AddAssignVS, AddAssignVV};
    use mech_core::{FunctionInvocation, MResult, ValueCell, with_reactive_journal_participant};
    use nalgebra::DMatrix;

    #[test]
    fn vector_and_scalar_forms_preserve_binary_base_layout_and_shape_state() {
        let output = ValueCell::from_exact(DMatrix::from_element(2, 2, 10.0_f64)).unwrap();
        let alias = output.clone();
        let source =
            ValueCell::from_exact(DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0])).unwrap();
        let function =
            super::managed_factory_instance::<AddAssignVV<f64, DMatrix<f64>, DMatrix<f64>>>(
                FunctionInvocation::unary(output.clone(), source),
                "test/math-add-assignment",
            )
            .unwrap();
        function.instance().solve_result().unwrap();
        super::assert_value_eq(
            &output,
            ValueCell::from_exact(DMatrix::from_row_slice(2, 2, &[11.0, 12.0, 13.0, 14.0]))
                .unwrap(),
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            output
                .replace(&ValueCell::from_exact(DMatrix::from_element(1, 3, 99.0))?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(output.same_cell(&alias));
        assert_eq!(output.shape().parameter_values(), &[2, 2]);

        let scalar_output = ValueCell::from_exact(DMatrix::from_element(2, 2, 1.0_f64)).unwrap();
        super::managed_factory_instance::<AddAssignVS<f64, DMatrix<f64>>>(
            FunctionInvocation::binary(
                scalar_output.clone(),
                scalar_output.clone(),
                ValueCell::from_exact(2.0_f64).unwrap(),
            ),
            "test/math-add-assignment",
        )
        .unwrap()
        .instance()
        .solve_result()
        .unwrap();
        super::assert_value_eq(
            &scalar_output,
            ValueCell::from_exact(DMatrix::from_element(2, 2, 3.0)).unwrap(),
        );
    }

    #[test]
    fn whole_matrix_assignment_allows_source_output_alias_without_partial_reads() {
        let cell = ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[2.0_f64, 3.0])).unwrap();
        super::managed_factory_instance::<AddAssignVV<f64, DMatrix<f64>, DMatrix<f64>>>(
            FunctionInvocation::unary(cell.clone(), cell.clone()),
            "test/math-add-assignment",
        )
        .unwrap()
        .instance()
        .solve_result()
        .unwrap();
        super::assert_value_eq(
            &cell,
            ValueCell::from_exact(DMatrix::from_row_slice(1, 2, &[4.0, 6.0])).unwrap(),
        );
    }
}

#[cfg(all(
    feature = "f64",
    feature = "bool",
    feature = "vectord",
    feature = "row_vectord",
    feature = "matrixd",
    feature = "add_assign",
    feature = "matrix"
))]
mod indexed_matrix_forms {
    use super::super::add_assign::{AddAssign1DRB, AddAssign1DRVB, AddAssign2DRAVB};
    use mech_core::{FunctionInvocation, ValueCell};
    use nalgebra::{DMatrix, DVector, RowDVector};

    #[test]
    fn boolean_scalar_and_matrix_selection_preserve_layouts() {
        let mask =
            ValueCell::from_exact(DVector::from_vec(vec![true, false, true, false])).unwrap();
        let scalar_sink =
            ValueCell::from_exact(DVector::from_vec(vec![1.0_f64, 2.0, 3.0, 4.0])).unwrap();
        super::managed_factory_instance::<AddAssign1DRB<f64, DVector<f64>, DVector<bool>>>(
            FunctionInvocation::binary(
                scalar_sink.clone(),
                ValueCell::from_exact(10.0_f64).unwrap(),
                mask.clone(),
            ),
            "test/math-add-indexed-assignment",
        )
        .unwrap()
        .instance()
        .solve_result()
        .unwrap();
        super::assert_value_eq(
            &scalar_sink,
            ValueCell::from_exact(DVector::from_vec(vec![11.0, 2.0, 13.0, 4.0])).unwrap(),
        );

        let matrix_sink =
            ValueCell::from_exact(DVector::from_vec(vec![1.0_f64, 2.0, 3.0, 4.0])).unwrap();
        let source =
            ValueCell::from_exact(DVector::from_vec(vec![10.0_f64, 20.0, 30.0, 40.0])).unwrap();
        super::managed_factory_instance::<
            AddAssign1DRVB<f64, DVector<f64>, DVector<f64>, DVector<bool>>,
        >(
            FunctionInvocation::binary(matrix_sink.clone(), source, mask),
            "test/math-add-indexed-assignment",
        )
        .unwrap()
        .instance()
        .solve_result()
        .unwrap();
        super::assert_value_eq(
            &matrix_sink,
            ValueCell::from_exact(DVector::from_vec(vec![11.0, 2.0, 33.0, 4.0])).unwrap(),
        );
    }

    #[test]
    fn row_all_selection_and_oversized_masks_are_atomic() {
        let sink = ValueCell::from_exact(DMatrix::from_row_slice(
            3,
            2,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ))
        .unwrap();
        let source =
            ValueCell::from_exact(DMatrix::from_row_slice(2, 2, &[10.0, 20.0, 30.0, 40.0]))
                .unwrap();
        let mask = ValueCell::from_exact(RowDVector::from_vec(vec![true, false, true])).unwrap();
        super::managed_factory_instance::<
            AddAssign2DRAVB<f64, DMatrix<f64>, DMatrix<f64>, RowDVector<bool>>,
        >(
            FunctionInvocation::binary(sink.clone(), source, mask),
            "test/math-add-row-assignment",
        )
        .unwrap()
        .instance()
        .solve_result()
        .unwrap();
        super::assert_value_eq(
            &sink,
            ValueCell::from_exact(DMatrix::from_row_slice(
                3,
                2,
                &[11.0, 22.0, 3.0, 4.0, 35.0, 46.0],
            ))
            .unwrap(),
        );

        let atomic = ValueCell::from_exact(DVector::from_vec(vec![1.0_f64, 2.0, 3.0])).unwrap();
        let original = atomic.snapshot().unwrap();
        let oversized =
            ValueCell::from_exact(DVector::from_vec(vec![true, false, true, false])).unwrap();
        let function =
            super::managed_factory_instance::<AddAssign1DRB<f64, DVector<f64>, DVector<bool>>>(
                FunctionInvocation::binary(
                    atomic.clone(),
                    ValueCell::from_exact(10.0_f64).unwrap(),
                    oversized,
                ),
                "test/math-add-indexed-assignment",
            )
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        let expected = ValueCell::from_snapshot(original).unwrap();
        super::assert_value_eq(&atomic, expected);
    }
}

#[cfg(all(
    feature = "u8",
    feature = "vectord",
    feature = "add_assign",
    feature = "div_assign",
    feature = "matrix"
))]
mod checked_indexed_integer {
    use super::super::{add_assign::AddAssign1DRV, div_assign::DivAssign1DRS};
    use mech_core::{FunctionInvocation, ValueCell};
    use nalgebra::DVector;

    #[test]
    fn indexed_overflow_and_division_failure_are_atomic() {
        let sink = ValueCell::from_exact(DVector::from_vec(vec![1_u8, u8::MAX])).unwrap();
        let source = ValueCell::from_exact(DVector::from_vec(vec![1_u8, 1])).unwrap();
        let indexes = ValueCell::from_exact(DVector::from_vec(vec![1_usize, 2])).unwrap();
        let function = super::managed_factory_instance::<
            AddAssign1DRV<u8, DVector<u8>, DVector<u8>, DVector<usize>>,
        >(
            FunctionInvocation::binary(sink.clone(), source, indexes.clone()),
            "test/math-add-indexed-assignment",
        )
        .unwrap();
        assert!(function.instance().solve_result().is_err());
        super::assert_value_eq(
            &sink,
            ValueCell::from_exact(DVector::from_vec(vec![1_u8, u8::MAX])).unwrap(),
        );

        let div_sink = ValueCell::from_exact(DVector::from_vec(vec![8_u8, 7])).unwrap();
        let div =
            super::managed_factory_instance::<DivAssign1DRS<u8, DVector<u8>, DVector<usize>>>(
                FunctionInvocation::binary(
                    div_sink.clone(),
                    ValueCell::from_exact(0_u8).unwrap(),
                    indexes,
                ),
                "test/math-div-indexed-assignment",
            )
            .unwrap();
        assert!(div.instance().solve_result().is_err());
        super::assert_value_eq(
            &div_sink,
            ValueCell::from_exact(DVector::from_vec(vec![8_u8, 7])).unwrap(),
        );
    }
}

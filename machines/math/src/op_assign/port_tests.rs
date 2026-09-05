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
            let function = $factory::<f64>::new_invocation(FunctionInvocation::unary(
                sink.clone(),
                ValueCell::from_exact(3.0_f64).unwrap(),
            ))
            .unwrap();
            function.solve_result().unwrap();
            assert_eq!(value(&sink), $expected);
            assert!(sink.same_cell(&alias));
            assert_eq!(
                function.reactive_output_cell_ids(),
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
        let function = AddAssignSS::<f64>::new_invocation(FunctionInvocation::unary(
            sink.clone(),
            ValueCell::from_exact(3.0_f64).unwrap(),
        ))
        .unwrap();
        assert_eq!(function.reactive_node_kind(), ReactiveNodeKind::Register);
        assert_eq!(
            function.transaction_state_ports().unwrap().unwrap().len(),
            1
        );

        let commit = function.stage_register().unwrap();
        assert_eq!(commit.output_cells(), &[sink.reactive_cell_id()]);
        assert_eq!(value(&sink), 2.0);
        commit.commit();
        assert_eq!(value(&sink), 5.0);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
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
    use mech_core::{FunctionInvocation, MechFunctionFactory, Ref, ValueCell};
    use nalgebra::Matrix2;

    #[test]
    fn fixed_matrix_assignment_preserves_exact_backing() {
        let source = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let sink = Ref::new(Matrix2::from_element(10.0_f64));
        let alias = sink.clone();
        let function = AddAssignVV::<f64, Matrix2<f64>, Matrix2<f64>>::new_invocation(
            FunctionInvocation::unary(
                ValueCell::from_exact_matrix_ref(sink.clone(), 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(source, 2, 2).unwrap(),
            ),
        )
        .unwrap();
        function.solve_result().unwrap();
        assert!(sink.same_handle(&alias));
        assert_eq!(*sink.borrow(), Matrix2::new(11.0, 12.0, 13.0, 14.0));
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
    use mech_core::{
        FunctionInvocation, MResult, MechFunctionFactory, Ref, ValueCell,
        with_reactive_journal_participant,
    };
    use nalgebra::DVector;

    #[test]
    fn indexed_assignment_is_atomic_and_checkpointed() {
        let sink = Ref::new(DVector::from_vec(vec![1.0_f64, 2.0, 3.0]));
        let alias = sink.clone();
        let output = ValueCell::from_exact_matrix_ref(sink.clone(), 3, 1).unwrap();
        let indexes = Ref::new(DVector::from_vec(vec![1_usize, 3]));
        let function = AddAssign1DRS::<f64, DVector<f64>, DVector<usize>>::new_invocation(
            FunctionInvocation::binary(
                output.clone(),
                ValueCell::from_exact(10.0_f64).unwrap(),
                ValueCell::from_exact_matrix_ref(indexes.clone(), 2, 1).unwrap(),
            ),
        )
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(sink.borrow().as_slice(), &[11.0, 2.0, 13.0]);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            *indexes.borrow_mut() = DVector::from_vec(vec![0, 2]);
            assert!(function.solve_result().is_err());
            assert_eq!(sink.borrow().as_slice(), &[11.0, 2.0, 13.0]);
            *sink.borrow_mut() = DVector::from_vec(vec![99.0, 98.0]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(sink.same_handle(&alias));
        assert_eq!(sink.borrow().as_slice(), &[11.0, 2.0, 13.0]);
        assert_eq!(output.shape().parameter_values(), &[3, 1]);
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
    use mech_core::{FunctionInvocation, MechFunctionFactory, Ref, ValueCell};
    use nalgebra::DVector;

    macro_rules! assert_indexed {
        ($factory:ident, $expected:expr) => {{
            let sink = Ref::new(DVector::from_vec(vec![12.0_f64, 12.0, 12.0]));
            $factory::<f64, DVector<f64>, DVector<usize>>::new_invocation(
                FunctionInvocation::binary(
                    ValueCell::from_exact_matrix_ref(sink.clone(), 3, 1).unwrap(),
                    ValueCell::from_exact(3.0_f64).unwrap(),
                    ValueCell::from_exact_matrix_ref(
                        Ref::new(DVector::from_vec(vec![1_usize, 3])),
                        2,
                        1,
                    )
                    .unwrap(),
                ),
            )
            .unwrap()
            .solve_result()
            .unwrap();
            assert_eq!(sink.borrow().as_slice(), $expected);
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
    use mech_core::{
        FunctionInvocation, MResult, MechFunctionFactory, Ref, ValueCell,
        with_reactive_journal_participant,
    };
    use nalgebra::DMatrix;

    #[test]
    fn vector_and_scalar_forms_preserve_binary_base_layout_and_shape_state() {
        let sink = Ref::new(DMatrix::from_element(2, 2, 10.0_f64));
        let alias = sink.clone();
        let output = ValueCell::from_exact_matrix_ref(sink.clone(), 2, 2).unwrap();
        let source = Ref::new(DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0]));
        let function = AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new_invocation(
            FunctionInvocation::unary(
                output.clone(),
                ValueCell::from_exact_matrix_ref(source, 2, 2).unwrap(),
            ),
        )
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(
            *sink.borrow(),
            DMatrix::from_row_slice(2, 2, &[11.0, 12.0, 13.0, 14.0])
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            *sink.borrow_mut() = DMatrix::from_element(1, 3, 99.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(sink.same_handle(&alias));
        assert_eq!(sink.borrow().shape(), (2, 2));

        let scalar_sink = Ref::new(DMatrix::from_element(2, 2, 1.0_f64));
        let scalar_output = ValueCell::from_exact_matrix_ref(scalar_sink.clone(), 2, 2).unwrap();
        AddAssignVS::<f64, DMatrix<f64>>::new_invocation(FunctionInvocation::binary(
            scalar_output.clone(),
            scalar_output,
            ValueCell::from_exact(2.0_f64).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*scalar_sink.borrow(), DMatrix::from_element(2, 2, 3.0));
    }

    #[test]
    fn whole_matrix_assignment_allows_source_output_alias_without_partial_reads() {
        let sink = Ref::new(DMatrix::from_row_slice(1, 2, &[2.0_f64, 3.0]));
        let cell = ValueCell::from_exact_matrix_ref(sink.clone(), 1, 2).unwrap();
        AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new_invocation(FunctionInvocation::unary(
            cell.clone(),
            cell,
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*sink.borrow(), DMatrix::from_row_slice(1, 2, &[4.0, 6.0]));
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
    use mech_core::{FunctionInvocation, MechFunctionFactory, Ref, ValueCell};
    use nalgebra::{DMatrix, DVector, RowDVector};

    #[test]
    fn boolean_scalar_and_matrix_selection_preserve_layouts() {
        let mask = Ref::new(DVector::from_vec(vec![true, false, true, false]));
        let scalar_sink = Ref::new(DVector::from_vec(vec![1.0_f64, 2.0, 3.0, 4.0]));
        AddAssign1DRB::<f64, DVector<f64>, DVector<bool>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(scalar_sink.clone(), 4, 1).unwrap(),
                ValueCell::from_exact(10.0_f64).unwrap(),
                ValueCell::from_exact_matrix_ref(mask.clone(), 4, 1).unwrap(),
            ),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(scalar_sink.borrow().as_slice(), &[11.0, 2.0, 13.0, 4.0]);

        let matrix_sink = Ref::new(DVector::from_vec(vec![1.0_f64, 2.0, 3.0, 4.0]));
        let source = Ref::new(DVector::from_vec(vec![10.0_f64, 20.0, 30.0, 40.0]));
        AddAssign1DRVB::<f64, DVector<f64>, DVector<f64>, DVector<bool>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(matrix_sink.clone(), 4, 1).unwrap(),
                ValueCell::from_exact_matrix_ref(source, 4, 1).unwrap(),
                ValueCell::from_exact_matrix_ref(mask, 4, 1).unwrap(),
            ),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(matrix_sink.borrow().as_slice(), &[11.0, 2.0, 33.0, 4.0]);
    }

    #[test]
    fn row_all_selection_and_oversized_masks_are_atomic() {
        let sink = Ref::new(DMatrix::from_row_slice(
            3,
            2,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let source = Ref::new(DMatrix::from_row_slice(2, 2, &[10.0_f64, 20.0, 30.0, 40.0]));
        let mask = Ref::new(RowDVector::from_vec(vec![true, false, true]));
        AddAssign2DRAVB::<f64, DMatrix<f64>, DMatrix<f64>, RowDVector<bool>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(sink.clone(), 3, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(source, 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(mask, 1, 3).unwrap(),
            ),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *sink.borrow(),
            DMatrix::from_row_slice(3, 2, &[11.0, 22.0, 3.0, 4.0, 35.0, 46.0])
        );

        let atomic = Ref::new(DVector::from_vec(vec![1.0_f64, 2.0, 3.0]));
        let original = atomic.borrow().clone();
        let oversized = Ref::new(DVector::from_vec(vec![true, false, true, false]));
        let function = AddAssign1DRB::<f64, DVector<f64>, DVector<bool>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(atomic.clone(), 3, 1).unwrap(),
                ValueCell::from_exact(10.0_f64).unwrap(),
                ValueCell::from_exact_matrix_ref(oversized, 4, 1).unwrap(),
            ),
        )
        .unwrap();
        assert!(function.solve_result().is_err());
        assert_eq!(*atomic.borrow(), original);
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
    use mech_core::{FunctionInvocation, MechFunctionFactory, Ref, ValueCell};
    use nalgebra::DVector;

    #[test]
    fn indexed_overflow_and_division_failure_are_atomic() {
        let sink = Ref::new(DVector::from_vec(vec![1_u8, u8::MAX]));
        let source = Ref::new(DVector::from_vec(vec![1_u8, 1]));
        let indexes = Ref::new(DVector::from_vec(vec![1_usize, 2]));
        let function =
            AddAssign1DRV::<u8, DVector<u8>, DVector<u8>, DVector<usize>>::new_invocation(
                FunctionInvocation::binary(
                    ValueCell::from_exact_matrix_ref(sink.clone(), 2, 1).unwrap(),
                    ValueCell::from_exact_matrix_ref(source, 2, 1).unwrap(),
                    ValueCell::from_exact_matrix_ref(indexes.clone(), 2, 1).unwrap(),
                ),
            )
            .unwrap();
        assert!(function.solve_result().is_err());
        assert_eq!(sink.borrow().as_slice(), &[1, u8::MAX]);

        let div_sink = Ref::new(DVector::from_vec(vec![8_u8, 7]));
        let div = DivAssign1DRS::<u8, DVector<u8>, DVector<usize>>::new_invocation(
            FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(div_sink.clone(), 2, 1).unwrap(),
                ValueCell::from_exact(0_u8).unwrap(),
                ValueCell::from_exact_matrix_ref(indexes, 2, 1).unwrap(),
            ),
        )
        .unwrap();
        assert!(div.solve_result().is_err());
        assert_eq!(div_sink.borrow().as_slice(), &[8, 7]);
    }
}

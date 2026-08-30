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
            assert_eq!(function.reactive_output_cell_ids(), vec![sink.reactive_cell_id()]);
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
        assert_eq!(function.transaction_state_ports().unwrap().unwrap().len(), 1);

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
        let function =
            AddAssign1DRS::<f64, DVector<f64>, DVector<usize>>::new_invocation(
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

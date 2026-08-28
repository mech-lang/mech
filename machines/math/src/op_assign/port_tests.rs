#[cfg(all(
    feature = "f64",
    feature = "add_assign",
    feature = "sub_assign",
    feature = "mul_assign",
    feature = "div_assign"
))]
mod scalar_f64 {
    use super::super::{AddAssignSS, DivAssignSS, MulAssignSS, SubAssignSS};
    use mech_core::{
        FunctionArgs, FunctionInvocation, IncorrectNumberOfArguments, MechError, MechFunction,
        MechFunctionFactory, ReactiveNodeKind, Ref, ToValue, with_reactive_journal_participant,
    };

    fn factory_error(result: Result<Box<dyn MechFunction>, MechError>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    macro_rules! assert_scalar_factories_match {
        ($factory:ident, $initial:expr, $source:expr, $expected:expr) => {{
            let source = Ref::new($source);
            let legacy_sink = Ref::new($initial);
            let invocation_sink = Ref::new($initial);
            let legacy = $factory::<f64>::new(FunctionArgs::Unary(
                legacy_sink.to_value(),
                source.to_value(),
            ))
            .unwrap();
            let invocation = $factory::<f64>::new_invocation(
                FunctionArgs::Unary(invocation_sink.to_value(), source.to_value()).into(),
            )
            .unwrap();

            legacy.solve_result().unwrap();
            invocation.solve_result().unwrap();

            assert_eq!(*legacy_sink.borrow(), $expected);
            assert_eq!(*invocation_sink.borrow(), $expected);
            assert_eq!(
                legacy.reactive_output_cell_ids(),
                legacy.out().reactive_root_cell_ids(),
            );
            assert_eq!(
                invocation.reactive_output_cell_ids(),
                invocation.out().reactive_root_cell_ids(),
            );
        }};
    }

    #[test]
    fn scalar_legacy_and_invocation_factories_are_equivalent() {
        assert_scalar_factories_match!(AddAssignSS, 12.0, 3.0, 15.0);
        assert_scalar_factories_match!(SubAssignSS, 12.0, 3.0, 9.0);
        assert_scalar_factories_match!(MulAssignSS, 12.0, 3.0, 36.0);
        assert_scalar_factories_match!(DivAssignSS, 12.0, 3.0, 4.0);
    }

    #[test]
    fn scalar_ports_are_exact_and_unary_layout_reports_one_input() {
        let sink = Ref::new(4.0_f64);
        let source = Ref::new(2.0_f64);

        assert!(
            AddAssignSS::<f64>::new_invocation(
                FunctionArgs::Unary(sink.to_value(), Ref::new(2_usize).to_value()).into(),
            )
            .is_err()
        );
        assert!(
            AddAssignSS::<f64>::new_invocation(
                FunctionArgs::Unary(Ref::new(0_usize).to_value(), source.to_value()).into(),
            )
            .is_err()
        );

        let error = factory_error(AddAssignSS::<f64>::new_invocation(
            FunctionInvocation::from(FunctionArgs::Binary(
                sink.to_value(),
                source.to_value(),
                source.to_value(),
            )),
        ));
        let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((arity.expected, arity.found), (1, 2));
    }

    #[test]
    fn whole_value_alias_staging_and_typed_checkpoint_preserve_one_sink() {
        let sink = Ref::new(2.0_f64);
        let sink_alias = sink.clone();
        let function = AddAssignSS::<f64>::new_invocation(
            FunctionArgs::Unary(sink.to_value(), sink.to_value()).into(),
        )
        .unwrap();

        assert_eq!(function.reactive_node_kind(), ReactiveNodeKind::Register);
        assert_eq!(
            function.transaction_state_ports().unwrap().unwrap().len(),
            1
        );
        let output_cells = function.reactive_output_cell_ids();
        assert_eq!(output_cells, function.out().reactive_root_cell_ids());

        let staged = function.stage_register().unwrap();
        assert_eq!(staged.output_cells(), output_cells.as_slice());
        assert_eq!(*sink.borrow(), 2.0);
        staged.commit();
        assert_eq!(*sink.borrow(), 4.0);

        function.solve_result().unwrap();
        assert_eq!(*sink.borrow(), 8.0);
        let successful = *sink.borrow();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *sink.borrow_mut() = 99.0;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(sink.same_handle(&sink_alias));
        assert_eq!(*sink.borrow(), successful);
        assert_eq!(function.reactive_output_cell_ids(), output_cells);
    }

    #[test]
    fn staged_assignment_matches_direct_solving_and_reports_the_same_output() {
        let source = Ref::new(3.0_f64);
        let sink = Ref::new(2.0_f64);
        let direct = AddAssignSS::<f64>::new_invocation(
            FunctionArgs::Unary(sink.to_value(), source.to_value()).into(),
        )
        .unwrap();
        let staged = AddAssignSS::<f64>::new_invocation(
            FunctionArgs::Unary(sink.to_value(), source.to_value()).into(),
        )
        .unwrap();

        let staged_cells = staged.reactive_output_cell_ids();
        assert_eq!(direct.reactive_output_cell_ids(), staged_cells);
        assert_eq!(staged_cells, staged.out().reactive_root_cell_ids());
        direct.solve_result().unwrap();
        let direct_result = *sink.borrow();
        *sink.borrow_mut() = 2.0;

        let commit = staged.stage_register().unwrap();
        assert_eq!(commit.output_cells(), staged_cells.as_slice());
        assert_eq!(*sink.borrow(), 2.0);
        commit.commit();
        assert_eq!(*sink.borrow(), direct_result);
        assert_eq!(staged.reactive_output_cell_ids(), staged_cells);
    }

    #[cfg(feature = "semantic-compiler")]
    fn decoded_scalar_wrapper(reference: bool) -> mech_core::LegacyValue {
        use mech_core::{EncodedConstant, RuntimeType, decode_encoded_constants};

        fn framed(payload: &[u8]) -> Vec<u8> {
            let mut framed = (payload.len() as u32).to_le_bytes().to_vec();
            framed.extend_from_slice(payload);
            framed
        }

        let payload = 1.0_f64.to_le_bytes();
        let (runtime_type, bytes) = if reference {
            (
                RuntimeType::Reference(Box::new(RuntimeType::F64)),
                framed(&payload),
            )
        } else {
            let mut bytes = vec![1];
            bytes.extend(framed(&payload));
            (RuntimeType::Option(Box::new(RuntimeType::F64)), bytes)
        };
        decode_encoded_constants(&[EncodedConstant {
            runtime_type,
            alignment: 8,
            bytes,
        }])
        .unwrap()
        .pop()
        .unwrap()
    }

    #[cfg(feature = "semantic-compiler")]
    #[test]
    fn scalar_ports_reject_typed_and_mutable_reference_inputs() {
        let sink = Ref::new(4.0_f64);
        for wrapped in [decoded_scalar_wrapper(false), decoded_scalar_wrapper(true)] {
            assert!(
                AddAssignSS::<f64>::new_invocation(
                    FunctionArgs::Unary(sink.to_value(), wrapped).into(),
                )
                .is_err()
            );
        }
    }
}

#[cfg(all(feature = "f64", feature = "matrix2", feature = "add_assign"))]
mod fixed_matrix_f64 {
    use super::super::AddAssignVV;
    use mech_core::{FunctionArgs, MechFunctionFactory, Ref, ToValue};
    use nalgebra::Matrix2;

    #[test]
    fn fixed_matrix_whole_value_assignment_uses_exact_ports() {
        let source = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let sink = Ref::new(Matrix2::from_element(10.0_f64));
        let sink_alias = sink.clone();
        let function = AddAssignVV::<f64, Matrix2<f64>, Matrix2<f64>>::new_invocation(
            FunctionArgs::Unary(sink.to_value(), source.to_value()).into(),
        )
        .unwrap();

        function.solve_result().unwrap();

        assert!(sink.same_handle(&sink_alias));
        assert_eq!(*sink.borrow(), Matrix2::new(11.0_f64, 12.0, 13.0, 14.0),);
        assert_eq!(
            function.reactive_output_cell_ids(),
            function.out().reactive_root_cell_ids(),
        );
    }
}

#[cfg(all(
    feature = "f64",
    feature = "matrixd",
    feature = "vectord",
    feature = "add_assign"
))]
mod dynamic_matrix_f64 {
    use super::super::{AddAssignVS, AddAssignVV};
    use mech_core::{
        FunctionArgs, FunctionInvocation, IncorrectNumberOfArguments, MechError, MechFunction,
        MechFunctionFactory, Ref, ToValue, with_reactive_journal_participant,
    };
    use nalgebra::{DMatrix, DVector};

    fn factory_error(result: Result<Box<dyn MechFunction>, MechError>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    #[test]
    fn dynamic_matrix_whole_value_factories_and_staging_preserve_exact_state() {
        let source = Ref::new(DMatrix::from_row_slice(2, 2, &[1.0_f64, 2.0, 3.0, 4.0]));
        let legacy_sink = Ref::new(DMatrix::from_element(2, 2, 10.0_f64));
        let invocation_sink = Ref::new(DMatrix::from_element(2, 2, 10.0_f64));
        let invocation_alias = invocation_sink.clone();
        let legacy = AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new(FunctionArgs::Unary(
            legacy_sink.to_value(),
            source.to_value(),
        ))
        .unwrap();
        let invocation = AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new_invocation(
            FunctionArgs::Unary(invocation_sink.to_value(), source.to_value()).into(),
        )
        .unwrap();

        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_sink.borrow(), *invocation_sink.borrow());
        assert_eq!(
            *invocation_sink.borrow(),
            DMatrix::from_row_slice(2, 2, &[11.0_f64, 12.0, 13.0, 14.0]),
        );
        assert_eq!(
            invocation.transaction_state_ports().unwrap().unwrap().len(),
            1
        );

        let output_cells = invocation.reactive_output_cell_ids();
        assert_eq!(output_cells, invocation.out().reactive_root_cell_ids());
        let before_stage = invocation_sink.borrow().clone();
        let staged = invocation.stage_register().unwrap();
        assert_eq!(staged.output_cells(), output_cells.as_slice());
        assert_eq!(*invocation_sink.borrow(), before_stage);
        staged.commit();
        assert_eq!(
            *invocation_sink.borrow(),
            DMatrix::from_row_slice(2, 2, &[12.0_f64, 14.0, 16.0, 18.0]),
        );
        let successful = invocation_sink.borrow().clone();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*invocation)?;
            *invocation_sink.borrow_mut() = DMatrix::from_element(1, 3, 99.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(invocation_sink.same_handle(&invocation_alias));
        assert_eq!(*invocation_sink.borrow(), successful);
    }

    #[test]
    fn matrix_scalar_assignment_keeps_binary_base_layout_and_stages_exact_sink() {
        let sink = Ref::new(DMatrix::from_row_slice(1, 3, &[1.0_f64, 2.0, 3.0]));
        let sink_alias = sink.clone();
        let function = AddAssignVS::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Binary(
                sink.to_value(),
                sink.to_value(),
                Ref::new(2.0_f64).to_value(),
            )
            .into(),
        )
        .unwrap();

        let output_cells = function.reactive_output_cell_ids();
        let staged = function.stage_register().unwrap();
        assert_eq!(staged.output_cells(), output_cells.as_slice());
        assert_eq!(sink.borrow().as_slice(), &[1.0, 2.0, 3.0]);
        staged.commit();

        assert!(sink.same_handle(&sink_alias));
        assert_eq!(sink.borrow().as_slice(), &[3.0, 4.0, 5.0]);
        assert_eq!(
            function.transaction_state_ports().unwrap().unwrap().len(),
            1
        );

        let error = factory_error(AddAssignVS::<f64, DMatrix<f64>>::new_invocation(
            FunctionInvocation::from(FunctionArgs::Unary(
                sink.to_value(),
                Ref::new(1.0_f64).to_value(),
            )),
        ));
        let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((arity.expected, arity.found), (2, 1));
    }

    #[test]
    fn matrix_output_source_aliasing_remains_supported() {
        let sink = Ref::new(DMatrix::from_row_slice(1, 3, &[1.0_f64, 2.0, 3.0]));
        let sink_alias = sink.clone();
        let function = AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new_invocation(
            FunctionArgs::Unary(sink.to_value(), sink.to_value()).into(),
        )
        .unwrap();

        function.solve_result().unwrap();

        assert!(sink.same_handle(&sink_alias));
        assert_eq!(sink.borrow().as_slice(), &[2.0, 4.0, 6.0]);
    }

    #[test]
    fn matrix_whole_value_ports_reject_storage_mismatches_and_report_unary_arity() {
        let sink = Ref::new(DMatrix::from_element(1, 2, 1.0_f64));
        let source = Ref::new(DMatrix::from_element(1, 2, 2.0_f64));
        let vector = Ref::new(DVector::from_vec(vec![1.0_f64, 2.0]));

        assert!(
            AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new_invocation(
                FunctionArgs::Unary(sink.to_value(), vector.to_value()).into(),
            )
            .is_err()
        );
        assert!(
            AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new_invocation(
                FunctionArgs::Unary(vector.to_value(), source.to_value()).into(),
            )
            .is_err()
        );

        let error = factory_error(
            AddAssignVV::<f64, DMatrix<f64>, DMatrix<f64>>::new_invocation(
                FunctionInvocation::from(FunctionArgs::Binary(
                    sink.to_value(),
                    source.to_value(),
                    source.to_value(),
                )),
            ),
        );
        let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((arity.expected, arity.found), (1, 2));
    }
}

#[cfg(all(feature = "u8", feature = "add_assign", feature = "div_assign"))]
mod checked_integer {
    use super::super::{AddAssignSS, DivAssignSS};
    use mech_core::{
        FunctionArgs, MechFunctionFactory, Ref, ToValue, with_reactive_journal_participant,
    };

    #[test]
    fn overflow_is_atomic_and_prior_successful_state_remains_restorable() {
        let sink = Ref::new(40_u8);
        let sink_alias = sink.clone();
        let source = Ref::new(1_u8);
        let function = AddAssignSS::<u8>::new_invocation(
            FunctionArgs::Unary(sink.to_value(), source.to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(*sink.borrow(), 41);

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *source.borrow_mut() = u8::MAX;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MathArithmeticOverflow");
            assert_eq!(*sink.borrow(), 41);
            let error = match function.stage_register() {
                Ok(_) => panic!("overflowing assignment unexpectedly staged"),
                Err(error) => error,
            };
            assert_eq!(error.kind_name(), "MathArithmeticOverflow");
            assert_eq!(*sink.borrow(), 41);
            *sink.borrow_mut() = 99;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(sink.same_handle(&sink_alias));
        assert_eq!(*sink.borrow(), 41);
    }

    #[test]
    fn integer_division_failure_leaves_the_sink_unchanged() {
        let sink = Ref::new(7_u8);
        let function = DivAssignSS::<u8>::new_invocation(
            FunctionArgs::Unary(sink.to_value(), Ref::new(0_u8).to_value()).into(),
        )
        .unwrap();

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(*sink.borrow(), 7);
        let error = match function.stage_register() {
            Ok(_) => panic!("invalid division unexpectedly staged"),
            Err(error) => error,
        };
        assert_eq!(error.kind_name(), "MathArithmeticOverflow");
        assert_eq!(*sink.borrow(), 7);
    }
}

#[cfg(all(
    feature = "f64",
    feature = "matrixd",
    feature = "row_vectord",
    feature = "inclusive",
    feature = "exclusive",
    feature = "inclusive_increment",
    feature = "exclusive_increment"
))]
mod dynamic_ranges {
    use crate::{
        RangeExclusiveScalar, RangeIncrementExclusiveScalar, RangeIncrementInclusiveScalar,
        RangeInclusiveScalar,
    };
    use mech_core::{
        EncodedConstant, FunctionArgs, FunctionArgumentRole, FunctionInvocation,
        IncorrectNumberOfArguments, LegacyValue, MechError, MechFunction, MechFunctionFactory,
        Ref, RuntimeType, ToValue, decode_encoded_constants, with_reactive_journal_participant,
    };
    use nalgebra::{DMatrix, RowDVector};

    fn output(len: usize) -> Ref<DMatrix<f64>> {
        Ref::new(DMatrix::from_element(1, len, 0.0))
    }

    fn factory_error(result: Result<Box<dyn MechFunction>, MechError>) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    fn framed(payload: &[u8]) -> Vec<u8> {
        let mut framed = (payload.len() as u32).to_le_bytes().to_vec();
        framed.extend_from_slice(payload);
        framed
    }

    fn decoded_scalar_wrapper(reference: bool) -> LegacyValue {
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

    fn assert_supplied_output(function: &dyn MechFunction, out: &Ref<DMatrix<f64>>) {
        assert_eq!(
            function.reactive_output_cell_ids(),
            out.to_value().reactive_root_cell_ids(),
        );
        let projected = function
            .out()
            .try_function_matrix::<f64>(FunctionArgumentRole::Output)
            .unwrap();
        let mech_core::matrix::Matrix::DMatrix(projected) = projected else {
            panic!("range output changed its dynamic matrix representation")
        };
        assert!(projected.same_handle(out));
    }

    #[test]
    fn all_range_factories_match_the_legacy_adapters() {
        let from = Ref::new(1.0_f64);
        let to = Ref::new(3.0_f64);
        let legacy_out = output(3);
        let invocation_out = output(3);
        let legacy = RangeInclusiveScalar::<f64, DMatrix<f64>>::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            from.to_value(),
            to.to_value(),
        ))
        .unwrap();
        let invocation = RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Binary(
                invocation_out.to_value(),
                from.to_value(),
                to.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        assert_supplied_output(&*legacy, &legacy_out);
        assert_supplied_output(&*invocation, &invocation_out);

        let exclusive_to = Ref::new(4.0_f64);
        let legacy_out = output(3);
        let invocation_out = output(3);
        let legacy = RangeExclusiveScalar::<f64, DMatrix<f64>>::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            from.to_value(),
            exclusive_to.to_value(),
        ))
        .unwrap();
        let invocation = RangeExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Binary(
                invocation_out.to_value(),
                from.to_value(),
                exclusive_to.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        assert_supplied_output(&*legacy, &legacy_out);
        assert_supplied_output(&*invocation, &invocation_out);

        let step = Ref::new(2.0_f64);
        let inclusive_to = Ref::new(5.0_f64);
        let legacy_out = output(3);
        let invocation_out = output(3);
        let legacy =
            RangeIncrementInclusiveScalar::<f64, DMatrix<f64>>::new(FunctionArgs::Ternary(
                legacy_out.to_value(),
                from.to_value(),
                step.to_value(),
                inclusive_to.to_value(),
            ))
            .unwrap();
        let invocation = RangeIncrementInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Ternary(
                invocation_out.to_value(),
                from.to_value(),
                step.to_value(),
                inclusive_to.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        assert_supplied_output(&*legacy, &legacy_out);
        assert_supplied_output(&*invocation, &invocation_out);

        let exclusive_to = Ref::new(7.0_f64);
        let legacy_out = output(3);
        let invocation_out = output(3);
        let legacy =
            RangeIncrementExclusiveScalar::<f64, DMatrix<f64>>::new(FunctionArgs::Ternary(
                legacy_out.to_value(),
                from.to_value(),
                step.to_value(),
                exclusive_to.to_value(),
            ))
            .unwrap();
        let invocation = RangeIncrementExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Ternary(
                invocation_out.to_value(),
                from.to_value(),
                step.to_value(),
                exclusive_to.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        assert_supplied_output(&*legacy, &legacy_out);
        assert_supplied_output(&*invocation, &invocation_out);
    }

    #[test]
    fn range_ports_reject_wrong_types_storage_and_wrappers() {
        let out = output(3);
        let to = Ref::new(3.0_f64);
        assert!(
            RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionArgs::Binary(
                    out.to_value(),
                    Ref::new(1_usize).to_value(),
                    to.to_value(),
                )
                .into(),
            )
            .is_err()
        );

        let wrong_out = Ref::new(RowDVector::from_element(3, 0.0_f64));
        assert!(
            RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionArgs::Binary(
                    wrong_out.to_value(),
                    Ref::new(1.0_f64).to_value(),
                    to.to_value(),
                )
                .into(),
            )
            .is_err()
        );

        for wrapped in [decoded_scalar_wrapper(false), decoded_scalar_wrapper(true)] {
            assert!(
                RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                    FunctionArgs::Binary(out.to_value(), wrapped, to.to_value()).into(),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn range_layout_errors_report_the_correct_arities() {
        let out = output(1);
        let scalar = Ref::new(1.0_f64);
        for error in [
            factory_error(RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::from(FunctionArgs::Unary(
                    out.to_value(),
                    scalar.to_value(),
                )),
            )),
            factory_error(RangeExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                FunctionInvocation::from(FunctionArgs::Unary(
                    out.to_value(),
                    scalar.to_value(),
                )),
            )),
        ] {
            let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
            assert_eq!((arity.expected, arity.found), (2, 1));
        }

        for error in [
            factory_error(
                RangeIncrementInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                    FunctionInvocation::from(FunctionArgs::Binary(
                        out.to_value(),
                        scalar.to_value(),
                        scalar.to_value(),
                    )),
                ),
            ),
            factory_error(
                RangeIncrementExclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
                    FunctionInvocation::from(FunctionArgs::Binary(
                        out.to_value(),
                        scalar.to_value(),
                        scalar.to_value(),
                    )),
                ),
            ),
        ] {
            let arity = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
            assert_eq!((arity.expected, arity.found), (3, 2));
        }
    }

    #[test]
    fn binary_and_incremented_outputs_restore_exact_state() {
        let binary_out = output(3);
        let binary_alias = binary_out.clone();
        let binary = RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Binary(
                binary_out.to_value(),
                Ref::new(1.0_f64).to_value(),
                Ref::new(3.0_f64).to_value(),
            )
            .into(),
        )
        .unwrap();
        let stepped_out = output(3);
        let stepped_alias = stepped_out.clone();
        let stepped = RangeIncrementInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Ternary(
                stepped_out.to_value(),
                Ref::new(1.0_f64).to_value(),
                Ref::new(2.0_f64).to_value(),
                Ref::new(5.0_f64).to_value(),
            )
            .into(),
        )
        .unwrap();
        binary.solve_result().unwrap();
        stepped.solve_result().unwrap();
        let binary_before = binary_out.borrow().clone();
        let stepped_before = stepped_out.borrow().clone();
        assert_eq!(
            binary.reactive_output_cell_ids(),
            binary.out().reactive_root_cell_ids(),
        );
        assert_eq!(
            stepped.reactive_output_cell_ids(),
            stepped.out().reactive_root_cell_ids(),
        );

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*binary)?;
            participant.capture_function_state(&*stepped)?;
            *binary_out.borrow_mut() = DMatrix::from_element(2, 2, 99.0);
            *stepped_out.borrow_mut() = DMatrix::from_element(2, 1, 88.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(binary_out.same_handle(&binary_alias));
        assert!(stepped_out.same_handle(&stepped_alias));
        assert_eq!(*binary_out.borrow(), binary_before);
        assert_eq!(*stepped_out.borrow(), stepped_before);
    }

    #[test]
    fn range_shape_failure_is_atomic_and_prior_state_remains_restorable() {
        let to = Ref::new(3.0_f64);
        let out = output(3);
        let out_alias = out.clone();
        let function = RangeInclusiveScalar::<f64, DMatrix<f64>>::new_invocation(
            FunctionArgs::Binary(
                out.to_value(),
                Ref::new(1.0_f64).to_value(),
                to.to_value(),
            )
            .into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let successful = out.borrow().clone();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *to.borrow_mut() = 4.0;
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "FunctionShapeContractViolation");
            assert_eq!(*out.borrow(), successful);
            *out.borrow_mut() = DMatrix::from_element(2, 2, -1.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(out.same_handle(&out_alias));
        assert_eq!(*out.borrow(), successful);
    }
}

#[cfg(all(feature = "f64", feature = "row_vector2", feature = "inclusive"))]
mod fixed_range {
    use crate::RangeInclusiveScalar;
    use mech_core::{FunctionArgs, FunctionArgumentRole, MechFunctionFactory, Ref, ToValue};
    use nalgebra::RowVector2;

    #[test]
    fn fixed_row_vector_output_retains_exact_representation_and_handle() {
        let out = Ref::new(RowVector2::zeros());
        let function = RangeInclusiveScalar::<f64, RowVector2<f64>>::new_invocation(
            FunctionArgs::Binary(
                out.to_value(),
                Ref::new(1.0_f64).to_value(),
                Ref::new(2.0_f64).to_value(),
            )
            .into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(out.borrow().as_slice(), &[1.0, 2.0]);

        let projected = function
            .out()
            .try_function_matrix::<f64>(FunctionArgumentRole::Output)
            .unwrap();
        let mech_core::matrix::Matrix::RowVector2(projected) = projected else {
            panic!("fixed range output changed its representation")
        };
        assert!(projected.same_handle(&out));
    }
}

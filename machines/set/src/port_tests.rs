#[cfg(all(
    feature = "union",
    feature = "difference",
    feature = "intersection",
    feature = "powerset",
    feature = "cartesian_product",
    feature = "subset",
    feature = "equals",
    feature = "size",
    feature = "bool",
    feature = "u64"
))]
mod typed_factories {
    use crate::{
        operations::{
            cartesian_product::SetCartesianProductFxn, difference::SetDifferenceFxn,
            intersection::SetIntersectionFxn, powerset::SetPowersetFxn, union::SetUnionFxn,
        },
        relations::{equals::SetEqualsFxn, subset::SetSubsetFxn},
        setdata::size::SetSizeFxn,
    };
    use mech_core::{
        EncodedConstant, FunctionArgs, FunctionInvocation, IncorrectNumberOfArguments,
        LegacyValue, MechError, MechFunctionFactory, MechSet, Ref, RuntimeType, ToValue,
        decode_encoded_constants,
    };

    fn set(values: &[usize]) -> Ref<MechSet> {
        Ref::new(MechSet::from_vec(
            values
                .iter()
                .copied()
                .map(|value| Ref::new(value).to_value())
                .collect(),
        ))
    }

    fn set_output() -> Ref<MechSet> {
        Ref::new(MechSet::from_vec(Vec::new()))
    }

    fn framed(payload: &[u8]) -> Vec<u8> {
        let mut framed = (payload.len() as u32).to_le_bytes().to_vec();
        framed.extend_from_slice(payload);
        framed
    }

    fn decoded_set_wrapper(reference: bool) -> LegacyValue {
        let set_type = RuntimeType::Set {
            element: Box::new(RuntimeType::Index),
            max_len: Some(1),
        };
        let set_payload = 0_u32.to_le_bytes();
        let (runtime_type, bytes) = if reference {
            (RuntimeType::Reference(Box::new(set_type)), framed(&set_payload))
        } else {
            let mut bytes = vec![1];
            bytes.extend(framed(&set_payload));
            (RuntimeType::Option(Box::new(set_type)), bytes)
        };
        decode_encoded_constants(&[EncodedConstant {
            runtime_type,
            alignment: 4,
            bytes,
        }])
        .unwrap()
        .pop()
        .unwrap()
    }

    fn factory_error(
        result: Result<Box<dyn mech_core::MechFunction>, MechError>,
    ) -> MechError {
        match result {
            Ok(_) => panic!("factory unexpectedly accepted invalid arguments"),
            Err(error) => error,
        }
    }

    fn assert_binary_set_factories_are_equivalent<F>()
    where
        F: MechFunctionFactory,
    {
        let lhs = set(&[1, 2]);
        let rhs = set(&[2, 3]);
        let legacy_out = set_output();
        let invocation_out = set_output();

        let legacy = F::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            lhs.to_value(),
            rhs.to_value(),
        ))
        .unwrap();
        let invocation = F::new_invocation(
            FunctionArgs::Binary(
                invocation_out.to_value(),
                lhs.to_value(),
                rhs.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();

        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }

    fn assert_binary_bool_factories_are_equivalent<F>()
    where
        F: MechFunctionFactory,
    {
        let lhs = set(&[1, 2]);
        let rhs = set(&[1, 2, 3]);
        let legacy_out = Ref::new(false);
        let invocation_out = Ref::new(false);

        let legacy = F::new(FunctionArgs::Binary(
            legacy_out.to_value(),
            lhs.to_value(),
            rhs.to_value(),
        ))
        .unwrap();
        let invocation = F::new_invocation(
            FunctionArgs::Binary(
                invocation_out.to_value(),
                lhs.to_value(),
                rhs.to_value(),
            )
            .into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();

        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }

    #[test]
    fn selected_binary_set_factories_match_the_legacy_adapter() {
        assert_binary_set_factories_are_equivalent::<SetUnionFxn>();
        assert_binary_set_factories_are_equivalent::<SetDifferenceFxn>();
        assert_binary_set_factories_are_equivalent::<SetIntersectionFxn>();
        assert_binary_set_factories_are_equivalent::<SetCartesianProductFxn>();
    }

    #[test]
    fn selected_relation_factories_match_the_legacy_adapter() {
        assert_binary_bool_factories_are_equivalent::<SetSubsetFxn>();
        assert_binary_bool_factories_are_equivalent::<SetEqualsFxn>();
    }

    #[test]
    fn powerset_and_size_factories_match_the_legacy_adapter() {
        let input = set(&[1, 2]);
        let legacy_powerset_out = set_output();
        let invocation_powerset_out = set_output();
        let legacy_powerset = SetPowersetFxn::new(FunctionArgs::Unary(
            legacy_powerset_out.to_value(),
            input.to_value(),
        ))
        .unwrap();
        let invocation_powerset = SetPowersetFxn::new_invocation(
            FunctionArgs::Unary(invocation_powerset_out.to_value(), input.to_value()).into(),
        )
        .unwrap();
        legacy_powerset.solve_result().unwrap();
        invocation_powerset.solve_result().unwrap();
        assert_eq!(
            *legacy_powerset_out.borrow(),
            *invocation_powerset_out.borrow()
        );

        let legacy_size_out = Ref::new(0_u64);
        let invocation_size_out = Ref::new(0_u64);
        let legacy_size = SetSizeFxn::new(FunctionArgs::Unary(
            legacy_size_out.to_value(),
            input.to_value(),
        ))
        .unwrap();
        let invocation_size = SetSizeFxn::new_invocation(
            FunctionArgs::Unary(invocation_size_out.to_value(), input.to_value()).into(),
        )
        .unwrap();
        legacy_size.solve_result().unwrap();
        invocation_size.solve_result().unwrap();
        assert_eq!(*legacy_size_out.borrow(), *invocation_size_out.borrow());
    }

    #[test]
    fn binary_and_unary_factories_preserve_input_positions() {
        let lhs = set(&[1, 2]);
        let rhs = set(&[2, 3]);
        let difference_out = set_output();
        let difference = SetDifferenceFxn::new_invocation(
            FunctionArgs::Binary(difference_out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        difference.solve_result().unwrap();
        assert_eq!(*difference_out.borrow(), *set(&[1]).borrow());

        let powerset_out = set_output();
        let powerset = SetPowersetFxn::new_invocation(
            FunctionArgs::Unary(powerset_out.to_value(), lhs.to_value()).into(),
        )
        .unwrap();
        lhs.borrow_mut()
            .set
            .insert(Ref::new(3_usize).to_value());
        powerset.solve_result().unwrap();
        assert_eq!(powerset_out.borrow().set.len(), 8);
    }

    #[test]
    fn typed_factories_reject_wrong_types_and_layouts() {
        let out = set_output();
        let rhs = set(&[1]);
        let wrong_type = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Binary(
                out.to_value(),
                Ref::new(true).to_value(),
                rhs.to_value(),
            )
            .into(),
        ));
        assert_eq!(wrong_type.kind_name(), "FunctionArgumentTypeMismatch");

        let wrong_binary_layout = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Unary(out.to_value(), rhs.to_value()).into(),
        ));
        assert_eq!(wrong_binary_layout.kind_name(), "IncorrectNumberOfArguments");
        let wrong_binary_layout = wrong_binary_layout
            .kind_as::<IncorrectNumberOfArguments>()
            .unwrap();
        assert_eq!((wrong_binary_layout.expected, wrong_binary_layout.found), (2, 1));

        let size_out = Ref::new(0_u64);
        let wrong_unary_layout = factory_error(SetSizeFxn::new_invocation(
            FunctionArgs::Binary(size_out.to_value(), rhs.to_value(), rhs.to_value()).into(),
        ));
        assert_eq!(wrong_unary_layout.kind_name(), "IncorrectNumberOfArguments");
        let wrong_unary_layout = wrong_unary_layout
            .kind_as::<IncorrectNumberOfArguments>()
            .unwrap();
        assert_eq!((wrong_unary_layout.expected, wrong_unary_layout.found), (1, 2));
    }

    #[test]
    fn typed_factories_do_not_traverse_legacy_wrappers() {
        let out = set_output();
        let rhs = set(&[2]);
        let typed = decoded_set_wrapper(false);
        let typed_error = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), typed, rhs.to_value()).into(),
        ));
        assert_eq!(typed_error.kind_name(), "FunctionArgumentTypeMismatch");

        let mutable = decoded_set_wrapper(true);
        let mutable_error = factory_error(SetUnionFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), mutable, rhs.to_value()).into(),
        ));
        assert_eq!(mutable_error.kind_name(), "FunctionArgumentTypeMismatch");
    }

    #[test]
    fn function_invocation_keeps_the_exact_argument_layout() {
        let out = set_output();
        let lhs = set(&[1]);
        let rhs = set(&[2]);
        let invocation: FunctionInvocation =
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into();
        let (_, first, second) = invocation.expect_binary().unwrap();

        assert!(first.try_ref::<MechSet>().unwrap().same_handle(&lhs));
        assert!(second.try_ref::<MechSet>().unwrap().same_handle(&rhs));
    }
}

#[cfg(all(
    feature = "union",
    feature = "insert",
    feature = "powerset",
    feature = "cartesian_product",
    feature = "tuple"
))]
mod graph_state {
    use crate::{
        modify::insert::SetInsertFxn,
        operations::{
            cartesian_product::SetCartesianProductFxn, powerset::SetPowersetFxn,
            union::SetUnionFxn,
        },
    };
    use mech_core::{
        FunctionArgs, LegacyValue, MechFunctionFactory, MechSet, MechTuple, Ref, ToValue,
        with_reactive_journal_participant,
    };

    fn index(value: usize) -> Ref<usize> {
        Ref::new(value)
    }

    fn index_value(reference: &Ref<usize>) -> LegacyValue {
        reference.to_value()
    }

    fn set(values: &[usize]) -> Ref<MechSet> {
        Ref::new(MechSet::from_vec(
            values
                .iter()
                .copied()
                .map(|value| index_value(&index(value)))
                .collect(),
        ))
    }

    fn output_set() -> Ref<MechSet> {
        Ref::new(MechSet::from_vec(Vec::new()))
    }

    #[test]
    fn union_graph_state_restores_root_metadata_and_deduplicates() {
        let lhs = set(&[1, 2]);
        let rhs = set(&[2, 3]);
        let out = output_set();
        let out_alias = out.clone();
        let function = SetUnionFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let expected = out.borrow().clone();
        let expected_ids = function.out().reactive_root_cell_ids();
        assert_eq!(function.reactive_output_cell_ids(), expected_ids);
        assert_eq!(function.transaction_state_ports().unwrap().unwrap().len(), 1);

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            let captured_cells = participant.cell_count();
            participant.capture_function_state(&*function)?;
            assert_eq!(participant.cell_count(), captured_cells);

            let mut payload = out.borrow_mut();
            payload.set.clear();
            payload.kind = lhs.to_value().kind();
            payload.max_elements = Some(99);
            payload.num_elements = 99;
            drop(payload);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(out.same_handle(&out_alias));
        assert_eq!(*out.borrow(), expected);
    }

    #[test]
    fn cartesian_product_graph_state_restores_tuples_and_shared_cells() {
        let shared = index(1);
        let nested_tuple = Ref::new(MechTuple::from_vec(vec![index_value(&shared)]));
        let nested_tuple_alias = nested_tuple.clone();
        let lhs = Ref::new(MechSet::from_vec(vec![nested_tuple.to_value()]));
        let rhs = set(&[10, 20]);
        let out = output_set();
        let function = SetCartesianProductFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let expected = out.borrow().clone();
        assert_eq!(nested_tuple.borrow().elements[0].reactive_root_cell_ids(), shared.to_value().reactive_root_cell_ids());

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *shared.borrow_mut() = 99;
            nested_tuple.borrow_mut().elements.clear();
            out.borrow_mut().num_elements = 77;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert_eq!(*shared.borrow(), 1);
        assert!(nested_tuple.same_handle(&nested_tuple_alias));
        assert_eq!(nested_tuple.borrow().elements.len(), 1);
        assert_eq!(nested_tuple.borrow().elements[0].reactive_root_cell_ids(), shared.to_value().reactive_root_cell_ids());
        assert_eq!(*out.borrow(), expected);
    }

    #[test]
    fn powerset_graph_state_restores_nested_sets_recursively() {
        let shared = index(1);
        let nested = Ref::new(MechSet::from_vec(vec![index_value(&shared)]));
        let nested_alias = nested.clone();
        let input = Ref::new(MechSet::from_vec(vec![nested.to_value()]));
        let out = output_set();
        let function = SetPowersetFxn::new_invocation(
            FunctionArgs::Unary(out.to_value(), input.to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let expected = out.borrow().clone();
        let changed_kind = nested.to_value().kind();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            nested.borrow_mut().set.clear();
            nested.borrow_mut().kind = changed_kind.clone();
            out.borrow_mut().set.clear();
            *shared.borrow_mut() = 9;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(nested.same_handle(&nested_alias));
        assert_eq!(nested.borrow().set.len(), 1);
        assert_eq!(nested.borrow().kind, index_value(&shared).kind());
        assert!(nested.borrow().set.contains(&index_value(&shared)));
        assert_eq!(*shared.borrow(), 1);
        assert_eq!(*out.borrow(), expected);
    }

    #[test]
    fn distinct_equal_graph_outputs_keep_distinct_reactive_identity() {
        let lhs = set(&[1]);
        let rhs = set(&[2]);
        let first_out = output_set();
        let second_out = output_set();
        let first = SetUnionFxn::new_invocation(
            FunctionArgs::Binary(first_out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        let second = SetUnionFxn::new_invocation(
            FunctionArgs::Binary(second_out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        first.solve_result().unwrap();
        second.solve_result().unwrap();

        assert_eq!(*first_out.borrow(), *second_out.borrow());
        assert!(!first_out.same_handle(&second_out));
        assert_ne!(
            first.reactive_output_cell_ids(),
            second.reactive_output_cell_ids()
        );
    }

    #[test]
    fn limit_error_is_atomic_and_the_prior_output_can_be_restored() {
        let lhs = set(&[1, 2]);
        let rhs = set(&[3, 4]);
        let out = output_set();
        let function = SetCartesianProductFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let successful = out.borrow().clone();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *lhs.borrow_mut() = MechSet::from_vec(
                (0..257)
                    .map(|value| Ref::new(value).to_value())
                    .collect(),
            );
            *rhs.borrow_mut() = MechSet::from_vec(
                (0..257)
                    .map(|value| Ref::new(value).to_value())
                    .collect(),
            );
            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "SetCartesianProductLimitExceeded");
            assert_eq!(*out.borrow(), successful);

            out.borrow_mut().set.clear();
            out.borrow_mut().kind = lhs.to_value().kind();
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert_eq!(*out.borrow(), successful);
    }

    #[test]
    fn insert_output_uses_the_same_graph_state_path() {
        let member = index(1);
        let input = Ref::new(MechSet::from_vec(vec![index_value(&member)]));
        let inserted = index(2);
        let out = Ref::new(MechSet::new(index_value(&inserted).kind(), 2));
        let out_alias = out.clone();
        let function = SetInsertFxn::new_invocation(
            FunctionArgs::Binary(out.to_value(), input.to_value(), index_value(&inserted)).into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        let expected = out.borrow().clone();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            out.borrow_mut().set.clear();
            out.borrow_mut().max_elements = Some(99);
            *inserted.borrow_mut() = 20;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(out.same_handle(&out_alias));
        assert_eq!(*out.borrow(), expected);
        assert_eq!(*inserted.borrow(), 2);
    }
}

#[cfg(all(
    feature = "element_of",
    feature = "not_element_of",
    feature = "insert",
    feature = "remove",
    feature = "bool",
    feature = "f64"
))]
mod any_value_compatibility {
    use crate::{
        membership::{element_of::SetElementOfFxn, not_element_of::SetNotElementOfFxn},
        modify::{insert::SetInsertFxn, remove::SetRemoveFxn},
    };
    use mech_core::{
        EncodedConstant, FunctionArgs, LegacyValue, MechFunctionFactory, MechSet, Ref,
        RuntimeType, ToValue, decode_encoded_constants, with_reactive_journal_participant,
    };

    fn scalar(value: f64) -> Ref<f64> {
        Ref::new(value)
    }

    fn scalar_value(reference: &Ref<f64>) -> LegacyValue {
        reference.to_value()
    }

    fn referenced_scalar(value: f64) -> LegacyValue {
        let payload = value.to_le_bytes();
        let mut bytes = (payload.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(&payload);
        decode_encoded_constants(&[EncodedConstant {
            runtime_type: RuntimeType::Reference(Box::new(RuntimeType::F64)),
            alignment: 8,
            bytes,
        }])
        .unwrap()
        .pop()
        .unwrap()
    }

    fn scalar_set(values: &[f64]) -> Ref<MechSet> {
        Ref::new(MechSet::from_vec(
            values
                .iter()
                .copied()
                .map(|value| scalar_value(&scalar(value)))
                .collect(),
        ))
    }

    #[test]
    fn any_value_membership_uses_the_default_invocation_bridge() {
        let set = scalar_set(&[1.0, 2.0]);
        let member = scalar(2.0);
        let element_of_out = Ref::new(false);
        let not_element_of_out = Ref::new(false);
        let element_of = SetElementOfFxn::new_invocation(
            FunctionArgs::Binary(
                element_of_out.to_value(),
                scalar_value(&member),
                set.to_value(),
            )
            .into(),
        )
        .unwrap();
        let not_element_of = SetNotElementOfFxn::new_invocation(
            FunctionArgs::Binary(
                not_element_of_out.to_value(),
                scalar_value(&member),
                set.to_value(),
            )
            .into(),
        )
        .unwrap();
        element_of.solve_result().unwrap();
        not_element_of.solve_result().unwrap();
        assert!(*element_of_out.borrow());
        assert!(!*not_element_of_out.borrow());

        let wrapped = referenced_scalar(2.0);
        let wrapped_out = Ref::new(false);
        SetElementOfFxn::new_invocation(
            FunctionArgs::Binary(wrapped_out.to_value(), wrapped, set.to_value()).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert!(*wrapped_out.borrow());
    }

    #[test]
    fn any_value_insert_and_remove_preserve_kind_and_cardinality() {
        let input = scalar_set(&[1.0]);
        let inserted = scalar(2.0);
        let scalar_kind = scalar_value(&inserted).kind();
        let insert_out = Ref::new(MechSet::new(scalar_kind.clone(), 2));
        let insert = SetInsertFxn::new_invocation(
            FunctionArgs::Binary(
                insert_out.to_value(),
                input.to_value(),
                scalar_value(&inserted),
            )
            .into(),
        )
        .unwrap();
        insert.solve_result().unwrap();
        assert_eq!(insert_out.borrow().kind, scalar_kind);
        assert_eq!(insert_out.borrow().num_elements, 2);
        assert_eq!(insert_out.borrow().max_elements, Some(2));

        let remove_out = Ref::new(MechSet::new(scalar_value(&inserted).kind(), 2));
        let remove = SetRemoveFxn::new_invocation(
            FunctionArgs::Binary(
                remove_out.to_value(),
                insert_out.to_value(),
                scalar_value(&inserted),
            )
            .into(),
        )
        .unwrap();
        remove.solve_result().unwrap();
        assert_eq!(remove_out.borrow().kind, scalar_value(&inserted).kind());
        assert_eq!(remove_out.borrow().num_elements, 1);
        assert_eq!(remove_out.borrow().max_elements, Some(1));
    }

    #[test]
    fn membership_scalar_state_is_authoritative() {
        let set = scalar_set(&[1.0]);
        let out = Ref::new(false);
        let function = SetElementOfFxn::new_invocation(
            FunctionArgs::Binary(
                out.to_value(),
                Ref::new(1.0_f64).to_value(),
                set.to_value(),
            )
            .into(),
        )
        .unwrap();
        function.solve_result().unwrap();
        assert!(function.transaction_state_ports().unwrap().is_some());

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *out.borrow_mut() = false;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(*out.borrow());
    }
}

#[cfg(all(
    feature = "equals",
    feature = "size",
    feature = "bool",
    feature = "u64"
))]
mod scalar_state {
    use crate::{relations::equals::SetEqualsFxn, setdata::size::SetSizeFxn};
    use mech_core::{
        FunctionArgs, MechFunctionFactory, MechSet, Ref, ToValue,
        with_reactive_journal_participant,
    };

    fn set(values: &[usize]) -> Ref<MechSet> {
        Ref::new(MechSet::from_vec(
            values
                .iter()
                .copied()
                .map(|value| Ref::new(value).to_value())
                .collect(),
        ))
    }

    #[test]
    fn relation_and_size_outputs_restore_through_exact_state_ports() {
        let lhs = set(&[1, 2]);
        let rhs = set(&[1, 2]);
        let relation_out = Ref::new(false);
        let size_out = Ref::new(0_u64);
        let relation = SetEqualsFxn::new_invocation(
            FunctionArgs::Binary(relation_out.to_value(), lhs.to_value(), rhs.to_value()).into(),
        )
        .unwrap();
        let size = SetSizeFxn::new_invocation(
            FunctionArgs::Unary(size_out.to_value(), lhs.to_value()).into(),
        )
        .unwrap();
        relation.solve_result().unwrap();
        size.solve_result().unwrap();
        assert!(*relation_out.borrow());
        assert_eq!(*size_out.borrow(), 2);

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*relation)?;
            participant.capture_function_state(&*size)?;
            *relation_out.borrow_mut() = false;
            *size_out.borrow_mut() = 99;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(*relation_out.borrow());
        assert_eq!(*size_out.borrow(), 2);
    }
}

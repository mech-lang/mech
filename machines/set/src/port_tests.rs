#[cfg(all(
    feature = "source",
    feature = "union",
    feature = "insert",
    feature = "remove",
    feature = "powerset",
    feature = "cartesian_product"
))]
mod dynamic_outputs {
    use crate::{
        SetCartesianProduct, SetInsert, SetPowerset, SetRemove, SetUnion,
        operations::union::SetUnionFxn,
    };
    use mech_core::{
        AsValueKind, CanonicalFunctionSpecializer, CardinalitySpec, DimensionExpr, FunctionArgs,
        MResult, MechFunctionFactory, MechSet, Ref, SchemaBody, SpecializationContext,
        SpecializationInvocation, ToValue, ValueCell, ValueData, ValueDataDraft,
        with_reactive_journal_participant,
    };

    fn set(values: &[u64]) -> ValueCell {
        ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::Index),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Set(
                values
                    .iter()
                    .copied()
                    .map(ValueDataDraft::Index)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        )
        .unwrap()
    }

    fn index(value: u64) -> ValueCell {
        ValueCell::from_schema_data(SchemaBody::Index, ValueDataDraft::Index(value)).unwrap()
    }

    fn specialize(
        specializer: &dyn CanonicalFunctionSpecializer,
        inputs: Vec<ValueCell>,
    ) -> mech_core::FunctionInstance {
        let invocation = SpecializationInvocation::from_cells(inputs.into_boxed_slice());
        let mut context = SpecializationContext::for_invocation(&invocation, None).unwrap();
        specializer
            .specialize_invocation(&invocation, &mut context)
            .unwrap()
            .into_instance()
    }

    fn replace_set(cell: &ValueCell, values: &[u64]) {
        cell.replace(&set(values).snapshot().unwrap()).unwrap();
    }

    fn set_values(cell: &ValueCell) -> Vec<u64> {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::Set(set) = snapshot.data() else {
            panic!("expected set output")
        };
        set.elements()
            .iter()
            .map(|value| match value.data() {
                ValueData::Index(value) => *value,
                _ => panic!("expected index element"),
            })
            .collect()
    }

    fn assert_dynamic_set(cell: &ValueCell) {
        assert!(matches!(
            cell.closed_schema_body().unwrap(),
            SchemaBody::Set {
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                ..
            }
        ));
    }

    #[test]
    fn union_preserves_output_identity_across_zero_three_one_zero() {
        let lhs = set(&[]);
        let rhs = set(&[1, 2, 3]);
        let function = specialize(&SetUnion {}, vec![lhs.clone(), rhs.clone()]);
        let output = function.output().clone();
        let alias = output.clone();

        function.solve_result().unwrap();
        assert_eq!(set_values(&output), vec![1, 2, 3]);
        replace_set(&rhs, &[2]);
        function.solve_result().unwrap();
        assert_eq!(set_values(&output), vec![2]);
        replace_set(&rhs, &[]);
        function.solve_result().unwrap();
        assert!(set_values(&output).is_empty());
        assert!(output.same_cell(&alias));
        assert_dynamic_set(&output);

        replace_set(&lhs, &[1, 2]);
        replace_set(&rhs, &[2, 3]);
        function.solve_result().unwrap();
        assert_eq!(set_values(&output), vec![1, 2, 3]);
        replace_set(&rhs, &[3, 4]);
        function.solve_result().unwrap();
        assert_eq!(set_values(&output), vec![1, 2, 3, 4]);
    }

    #[test]
    fn insert_and_remove_observe_present_and_absent_elements() {
        let source = set(&[1, 2]);
        let present = specialize(&SetInsert {}, vec![source.clone(), index(2)]);
        present.solve_result().unwrap();
        assert_eq!(set_values(present.output()), vec![1, 2]);

        let fresh = specialize(&SetInsert {}, vec![source.clone(), index(3)]);
        fresh.solve_result().unwrap();
        assert_eq!(set_values(fresh.output()), vec![1, 2, 3]);

        let absent = specialize(&SetRemove {}, vec![source.clone(), index(3)]);
        absent.solve_result().unwrap();
        assert_eq!(set_values(absent.output()), vec![1, 2]);

        let removed = specialize(&SetRemove {}, vec![source, index(1)]);
        removed.solve_result().unwrap();
        assert_eq!(set_values(removed.output()), vec![2]);
    }

    #[test]
    fn powerset_and_cartesian_product_have_nested_dynamic_schemas() {
        let powerset = specialize(&SetPowerset {}, vec![set(&[1, 2])]);
        powerset.solve_result().unwrap();
        let snapshot = powerset.output().snapshot().unwrap();
        let ValueData::Set(subsets) = snapshot.data() else {
            panic!("expected powerset output")
        };
        let mut sizes = subsets
            .elements()
            .iter()
            .map(|subset| match subset.data() {
                ValueData::Set(values) => values.elements().len(),
                _ => panic!("expected nested set"),
            })
            .collect::<Vec<_>>();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![0, 1, 1, 2]);
        assert!(matches!(
            powerset.output().closed_schema_body().unwrap(),
            SchemaBody::Set {
                element,
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            } if matches!(
                *element,
                SchemaBody::Set {
                    cardinality: CardinalitySpec::Dynamic { upper_bound: None },
                    ..
                }
            )
        ));

        let product = specialize(&SetCartesianProduct {}, vec![set(&[1, 2]), set(&[3, 4])]);
        product.solve_result().unwrap();
        let snapshot = product.output().snapshot().unwrap();
        let ValueData::Set(pairs) = snapshot.data() else {
            panic!("expected product set")
        };
        assert_eq!(pairs.elements().len(), 4);
        assert!(pairs
            .elements()
            .iter()
            .all(|pair| matches!(pair.data(), ValueData::Tuple(_))));
        assert_dynamic_set(product.output());
    }

    #[test]
    fn transaction_rollback_restores_dynamic_payload_and_identity() {
        let lhs = set(&[1]);
        let rhs = set(&[2]);
        let function = specialize(&SetUnion {}, vec![lhs, rhs.clone()]);
        function.solve_result().unwrap();
        let output = function.output().clone();
        let alias = output.clone();

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(&function)?;
            replace_set(&rhs, &[2, 3, 4]);
            function.solve_result()?;
            assert_eq!(set_values(&output), vec![1, 2, 3, 4]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(output.same_cell(&alias));
        assert_eq!(set_values(&output), vec![1, 2]);
    }

    #[test]
    fn dynamic_and_exact_set_schema_identities_are_distinct() {
        let dynamic = set(&[1, 2]);
        let exact = ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(SchemaBody::Index),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
            ValueDataDraft::Set(
                vec![ValueDataDraft::Index(1), ValueDataDraft::Index(2)].into_boxed_slice(),
            ),
        )
        .unwrap();
        assert_ne!(dynamic.schema_key(), exact.schema_key());
    }

    #[test]
    fn compatibility_adapter_populates_the_original_empty_set_handle() {
        let output = Ref::new(MechSet::new(<usize as AsValueKind>::as_value_kind(), 0));
        let output_alias = output.clone();
        let lhs = Ref::new(MechSet::from_vec(vec![Ref::new(1usize).to_value()]));
        let rhs = Ref::new(MechSet::from_vec(vec![
            Ref::new(2usize).to_value(),
            Ref::new(3usize).to_value(),
        ]));
        let function = SetUnionFxn::new(FunctionArgs::Binary(
            output.clone().to_value(),
            lhs.to_value(),
            rhs.to_value(),
        ))
        .unwrap();

        function.solve_result().unwrap();

        assert!(output.same_handle(&output_alias));
        assert_eq!(output.borrow().set.len(), 3);
        assert_eq!(output.borrow().num_elements, 3);
    }
}

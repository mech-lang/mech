#[cfg(all(
    feature = "source",
    feature = "union",
    feature = "insert",
    feature = "remove",
    feature = "powerset",
    feature = "cartesian_product"
))]
mod dynamic_outputs {
    use crate::{SetCartesianProduct, SetInsert, SetPowerset, SetRemove, SetUnion};
    use mech_core::{
        CanonicalFunctionSpecializer, CardinalitySpec, DimensionExpr, MResult, SchemaBody,
        SpecializationContext, SpecializationInvocation, ValueCell, ValueData, ValueDataDraft,
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
            .into_parts()
            .0
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
    fn union_preserves_output_identity_across_turn_varying_extents() {
        let lhs = set(&[]);
        let rhs = set(&[1, 2, 3]);
        let function = specialize(&SetUnion {}, vec![lhs.clone(), rhs.clone()]);
        let output = function.output().clone();
        let alias = output.clone();

        for (next, expected) in [
            (vec![1, 2, 3], vec![1, 2, 3]),
            (vec![2], vec![2]),
            (vec![], vec![]),
            (vec![3, 4], vec![3, 4]),
        ] {
            replace_set(&rhs, &next);
            function.solve_result().unwrap();
            assert_eq!(set_values(&output), expected);
            assert!(output.same_cell(&alias));
        }
        assert_dynamic_set(&output);
    }

    #[test]
    fn insert_remove_powerset_and_product_preserve_dynamic_schemas() {
        let source = set(&[1, 2]);
        let inserted = specialize(&SetInsert {}, vec![source.clone(), index(3)]);
        inserted.solve_result().unwrap();
        assert_eq!(set_values(inserted.output()), vec![1, 2, 3]);

        let removed = specialize(&SetRemove {}, vec![source.clone(), index(1)]);
        removed.solve_result().unwrap();
        assert_eq!(set_values(removed.output()), vec![2]);

        let powerset = specialize(&SetPowerset {}, vec![source]);
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
            panic!("expected product output")
        };
        assert_eq!(pairs.elements().len(), 4);
        assert!(
            pairs
                .elements()
                .iter()
                .all(|pair| matches!(pair.data(), ValueData::Tuple(_)))
        );
        assert_dynamic_set(product.output());
    }

    #[test]
    fn transaction_rollback_restores_dynamic_payload_schema_and_identity() {
        let lhs = set(&[1]);
        let rhs = set(&[2]);
        let function = specialize(&SetUnion {}, vec![lhs, rhs.clone()]);
        function.solve_result().unwrap();
        let output = function.output().clone();
        let alias = output.clone();
        let schema = output.schema_key();

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
        assert_eq!(output.schema_key(), schema);
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
}

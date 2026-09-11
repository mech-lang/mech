#[cfg(all(
    feature = "source",
    feature = "union",
    feature = "insert",
    feature = "remove",
    feature = "powerset",
    feature = "cartesian_product"
))]
mod dynamic_outputs {
    use crate::*;
    use mech_core::{
        CardinalitySpec, DimensionExpr, MResult, MemoryFailurePoint, SchemaBody, ValueCell,
        ValueData, ValueDataDraft, with_reactive_journal_participant,
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

    fn specialize(name: &str, cells: Vec<ValueCell>) -> mech_core::SpecializedFunction {
        let mut builder = FunctionCatalogBuilder::new();
        crate::catalog::install_runtime(&mut builder).unwrap();
        crate::catalog::install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let entry = catalog.specializer(OperationId::from_name(name)).unwrap();
        let originals = cells
            .iter()
            .map(ValueCell::resolved_type)
            .collect::<MResult<Vec<_>>>()
            .unwrap();
        let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
            panic!("set operation must have a semantic scheme")
        };
        let instantiated = declaration.template.map(|template| {
            FunctionTypeDeclaration::from_schemes(
                instantiate_source_scheme_template(template, &originals).unwrap(),
            )
        });
        let declaration = instantiated.as_ref().unwrap_or(declaration);
        let candidates = declaration
            .overloads
            .iter()
            .map(|overload| TypeOverloadCandidate {
                id: u64::from(overload.id),
                scheme: &overload.scheme,
            })
            .collect::<Vec<_>>();
        let resolved = resolve_type_overloads(
            TypeConstraintOrigin::new(name, None),
            &candidates,
            &originals,
            None,
        )
        .unwrap();
        let overload_id = u32::try_from(resolved.candidate_ids[0]).unwrap();
        let overload = declaration
            .overloads
            .iter()
            .find(|overload| overload.id == overload_id)
            .unwrap();
        let converted_inputs = resolved
            .conversions
            .iter()
            .map(|plan| plan.target.clone())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let operation = entry
            .resolved_operation(converted_inputs.len(), &resolved.outputs)
            .unwrap();
        let resolved = ResolvedCall {
            operation,
            overload_id,
            original_inputs: originals.into_boxed_slice(),
            converted_inputs,
            input_conversions: resolved.conversions,
            outputs: resolved.outputs,
            output_schema_rules: overload.output_schema_rules.clone(),
        };
        let invocation = SpecializationInvocation::from_cells(cells.into_boxed_slice());
        let mut context = SpecializationContext::for_resolved_invocation(
            &invocation,
            Some(&catalog),
            entry.operation.id,
            name,
            resolved,
        )
        .unwrap();
        entry
            .specializer
            .specialize_invocation(&invocation, &mut context)
            .unwrap()
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
            SchemaBody::Set { .. }
        ));
    }

    #[test]
    fn union_preserves_output_identity_across_turn_varying_extents() {
        let lhs = set(&[1]);
        let rhs = set(&[1]);
        let function = specialize("set/union", vec![lhs.clone(), rhs.clone()]);
        let output = function.output().clone();
        let alias = output.clone();

        for (next, expected) in [
            (vec![1], vec![1]),
            (vec![2], vec![1, 2]),
            (vec![], vec![1]),
            (vec![3, 4], vec![1, 3, 4]),
        ] {
            replace_set(&rhs, &next);
            function.instance().solve_result().unwrap();
            assert_eq!(set_values(&output), expected);
            assert!(output.same_logical_cell(&alias));
        }
        assert_dynamic_set(&output);
    }

    #[test]
    fn set_draft_allocation_failure_preserves_the_published_root() {
        let lhs = set(&[1, 2]);
        let rhs = set(&[3]);
        let function = specialize("set/union", vec![lhs, rhs.clone()]);
        function.instance().solve_result().unwrap();
        let output = function.output().clone();
        let before = set_values(&output);
        let version = output.published_version();

        replace_set(&rhs, &[3, 4, 5]);
        output
            .memory_domain()
            .unwrap()
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 1)
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(set_values(&output), before);
        assert_eq!(output.published_version(), version);

        function.instance().solve_result().unwrap();
        assert_eq!(set_values(&output), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn insert_remove_powerset_and_product_preserve_dynamic_schemas() {
        let source = set(&[1, 2]);
        let inserted = specialize("set/insert", vec![source.clone(), index(3)]);
        inserted.instance().solve_result().unwrap();
        assert_eq!(set_values(inserted.output()), vec![1, 2, 3]);

        let removed = specialize("set/remove", vec![source.clone(), index(1)]);
        removed.instance().solve_result().unwrap();
        assert_eq!(set_values(removed.output()), vec![2]);

        let powerset = specialize("set/powerset", vec![source]);
        powerset.instance().solve_result().unwrap();
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
            SchemaBody::Set { element, .. } if matches!(*element, SchemaBody::Set { .. })
        ));

        let product = specialize("set/cartesian-product", vec![set(&[1, 2]), set(&[3, 4])]);
        product.instance().solve_result().unwrap();
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
        let function = specialize("set/union", vec![lhs, rhs.clone()]);
        function.instance().solve_result().unwrap();
        let output = function.output().clone();
        let alias = output.clone();
        let schema = output.schema_key();

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_instance(function.instance())?;
            replace_set(&rhs, &[2, 3, 4]);
            function.instance().solve_result()?;
            assert_eq!(set_values(&output), vec![1, 2, 3, 4]);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(output.same_logical_cell(&alias));
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

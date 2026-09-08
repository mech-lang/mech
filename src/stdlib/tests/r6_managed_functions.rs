#[cfg(feature = "full_compiler")]
use mech_core::{ExecutionTarget, ImplementationMemoryClass, OperationId, RuntimeBindingSelector};

#[cfg(feature = "full_compiler")]
#[test]
fn maintained_catalog_has_no_open_or_unclassified_memory_implementation() {
    let catalog = mech_stdlib::source_catalog();
    assert_ne!(catalog.runtime_entries().len(), 0);
    for entry in catalog.runtime_entries() {
        match entry.implementation_memory_class() {
            ImplementationMemoryClass::NoAdditionalScratch
            | ImplementationMemoryClass::CloneInput { .. }
            | ImplementationMemoryClass::AbiContiguousBridge { .. }
            | ImplementationMemoryClass::MatrixSolve
            | ImplementationMemoryClass::CanonicalFinalize
            | ImplementationMemoryClass::CanonicalSortUnique => {}
        }
    }
}

#[cfg(feature = "full_compiler")]
#[test]
fn scratch_owning_families_never_fall_back_to_an_open_memory_class() {
    let catalog = mech_stdlib::source_catalog();
    for (operation, expected) in [
        ("matrix/solve", ImplementationMemoryClass::MatrixSolve),
        ("set/union", ImplementationMemoryClass::CanonicalSortUnique),
        (
            "set/intersection",
            ImplementationMemoryClass::CanonicalSortUnique,
        ),
        (
            "set/difference",
            ImplementationMemoryClass::CanonicalSortUnique,
        ),
    ] {
        let entries = catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(OperationId::from_name(operation)),
                ExecutionTarget::DirectRuntime,
            )
            .collect::<Vec<_>>();
        assert!(!entries.is_empty(), "missing {operation}");
        assert!(
            entries
                .iter()
                .all(|entry| entry.implementation_memory_class() == expected),
            "{operation} has a non-managed scratch declaration"
        );
    }
}

#[cfg(feature = "full_compiler")]
mod ordinary_managed_execution {
    use mech_core::*;
    use nalgebra::DMatrix;

    fn specialize(name: &str, inputs: Vec<ValueCell>) -> SpecializedFunction {
        let catalog = mech_stdlib::source_catalog();
        let entry = catalog.specializer(OperationId::from_name(name)).unwrap();
        let original = inputs
            .iter()
            .map(|input| input.resolved_type().unwrap())
            .collect::<Vec<_>>();
        let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
            panic!("{name} must be selected by its semantic scheme")
        };
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
            &original,
            None,
        )
        .unwrap();
        let overload_id = u32::try_from(resolved.candidate_ids[0]).unwrap();
        let overload = declaration
            .overloads
            .iter()
            .find(|overload| overload.id == overload_id)
            .unwrap();
        assert!(
            resolved
                .conversions
                .iter()
                .all(|plan| matches!(plan.step, ConversionStep::Identity))
        );
        let operation = entry
            .resolved_operation(inputs.len(), &resolved.outputs)
            .unwrap();
        let resolved = ResolvedCall {
            operation,
            overload_id,
            original_inputs: original.clone().into_boxed_slice(),
            converted_inputs: original.into_boxed_slice(),
            input_conversions: resolved.conversions,
            outputs: resolved.outputs,
            output_schema_rules: overload.output_schema_rules.clone(),
        };
        let invocation = SpecializationInvocation::from_cells(inputs.into_boxed_slice());
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

    fn transpose(input: ValueCell) -> SpecializedFunction {
        specialize("matrix/transpose", vec![input])
    }

    fn add(left: ValueCell, right: ValueCell) -> SpecializedFunction {
        specialize("math/add", vec![left, right])
    }

    fn matrix_values(cell: &ValueCell) -> (Vec<u64>, Vec<f64>) {
        let value = cell.snapshot().unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("expected matrix")
        };
        let snapshot::SequenceView::F64(values) = matrix.elements() else {
            panic!("expected F64")
        };
        (
            value.shape().parameter_values().to_vec(),
            values.iter().map(|value| value.to_f64()).collect(),
        )
    }

    fn scalar_value(cell: &ValueCell) -> f64 {
        let value = cell.snapshot().unwrap();
        let ValueData::F64(value) = value.data() else {
            panic!("expected F64 scalar")
        };
        value.to_f64()
    }

    #[test]
    fn maintained_scalar_add_uses_the_ordinary_managed_function_entry() {
        let session = MemoryDomain::new().unwrap();
        let left = ValueCell::from_exact_in(&session, 20.0_f64).unwrap();
        let right = ValueCell::from_exact_in(&session, 22.0_f64).unwrap();
        let function = add(left.clone(), right.clone());
        let output = function.output().clone();
        assert_eq!(output.memory_domain().unwrap().id(), session.id());

        function.instance().solve_result().unwrap();
        assert_eq!(scalar_value(&output), 42.0);

        left.replace(&ValueCell::from_exact(1.5_f64).unwrap().snapshot().unwrap())
            .unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(scalar_value(&output), 23.5);
    }

    #[test]
    fn nested_convenience_call_rejects_before_cold_replanning_or_kernel_execution() {
        let session = MemoryDomain::new().unwrap();
        let input = ValueCell::from_exact_in(
            &session,
            DMatrix::from_row_slice(2, 3, &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0]),
        )
        .unwrap();
        let function = transpose(input.clone());
        function.instance().solve_result().unwrap();
        let output = function.output().clone();
        let version = output.published_version();
        input
            .replace(
                &ValueCell::from_exact(DMatrix::from_row_slice(
                    3,
                    2,
                    &[7.0_f64, 8.0, 9.0, 10.0, 11.0, 12.0],
                ))
                .unwrap()
                .snapshot()
                .unwrap(),
            )
            .unwrap();
        session
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 0)
            .unwrap();
        {
            let _outer_scope = session.enter_plan_point(MemoryPlanPoint::new(0)).unwrap();
            let before = session.ledger();
            let error = function.instance().solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "ManagedExecutionScopeRequired");
            assert_eq!(session.ledger(), before);
            assert_eq!(output.published_version(), version);
        }
        // The nested call did not even consume the allocation-failure probe.
        assert!(function.instance().solve_result().is_err());
        assert_eq!(output.published_version(), version);
        assert_eq!(matrix_values(&output).1, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
        function.instance().solve_result().unwrap();
        assert_eq!(
            matrix_values(&output).1,
            vec![7.0, 9.0, 11.0, 8.0, 10.0, 12.0]
        );
    }

    #[test]
    fn reactive_plan_executes_retained_managed_instance_and_shares_its_plan() {
        let session = MemoryDomain::new().unwrap();
        let input = ValueCell::from_exact_in(
            &session,
            DMatrix::from_row_slice(2, 3, &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0]),
        )
        .unwrap();
        let function = transpose(input.clone());
        let output = function.output().clone();
        let plan = Plan::new();
        let node = plan.register_specialized(function).unwrap();
        {
            let graph = plan.borrow();
            let function = &graph.node(node).unwrap().function;
            let retained = function.instance().unwrap();
            assert!(std::ptr::eq(
                function.memory_plan().unwrap(),
                retained.memory_plan()
            ));
            assert!(std::ptr::eq(
                function.bound_call().unwrap(),
                &retained.memory_plan().bound_call
            ));
        }
        let result = plan
            .borrow_mut()
            .solve_dirty_cells(&[input.reactive_cell_id()])
            .unwrap();
        assert_eq!(result.executed_nodes, vec![node]);
        assert_eq!(matrix_values(&output).1, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
        input
            .replace(
                &ValueCell::from_exact(DMatrix::from_row_slice(
                    3,
                    2,
                    &[7.0_f64, 8.0, 9.0, 10.0, 11.0, 12.0],
                ))
                .unwrap()
                .snapshot()
                .unwrap(),
            )
            .unwrap();
        let result = plan
            .borrow_mut()
            .solve_dirty_cells(&[input.reactive_cell_id()])
            .unwrap();
        assert_eq!(result.executed_nodes, vec![node]);
        assert_eq!(
            matrix_values(&output).1,
            vec![7.0, 9.0, 11.0, 8.0, 10.0, 12.0]
        );
    }

    #[test]
    fn bound_ordinary_transpose_follows_actual_cell_growth_and_rejected_update() {
        let session = MemoryDomain::new().unwrap();
        let input = ValueCell::from_exact_in(
            &session,
            DMatrix::from_row_slice(2, 3, &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0]),
        )
        .unwrap();
        let input_alias = input.clone();
        let old_snapshot = input.snapshot().unwrap();
        let function = transpose(input.clone());
        let output_alias = function.output().clone();
        assert_eq!(
            function.output().memory_domain().unwrap().id(),
            session.id()
        );
        function.instance().solve_result().unwrap();
        assert_eq!(
            matrix_values(&output_alias).1,
            vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]
        );

        let next = ValueCell::from_exact(DMatrix::from_row_slice(
            3,
            2,
            &[7.0_f64, 8.0, 9.0, 10.0, 11.0, 12.0],
        ))
        .unwrap()
        .snapshot()
        .unwrap();
        for failure in [
            MemoryFailurePoint::Admission,
            MemoryFailurePoint::HostAllocation,
        ] {
            let ledger = session.ledger();
            let version = input.published_version();
            session.inject_failure_after(failure, 0).unwrap();
            assert!(
                input.replace(&next).is_err(),
                "expected {failure:?} rejection"
            );
            assert_eq!(
                session.ledger(),
                ledger,
                "failed growth leaked ownership at {failure:?}"
            );
            assert_eq!(input.published_version(), version);
            assert_eq!(
                matrix_values(&input_alias).1,
                vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
            );
            function.instance().solve_result().unwrap();
            assert_eq!(
                matrix_values(&output_alias).1,
                vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]
            );
        }
        input.replace(&next).unwrap();
        assert_eq!(
            matrix_values(&input_alias).1,
            vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]
        );
        // The already-bound maintained implementation receives fresh logical
        // views. This test supplies no object mapping or side buffer.
        let output_version = output_alias.published_version();
        let ledger = session.ledger();
        session
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 0)
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(session.ledger(), ledger);
        assert_eq!(output_alias.published_version(), output_version);
        assert_eq!(
            matrix_values(&output_alias).1,
            vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]
        );
        function.instance().solve_result().unwrap();
        assert_eq!(
            matrix_values(&output_alias).1,
            vec![7.0, 9.0, 11.0, 8.0, 10.0, 12.0]
        );
        assert!(function.output().same_logical_cell(&output_alias));
        assert!(input.same_logical_cell(&input_alias));
        let version = input.published_version();
        let bad = ValueCell::from_exact(99.0_f64).unwrap().snapshot().unwrap();
        assert!(input.replace(&bad).is_err());
        assert_eq!(input.published_version(), version);
        session.issue_plan_revision().unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(
            matrix_values(&output_alias).1,
            vec![7.0, 9.0, 11.0, 8.0, 10.0, 12.0]
        );

        session.close().unwrap();
        assert_eq!(old_snapshot.shape().parameter_values(), &[2, 3]);
        let ValueData::Matrix(matrix) = old_snapshot.data() else {
            panic!("expected matrix")
        };
        let snapshot::SequenceView::F64(values) = matrix.elements() else {
            panic!("expected F64")
        };
        assert_eq!(
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>(),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
        assert!(input_alias.snapshot().is_err());
        drop(function);
        drop(output_alias);
        drop(input_alias);
        drop(input);
        session.collect_retired().unwrap();
        assert_eq!(session.ledger().committed_bytes, 0);
        assert_eq!(session.ledger().live_allocations, 0);
        assert_eq!(session.ledger().retired_allocations, 0);
    }
}

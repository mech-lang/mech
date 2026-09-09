#[cfg(any(feature = "standard_compiler", feature = "full_compiler"))]
use mech_core::{ExecutionTarget, ImplementationMemoryClass, OperationId, RuntimeBindingSelector};

#[cfg(any(feature = "standard_compiler", feature = "full_compiler"))]
#[test]
fn maintained_catalog_has_no_open_or_unclassified_memory_implementation() {
    let catalog = mech_stdlib::source_catalog();
    assert_ne!(catalog.runtime_entries().len(), 0);
    for entry in catalog.runtime_entries() {
        match entry.implementation_memory_class() {
            ImplementationMemoryClass::NoAdditionalScratch
            | ImplementationMemoryClass::CloneInput { .. }
            | ImplementationMemoryClass::CanonicalCloneInput { .. }
            | ImplementationMemoryClass::AbiContiguousBridge { .. }
            | ImplementationMemoryClass::ExternalMarshalling
            | ImplementationMemoryClass::MatrixSolve
            | ImplementationMemoryClass::CanonicalFinalize
            | ImplementationMemoryClass::CanonicalSortUnique => {}
        }
    }
}

#[cfg(any(feature = "standard_compiler", feature = "full_compiler"))]
fn r6_runtime_family(entry: &mech_core::RuntimeFunctionEntry) -> Option<&'static str> {
    use mech_core::{
        FunctionMatrixElement, FunctionMatrixStoragePattern, FunctionValueRepresentation,
    };

    let name = entry.name.as_str();
    if name.starts_with("Access")
        || name.starts_with("RecordAccess")
        || name.starts_with("TableAccess")
        || name.starts_with("TupleAccess")
        || name == "access/index"
    {
        return Some("F04");
    }
    if name.starts_with("Assign")
        || name.starts_with("Set") && name.as_bytes().get(3).is_some_and(u8::is_ascii_digit)
    {
        return Some("F05");
    }
    if name.starts_with("AddAssign")
        || name.starts_with("SubAssign")
        || name.starts_with("MulAssign")
        || name.starts_with("DivAssign")
        || name.starts_with("Dot")
        || name.starts_with("MatMul")
        || name.starts_with("MatrixSolve")
    {
        return Some("F02");
    }
    if name.starts_with("Transpose") {
        return match entry.signature().output {
            FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::String,
                storage: FunctionMatrixStoragePattern::Exact(_),
            } => Some("F03"),
            _ => Some("F02"),
        };
    }
    if name.starts_with("Concat") {
        return Some("F03");
    }
    if name.starts_with("HorizontalConcatenate")
        || name.starts_with("VerticalConcatenate")
        || name.starts_with("Set")
        || name.starts_with("Table")
        || matches!(
            name,
            "set/define"
                | "set/comprehension"
                | "matrix/comprehension"
                | "matrix/horzcat"
                | "matrix/vertcat"
        )
    {
        return Some("F06");
    }
    if name.starts_with("Convert") || name == "convert/kind" {
        return Some("F07");
    }
    if name.starts_with("VariableDefine") || name == "integrity/constraint" {
        return Some("F09");
    }
    if [
        "Add",
        "And",
        "Atan",
        "Atom",
        "Copysign",
        "Div",
        "Fdim",
        "Fmod",
        "EQ",
        "GT",
        "Jn",
        "LT",
        "Math",
        "Max",
        "Min",
        "Mod",
        "Mul",
        "NChooseK",
        "NEQ",
        "Negate",
        "Nextafter",
        "Not",
        "Or",
        "Pow",
        "Range",
        "Remainder",
        "Stats",
        "Sub",
        "Xor",
        "Yn",
        "compare/",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
    {
        return Some("F01");
    }
    None
}

#[cfg(any(feature = "standard_compiler", feature = "full_compiler"))]
fn r6_source_family(name: &str, intrinsic: bool) -> Option<&'static str> {
    if intrinsic {
        return if name.starts_with("access/") {
            Some("F04")
        } else if name.starts_with("assign") || name.ends_with("-assign") {
            Some("F05")
        } else if name.starts_with("convert/") {
            Some("F07")
        } else if name == "var/define" {
            Some("F09")
        } else {
            None
        };
    }
    if name.starts_with("string/") {
        Some("F03")
    } else if name.starts_with("table/")
        || name.starts_with("set/")
        || matches!(
            name,
            "matrix/comprehension" | "matrix/horzcat" | "matrix/vertcat"
        )
    {
        Some("F06")
    } else if matches!(
        name,
        "matrix/transpose" | "matrix/matmul" | "matrix/dot" | "matrix/solve"
    ) || name.starts_with("math/add-assign")
        || name.starts_with("math/sub-assign")
        || name.starts_with("math/mul-assign")
        || name.starts_with("math/div-assign")
    {
        Some("F02")
    } else if name.starts_with("math/")
        || name.starts_with("logic/")
        || name.starts_with("compare/")
        || name.starts_with("range/")
        || name.starts_with("stats/")
        || name.starts_with("combinatorics/")
    {
        Some("F01")
    } else {
        None
    }
}

#[cfg(any(feature = "standard_compiler", feature = "full_compiler"))]
#[test]
fn catalog_inventory_is_classified_into_r6_implementation_families() {
    let catalog = mech_stdlib::source_catalog();
    let mut family_counts = std::collections::BTreeMap::<&str, usize>::new();
    let mut unclassified = Vec::new();
    for entry in catalog.runtime_entries() {
        if let Some(family) = r6_runtime_family(entry) {
            *family_counts.entry(family).or_default() += 1;
        } else {
            unclassified.push(format!("runtime {} {:?}", entry.name, entry.signature()));
        }
    }
    for entry in catalog.specializer_entries() {
        if let Some(family) = r6_source_family(&entry.operation.canonical_name, false) {
            *family_counts.entry(family).or_default() += 1;
        } else {
            unclassified.push(format!("source {}", entry.operation.canonical_name));
        }
    }
    for entry in catalog.intrinsic_specializer_entries() {
        if let Some(family) = r6_source_family(&entry.operation.canonical_name, true) {
            *family_counts.entry(family).or_default() += 1;
        } else {
            unclassified.push(format!("intrinsic {}", entry.operation.canonical_name));
        }
    }
    assert_eq!(
        catalog.runtime_entries().len(),
        catalog.runtime_execution_capabilities().len(),
        "every concrete runtime registration must expose one target capability record",
    );
    assert!(
        unclassified.is_empty(),
        "unclassified maintained R6 registrations:\n{}",
        unclassified.join("\n"),
    );
    for family in ["F01", "F02", "F03", "F04", "F05", "F06", "F07", "F09"] {
        assert!(
            family_counts.get(family).copied().unwrap_or_default() > 0,
            "catalog profile has no classified {family} registration",
        );
    }
    eprintln!(
        "R6 catalog coverage: runtime={} capabilities={} specializers={} intrinsics={} families={family_counts:?}",
        catalog.runtime_entries().len(),
        catalog.runtime_execution_capabilities().len(),
        catalog.specializer_entries().len(),
        catalog.intrinsic_specializer_entries().len(),
    );
}

#[cfg(any(feature = "standard_compiler", feature = "full_compiler"))]
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

#[cfg(any(feature = "standard_compiler", feature = "full_compiler"))]
mod ordinary_managed_execution {
    use mech_core::*;
    use nalgebra::DMatrix;
    #[cfg(any(
        feature = "matrix1",
        feature = "matrix2",
        feature = "matrix3",
        feature = "matrix4",
        feature = "matrix2x3",
        feature = "matrix3x2",
        feature = "row_vector2",
        feature = "row_vector3",
        feature = "row_vector4",
        feature = "vector2",
        feature = "vector3",
        feature = "vector4",
    ))]
    use nalgebra::SMatrix;

    struct OversizedCanonicalTemporary {
        output: ValueCell,
    }

    impl MechFunctionImpl for OversizedCanonicalTemporary {
        fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
            Ok(Some(
                vec![self.output.current_memory_footprint()?].into_boxed_slice(),
            ))
        }

        fn solve_managed(
            &self,
            frame: &mut KernelMemoryFrame<'_>,
            _services: &mut dyn MechExecutionServices,
        ) -> MResult<ReactiveSolveStatus> {
            let footprint = self.output.current_memory_footprint()?;
            frame.with_admitted_canonical_output(
                &self.output,
                footprint,
                |_frame, construction| {
                    let oversized = construction
                        .remaining_temporary_bytes()
                        .checked_add(1)
                        .and_then(|bytes| usize::try_from(bytes).ok())
                        .ok_or_else(|| {
                            MechError::new(
                                MemoryPlanError::ArithmeticOverflow {
                                    field: "test canonical temporary bytes",
                                },
                                None,
                            )
                            .with_compiler_loc()
                        })?;
                    let _temporary = construction.try_vec_with_capacity::<u8>(oversized)?;
                    Ok((
                        (),
                        self.output
                            .rebuild_data_draft(ValueDataDraft::String("tiny".to_owned()))?,
                    ))
                },
            )?;
            Ok(ReactiveSolveStatus::Changed)
        }

        fn to_string(&self) -> String {
            "OversizedCanonicalTemporary".to_owned()
        }
    }

    impl MechFunctionCompiler for OversizedCanonicalTemporary {
        fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            compile_value_cell_register(&self.output, context)
        }
    }

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
        let instantiated = declaration.template.map(|template| {
            FunctionTypeDeclaration::from_schemes(
                instantiate_source_scheme_template(template, &original).unwrap(),
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

    fn concat(left: ValueCell, right: ValueCell) -> SpecializedFunction {
        specialize("string/concat", vec![left, right])
    }

    fn bind_runtime(name: &str, invocation: FunctionInvocation) -> SpecializedFunction {
        let catalog = mech_stdlib::source_catalog();
        let operation = OperationId::from_name(name);
        let mut candidates = catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(operation),
                ExecutionTarget::DirectRuntime,
            )
            .filter_map(|entry| {
                let parts = entry
                    .bind_resolved_invocation(
                        operation,
                        ExecutionTarget::DirectRuntime,
                        invocation.clone(),
                    )
                    .ok()?;
                Some((
                    parts,
                    entry.id,
                    entry.implementation_memory_class(),
                    entry.operation_contract(operation)?.clone(),
                ))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            candidates.len(),
            1,
            "expected one runtime binding for {name}"
        );
        let (parts, runtime, memory, contract) = candidates.pop().unwrap();
        SpecializedFunction::syntax_directed(
            parts,
            ResolvedOperationDescriptor::from_name(name, contract).unwrap(),
            runtime,
            ExecutionTarget::DirectRuntime,
            memory,
        )
        .unwrap()
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

    fn string_value(cell: &ValueCell) -> String {
        let value = cell.snapshot().unwrap();
        let ValueData::String(value) = value.data() else {
            panic!("expected String scalar")
        };
        value.to_string()
    }

    fn string_matrix_values(cell: &ValueCell) -> (Vec<u64>, Vec<String>) {
        let value = cell.snapshot().unwrap();
        let ValueData::Matrix(matrix) = value.data() else {
            panic!("expected String matrix")
        };
        let snapshot::SequenceView::String(values) = matrix.elements() else {
            panic!("expected String matrix elements")
        };
        (
            value.shape().parameter_values().to_vec(),
            values.iter().map(ToString::to_string).collect(),
        )
    }

    #[cfg(any(
        feature = "matrix1",
        feature = "matrix2",
        feature = "matrix3",
        feature = "matrix4",
        feature = "matrix2x3",
        feature = "matrix3x2",
        feature = "row_vector2",
        feature = "row_vector3",
        feature = "row_vector4",
        feature = "vector2",
        feature = "vector3",
        feature = "vector4",
    ))]
    fn execute_fixed_add<const ROWS: usize, const COLUMNS: usize>(
        storage: FunctionMatrixRepresentation,
        values: &[f64],
    ) where
        SMatrix<f64, ROWS, COLUMNS>: CanonicalCellBacking,
    {
        let catalog = mech_stdlib::source_catalog();
        let expected = FunctionValueRepresentation::Matrix {
            element: FunctionMatrixElement::F64,
            storage: FunctionMatrixStoragePattern::Exact(storage),
        };
        assert!(
            catalog
                .runtime_entries()
                .any(|entry| entry.signature().output == expected),
            "selected fixed-shape profile did not install an F64 {} runtime entry",
            storage.runtime_name(),
        );

        let session = MemoryDomain::new().unwrap();
        let input = ValueCell::from_exact_in(
            &session,
            SMatrix::<f64, ROWS, COLUMNS>::from_row_slice(values),
        )
        .unwrap();
        let function = add(input.clone(), input);
        function.instance().solve_result().unwrap();
        let (shape, result) = matrix_values(function.output());
        assert_eq!(shape, Vec::<u64>::new());
        assert_eq!(
            result,
            values.iter().map(|value| value * 2.0).collect::<Vec<_>>()
        );
    }

    #[cfg(any(
        feature = "matrix1",
        feature = "matrix2",
        feature = "matrix3",
        feature = "matrix4",
        feature = "matrix2x3",
        feature = "matrix3x2",
        feature = "row_vector2",
        feature = "row_vector3",
        feature = "row_vector4",
        feature = "vector2",
        feature = "vector3",
        feature = "vector4",
    ))]
    #[test]
    fn selected_fixed_shape_profile_installs_and_executes_its_managed_runtime_entry() {
        let mut executed = 0_u32;
        macro_rules! run {
            ($feature:literal, $storage:ident, $rows:literal, $columns:literal) => {
                #[cfg(feature = $feature)]
                {
                    let values = (1..=($rows * $columns))
                        .map(|value| value as f64)
                        .collect::<Vec<_>>();
                    execute_fixed_add::<$rows, $columns>(
                        FunctionMatrixRepresentation::$storage,
                        &values,
                    );
                    executed += 1;
                }
            };
        }
        run!("matrix1", Matrix1, 1, 1);
        run!("matrix2", Matrix2, 2, 2);
        run!("matrix3", Matrix3, 3, 3);
        run!("matrix4", Matrix4, 4, 4);
        run!("matrix2x3", Matrix2x3, 2, 3);
        run!("matrix3x2", Matrix3x2, 3, 2);
        run!("row_vector2", RowVector2, 1, 2);
        run!("row_vector3", RowVector3, 1, 3);
        run!("row_vector4", RowVector4, 1, 4);
        run!("vector2", Vector2, 2, 1);
        run!("vector3", Vector3, 3, 1);
        run!("vector4", Vector4, 4, 1);
        assert_ne!(executed, 0, "the fixed-shape test was compiled out");
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
    fn maintained_string_concat_replans_payload_growth_and_recovers_after_rejection() {
        let session = MemoryDomain::new().unwrap();
        let left = ValueCell::from_exact_in(&session, "a".to_owned()).unwrap();
        let right = ValueCell::from_exact_in(&session, "!".to_owned()).unwrap();
        let function = concat(left.clone(), right);
        let output = function.output().clone();

        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "a!");

        let grown = "same-shape-payload-growth".repeat(1_024);
        left.replace(
            &ValueCell::from_exact(grown.clone())
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), format!("{grown}!"));

        let before = string_value(&output);
        let version = output.published_version();
        left.replace(
            &ValueCell::from_exact("captured-rejected-candidate".repeat(512))
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        session
            .inject_failure_after(MemoryFailurePoint::Admission, 0)
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(string_value(&output), before);
        assert_eq!(output.published_version(), version);

        left.replace(
            &ValueCell::from_exact("valid".to_owned())
                .unwrap()
                .snapshot()
                .unwrap(),
        )
        .unwrap();
        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "valid!");

        let before = string_value(&output);
        let version = output.published_version();
        session
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 0)
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(string_value(&output), before);
        assert_eq!(output.published_version(), version);

        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "valid!");

        // The first checked allocation constructs the String draft. Allow it,
        // then fail the next checked allocation inside the common canonical
        // finalizer itself. Publication and ownership remain unchanged.
        let before = string_value(&output);
        let version = output.published_version();
        let ledger = session.ledger();
        session
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 1)
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(string_value(&output), before);
        assert_eq!(output.published_version(), version);
        assert_eq!(session.ledger(), ledger);

        function.instance().solve_result().unwrap();
        assert_eq!(string_value(&output), "valid!");
    }

    #[test]
    fn string_matrix_concat_and_transpose_share_admitted_canonical_construction() {
        let session = MemoryDomain::new().unwrap();
        let left = ValueCell::from_exact_in(
            &session,
            DMatrix::from_row_slice(
                2,
                2,
                &[
                    "a".to_owned(),
                    "b".to_owned(),
                    "c".to_owned(),
                    "d".to_owned(),
                ],
            ),
        )
        .unwrap();
        let right = ValueCell::from_exact_in(
            &session,
            DMatrix::from_row_slice(
                2,
                2,
                &[
                    "1".to_owned(),
                    "2".to_owned(),
                    "3".to_owned(),
                    "4".to_owned(),
                ],
            ),
        )
        .unwrap();
        let concat_output =
            ValueCell::from_exact_in(&session, DMatrix::from_element(2, 2, String::new())).unwrap();
        let concatenated = bind_runtime(
            "string/concat",
            FunctionInvocation::binary(concat_output, left.clone(), right.clone()),
        );
        let transpose_output =
            ValueCell::from_exact_in(&session, DMatrix::from_element(2, 2, String::new())).unwrap();
        let transposed = bind_runtime(
            "matrix/transpose",
            FunctionInvocation::unary(transpose_output, concatenated.output().clone()),
        );

        concatenated.instance().solve_result().unwrap();
        transposed.instance().solve_result().unwrap();
        assert_eq!(
            string_matrix_values(transposed.output()),
            (
                vec![2, 2],
                vec![
                    "a1".to_owned(),
                    "c3".to_owned(),
                    "b2".to_owned(),
                    "d4".to_owned(),
                ],
            )
        );

        left.replace(
            &ValueCell::from_exact(DMatrix::from_row_slice(
                2,
                2,
                &[
                    "left-a-".repeat(256),
                    "left-b-".repeat(256),
                    "left-c-".repeat(256),
                    "left-d-".repeat(256),
                ],
            ))
            .unwrap()
            .snapshot()
            .unwrap(),
        )
        .unwrap();
        concatenated.instance().solve_result().unwrap();
        transposed.instance().solve_result().unwrap();
        let before = string_matrix_values(transposed.output());
        let version = transposed.output().published_version();

        session
            .inject_failure_after(MemoryFailurePoint::HostAllocation, 0)
            .unwrap();
        assert!(transposed.instance().solve_result().is_err());
        assert_eq!(string_matrix_values(transposed.output()), before);
        assert_eq!(transposed.output().published_version(), version);

        transposed.instance().solve_result().unwrap();
        assert_eq!(
            string_matrix_values(transposed.output()).1,
            vec![
                format!("{}1", "left-a-".repeat(256)),
                format!("{}3", "left-c-".repeat(256)),
                format!("{}2", "left-b-".repeat(256)),
                format!("{}4", "left-d-".repeat(256)),
            ]
        );
    }

    #[test]
    fn canonical_construction_rejects_temporary_peak_before_allocation() {
        let session = MemoryDomain::new().unwrap();
        let output = ValueCell::from_exact_in(&session, "tiny".to_owned()).unwrap();
        let contract = OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(Box::new([])),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::AlwaysChanged,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        };
        let function = SpecializedFunction::syntax_directed(
            (
                Box::new(OversizedCanonicalTemporary {
                    output: output.clone(),
                }),
                FunctionInvocation::nullary(output.clone()),
            ),
            ResolvedOperationDescriptor::from_name("test/canonical-temporary", contract).unwrap(),
            RuntimeFunctionId::from_name("test/canonical-temporary"),
            ExecutionTarget::DirectRuntime,
            ImplementationMemoryClass::CanonicalFinalize,
        )
        .unwrap();
        let version = output.published_version();

        assert!(function.instance().solve_result().is_err());
        assert_eq!(string_value(&output), "tiny");
        assert_eq!(output.published_version(), version);
    }

    #[test]
    fn maintained_set_definition_preserves_its_specialized_frozen_output_without_write_access() {
        let session = MemoryDomain::new().unwrap();
        let function = specialize(
            "set/define",
            vec![
                ValueCell::from_exact_in(&session, "left".to_owned()).unwrap(),
                ValueCell::from_exact_in(&session, "right".to_owned()).unwrap(),
            ],
        );
        let output = function.output().clone();
        let before = output.snapshot().unwrap();
        let version = output.published_version();

        assert_eq!(
            function.instance().solve_reactive().unwrap(),
            ReactiveSolveStatus::Unchanged,
        );
        assert_eq!(output.published_version(), version);
        let after = output.snapshot().unwrap();
        assert!(
            before
                .language_eq(
                    &before.schemas().unwrap(),
                    &after,
                    &after.schemas().unwrap()
                )
                .unwrap()
        );
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

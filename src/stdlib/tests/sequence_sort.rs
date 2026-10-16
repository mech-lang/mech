#![cfg(all(feature = "full_compiler", feature = "sequence_sort"))]

use mech_core::{
    DimensionExpr, ExecutionTarget, FunctionExposure, FunctionInvocation,
    FunctionValueRepresentation, IntegerWidth, OperationId, ReactiveInstanceId,
    ResolvedOperationDescriptor, RuntimeFunctionId, RuntimeFunctionSignature, SchemaBody,
    SpecializedFunction, Value, ValueCell, ValueData, ValueDataDraft, snapshot::SequenceView,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const NAME: &str = "sequence/sort-indices";

fn matrix(element: SchemaBody, rows: u64, columns: u64, values: Vec<ValueDataDraft>) -> ValueCell {
    ValueCell::from_schema_data(
        SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: Box::new([
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]),
        },
        ValueDataDraft::Matrix(values.into_boxed_slice()),
    )
    .unwrap()
}
// A real Turn-scoped schema keeps its identity as the current cardinality
// changes. This exercises call-plan refresh rather than constructing a fresh
// function for each geometry.
fn turn_row(element: SchemaBody, values: Vec<ValueDataDraft>) -> ValueCell {
    use mech_core::{
        DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
        DimensionParameterOrigin, SchemaDraft, SchemaTableBuilder, ValueDraft,
        snapshot::SnapshotValidationContext,
    };
    let count = values.len() as u64;
    let schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(16)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![
                DimensionExpr::Constant(1),
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let mut builder = SchemaTableBuilder::new();
    let handle = builder.insert(schema).unwrap();
    let build = builder.finish().unwrap();
    let id = build.resolve(handle).unwrap();
    let (schemas, _) = build.into_parts();
    let value = ValueDraft {
        schema: id,
        shape_values: vec![count].into_boxed_slice(),
        data: ValueDataDraft::Matrix(values.into_boxed_slice()),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    ValueCell::from_snapshot(value).unwrap()
}

fn primary(rows: u64, columns: u64, values: &[u64]) -> ValueCell {
    matrix(
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        rows,
        columns,
        values.iter().copied().map(ValueDataDraft::U64).collect(),
    )
}
fn secondary(rows: u64, columns: u64, values: &[&str]) -> ValueCell {
    matrix(
        SchemaBody::String,
        rows,
        columns,
        values
            .iter()
            .map(|value| ValueDataDraft::String((*value).into()))
            .collect(),
    )
}
fn order(value: &str) -> ValueCell {
    ValueCell::from_schema_data(SchemaBody::String, ValueDataDraft::String(value.into())).unwrap()
}
fn output(n: u64) -> ValueCell {
    let seed = matrix(
        SchemaBody::Index,
        1,
        n,
        (0..n).map(|_| ValueDataDraft::Index(1)).collect(),
    );
    // This factory builds an owned canonical aggregate. A normal dense Index
    // cell is a distinct valid topology, covered by the rejection test below.
    let output = ValueCell::allocate_for_descriptor(
        &seed.resolved_descriptor().unwrap(),
        FunctionValueRepresentation::AnyValue,
    )
    .unwrap();
    output.replace(&seed.snapshot().unwrap()).unwrap();
    assert!(output.requires_canonical_output_builder().unwrap());
    output
}

fn assert_indices(value: &Value, expected: &[u64]) {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("{value:?}");
    };
    let SequenceView::Index(values) = matrix.elements() else {
        panic!("{value:?}");
    };
    assert_eq!(values, expected);
    let schemas = value.schemas().unwrap();
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schemas
        .get(value.schema())
        .unwrap()
        .closed_body(value.shape())
        .unwrap()
    else {
        panic!();
    };
    assert_eq!(element.as_ref(), &SchemaBody::Index);
    assert_eq!(
        dimensions.as_ref(),
        &[
            DimensionExpr::Constant(1),
            DimensionExpr::Constant(expected.len() as u64)
        ]
    );
}
fn managed(output: &ValueCell, inputs: &[ValueCell; 4]) -> SpecializedFunction {
    let catalog = mech_stdlib::source_catalog();
    let operation = OperationId::from_name(NAME);
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name(NAME))
        .unwrap();
    let invocation = FunctionInvocation::quaternary(
        output.clone(),
        inputs[0].clone(),
        inputs[1].clone(),
        inputs[2].clone(),
        inputs[3].clone(),
    );
    let parts = entry
        .bind_resolved_invocation(operation, ExecutionTarget::DirectRuntime, invocation)
        .unwrap();
    SpecializedFunction::syntax_directed(
        parts,
        ResolvedOperationDescriptor::from_name(
            NAME,
            entry.operation_contract(operation).unwrap().clone(),
        )
        .unwrap(),
        entry.id,
        ExecutionTarget::DirectRuntime,
        entry.implementation_memory_class(),
    )
    .unwrap()
}
fn compile(source: &str) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5351), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = product
        .document_outputs()
        .iter()
        .rev()
        .find(|row| row.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    (product.compile_artifact().unwrap(), output)
}

#[test]
fn ordinary_sort_has_one_exact_maintained_factory_and_module_export() {
    let catalog = mech_stdlib::source_catalog();
    let operation = OperationId::from_name(NAME);
    let export = catalog.module_export("sequence", "sort-indices").unwrap();
    assert_eq!(export.operation, operation);
    assert_eq!(export.exposure, FunctionExposure::ModuleOnly);
    assert!(catalog.specializer(operation).is_some());
    assert!(
        catalog
            .resident_factory(&["sequence".into()], "sort-indices")
            .is_some()
    );
    let entries = catalog
        .runtime_entries_for_binding(
            mech_core::RuntimeBindingSelector::Operation(operation),
            ExecutionTarget::DirectRuntime,
        )
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    assert_eq!(entry.name, NAME);
    assert_eq!(entry.id, RuntimeFunctionId::from_name(NAME));
    assert_eq!(
        entry.signature(),
        RuntimeFunctionSignature::quaternary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::String,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::String
        )
    );
    assert!(
        matches!(entry.operation_binding(), mech_core::RuntimeOperationBinding::Fixed(ids) if ids.as_ref() == [operation])
    );
    assert_eq!(
        entry.implementation_memory_class(),
        mech_core::ImplementationMemoryClass::CanonicalFinalize
    );
    let linkage = entry.native_linkage.as_ref().unwrap();
    assert_eq!(linkage.package, "mech-engine");
    assert_eq!(linkage.crate_name, "mech_engine");
    assert_eq!(
        linkage.installer_path,
        "mech_engine::__mech_native::install_sequence_sort_indices"
    );
    assert!(
        linkage
            .cargo_features
            .iter()
            .any(|feature| *feature == "sequence_sort")
    );
    let declaration = mech_core::maintained_source_type_declaration(NAME).unwrap();
    assert_eq!(catalog.source_type_declaration(NAME).unwrap(), &declaration);
}

#[test]
fn canonical_sort_factory_rejects_a_dense_index_output() {
    let dense = matrix(
        SchemaBody::Index,
        1,
        2,
        vec![ValueDataDraft::Index(1), ValueDataDraft::Index(1)],
    );
    assert!(!dense.requires_canonical_output_builder().unwrap());
    let before = dense.snapshot().unwrap();
    let version = dense.published_version();
    let inputs = [
        primary(1, 2, &[2, 1]),
        order("desc"),
        secondary(1, 2, &["b", "a"]),
        order("asc"),
    ];
    let catalog = mech_stdlib::source_catalog();
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name(NAME))
        .unwrap();
    let invocation = FunctionInvocation::quaternary(
        dense.clone(),
        inputs[0].clone(),
        inputs[1].clone(),
        inputs[2].clone(),
        inputs[3].clone(),
    );
    assert!(
        entry
            .bind_resolved_invocation(
                OperationId::from_name(NAME),
                ExecutionTarget::DirectRuntime,
                invocation
            )
            .is_err()
    );
    assert_eq!(dense.published_version(), version);
    let after = dense.snapshot().unwrap();
    assert!(
        before
            .snapshot_eq(
                &before.schemas().unwrap(),
                &after,
                &after.schemas().unwrap()
            )
            .unwrap()
    );
}

#[test]
fn managed_sort_is_exact_stable_and_accepts_distinct_equal_length_matrix_geometries() {
    let values = [
        u64::MAX,
        9_007_199_254_740_992,
        9_007_199_254_740_993,
        u64::MAX,
        u64::MAX,
        0,
    ];
    let keys = ["é", "x", "x", "A", "A", ""];
    for ((rows, columns), (key_rows, key_columns)) in
        [((2, 3), (1, 6)), ((1, 6), (3, 2)), ((6, 1), (2, 3))]
    {
        let result = output(6);
        let inputs = [
            primary(rows, columns, &values),
            order("desc"),
            secondary(key_rows, key_columns, &keys),
            order("asc"),
        ];
        let function = managed(&result, &inputs);
        function.instance().solve_result().unwrap();
        assert_indices(&result.snapshot().unwrap(), &[4, 5, 1, 3, 2, 6]);
        // Direction values remain ordinary runtime inputs, not source literals
        // or operation-name-selected algorithms.
        inputs[1]
            .replace(&order("asc").snapshot().unwrap())
            .unwrap();
        inputs[3]
            .replace(&order("desc").snapshot().unwrap())
            .unwrap();
        function.instance().solve_result().unwrap();
        assert_indices(&result.snapshot().unwrap(), &[6, 2, 3, 1, 4, 5]);
    }
    for (rows, columns) in [(1, 0), (0, 3), (5, 0)] {
        let result = output(0);
        let function = managed(
            &result,
            &[
                primary(rows, columns, &[]),
                order("desc"),
                secondary(0, 7, &[]),
                order("asc"),
            ],
        );
        function.instance().solve_result().unwrap();
        assert_indices(&result.snapshot().unwrap(), &[]);
    }
}

#[test]
fn managed_sort_refreshes_current_turn_geometry_and_demand_without_rebinding() {
    let result = turn_row(
        SchemaBody::Index,
        vec![ValueDataDraft::Index(1), ValueDataDraft::Index(1)],
    );
    let counts = |values: &[u64]| {
        turn_row(
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            values.iter().copied().map(ValueDataDraft::U64).collect(),
        )
    };
    let keys = |values: &[&str]| {
        turn_row(
            SchemaBody::String,
            values
                .iter()
                .map(|value| ValueDataDraft::String((*value).into()))
                .collect(),
        )
    };
    let inputs = [
        counts(&[2, 1]),
        order("desc"),
        keys(&["b", "a"]),
        order("asc"),
    ];
    let function = managed(&result, &inputs);
    function.instance().solve_result().unwrap();
    assert_indices(&result.snapshot().unwrap(), &[1, 2]);
    for _ in 0..2 {
        inputs[0]
            .replace(&counts(&[3, 3, u64::MAX, 0]).snapshot().unwrap())
            .unwrap();
        inputs[2]
            .replace(&keys(&["z", "a", "é", ""]).snapshot().unwrap())
            .unwrap();
        function.instance().solve_result().unwrap();
        assert_indices(&result.snapshot().unwrap(), &[3, 2, 1, 4]);
        inputs[0].replace(&counts(&[]).snapshot().unwrap()).unwrap();
        inputs[2].replace(&keys(&[]).snapshot().unwrap()).unwrap();
        function.instance().solve_result().unwrap();
        assert_indices(&result.snapshot().unwrap(), &[]);
    }
}

#[test]
fn invalid_directions_and_allocation_failures_preserve_managed_output() {
    let domain = mech_core::MemoryDomain::new().unwrap();
    let in_domain =
        |cell: ValueCell| ValueCell::from_snapshot_in(&domain, cell.snapshot().unwrap()).unwrap();
    let inputs = [
        in_domain(primary(1, 3, &[2, 1, 2])),
        in_domain(order("desc")),
        in_domain(secondary(3, 1, &["b", "z", "a"])),
        in_domain(order("asc")),
    ];
    let result = in_domain(output(3));
    let function = managed(&result, &inputs);
    function.instance().solve_result().unwrap();
    assert_indices(&result.snapshot().unwrap(), &[3, 1, 2]);
    inputs[1]
        .replace(&order("DESC").snapshot().unwrap())
        .unwrap();
    assert!(function.instance().solve_result().is_err());
    assert_indices(&result.snapshot().unwrap(), &[3, 1, 2]);
    inputs[1]
        .replace(&order("asc").snapshot().unwrap())
        .unwrap();
    for point in [
        mech_core::MemoryFailurePoint::Admission,
        mech_core::MemoryFailurePoint::HostAllocation,
    ] {
        let previous = result.snapshot().unwrap();
        let ledger = domain.ledger();
        let version = result.published_version();
        domain.inject_failure_after(point, 0).unwrap();
        assert!(function.instance().solve_result().is_err(), "{point:?}");
        assert_eq!(result.published_version(), version);
        assert_eq!(domain.ledger(), ledger);
        assert!(
            previous
                .snapshot_eq(
                    &previous.schemas().unwrap(),
                    &result.snapshot().unwrap(),
                    &result.snapshot().unwrap().schemas().unwrap()
                )
                .unwrap()
        );
        function.instance().solve_result().unwrap();
    }
}

#[test]
fn original_and_decoded_source_artifacts_use_the_same_exact_sort_semantics() {
    let source = "+> sequence\nsequence/sort-indices([18446744073709551615u64 9007199254740992u64 9007199254740993u64 18446744073709551615u64 18446744073709551615u64 0u64], \"desc\", [\"é\" \"x\" \"x\"; \"A\" \"A\" \"\"], \"asc\")\n";
    let (original, output) = compile(source);
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x5351, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert_indices(
            &instance.copied_output(output).unwrap(),
            &[4, 5, 1, 3, 2, 6],
        );
    }
}

#[test]
fn resident_sort_rejects_excessive_comparison_work_and_recovers_atomically() {
    let (original, output) = compile(
        "+> sequence\nsequence/sort-indices(counts<[u64]:1,2>, \"desc\", keys<[string]:2,1>, \"asc\")\n",
    );
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x5352, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let inputs = instance.plan.inputs.clone();
        let values = |long: bool| {
            [
                primary(1, 2, &[1, 1]).snapshot().unwrap(),
                secondary(2, 1, if long { &["a", "b"] } else { &["b", "a"] })
                    .snapshot()
                    .unwrap(),
            ]
        };
        let run = |instance: &mut mech_engine::resident::ReactiveInstance, values: &[Value]| {
            let rebound = values
                .iter()
                .zip(inputs.iter())
                .map(|(value, input)| {
                    value
                        .rebind(input.schema, &input.shape, artifact.schemas())
                        .unwrap()
                })
                .collect::<Vec<_>>();
            instance
                .prepare_turn_values(
                    &rebound
                        .iter()
                        .zip(inputs.iter())
                        .map(|(value, input)| CapturedValueInput {
                            slot: input.slot,
                            value,
                        })
                        .collect::<Vec<_>>(),
                )
                .and_then(|prepared| prepared.publish())
        };
        run(&mut instance, &values(false)).unwrap();
        assert_indices(&instance.copied_output(output).unwrap(), &[2, 1]);
        let width = 4096;
        let long = ["x".repeat(width), "y".repeat(width)];
        let denied = [
            primary(1, 2, &[1, 1]).snapshot().unwrap(),
            secondary(2, 1, &[&long[0], &long[1]]).snapshot().unwrap(),
        ];
        assert!(run(&mut instance, &denied).is_err());
        assert_indices(&instance.copied_output(output).unwrap(), &[2, 1]);
        run(&mut instance, &values(true)).unwrap();
        assert_indices(&instance.copied_output(output).unwrap(), &[1, 2]);
    }
}

#[test]
fn ordinary_sort_rejects_wrong_types_and_mismatched_cardinalities() {
    for source in [
        "+> sequence\nsequence/sort-indices([1.0 2.0], \"desc\", [\"a\" \"b\"], \"asc\")\n",
        "+> sequence\nsequence/sort-indices([1u64], \"desc\", [\"a\" \"b\"], \"asc\")\n",
        "+> sequence\nsequence/sort-indices([1u64], 0u64, [\"a\"], \"asc\")\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5353), Revision(0), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean());
        assert!(
            CanonicalSourceFrontend
                .compile_document_with_catalog(
                    &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                    mech_stdlib::source_catalog()
                )
                .is_err(),
            "{source}"
        );
    }
}

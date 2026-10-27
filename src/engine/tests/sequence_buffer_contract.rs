#![cfg(all(
    feature = "semantic-compiler",
    feature = "resident-artifact",
    feature = "sequence_buffers"
))]
use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueDataDraft as Data};
use mech_engine::{
    CanonicalSourceFrontend,
    resident::{ActivationFacts, activate},
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

#[test]
fn source_buffer_leaves_preserve_values_and_capacity_in_resident_execution() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    for width in [8, 16, 32, 64] {
        let scalar = |n| match width {
            8 => Data::U8(n as u8),
            16 => Data::U16(n as u16),
            32 => Data::U32(n as u32),
            _ => Data::U64(n),
        };
        for (expression, expected) in [
            (
                format!("sequence/with-capacity(0u{width}, 32u64)"),
                Data::Matrix(vec![].into()),
            ),
            (
                format!("sequence/reserve(sequence/with-capacity(0u{width}, 32u64), 33u64)"),
                Data::Matrix(vec![].into()),
            ),
            (
                format!("sequence/append(sequence/with-capacity(0u{width}, 32u64), 129u{width})"),
                Data::Matrix(vec![scalar(129)].into()),
            ),
            (
                format!(
                    "sequence/clear(sequence/append(sequence/with-capacity(0u{width}, 32u64), 129u{width}))"
                ),
                Data::Matrix(vec![].into()),
            ),
        ] {
            let source = format!("+> sequence\n{expression}\n");
            let parsed = parse_canonical_document(
                TextSnapshot::new(DocumentId(0x425546), Revision(1), source.as_str()).unwrap(),
                ParseConfig::default(),
            );
            assert!(
                parsed.is_strictly_clean(),
                "{source}: {:?}",
                parsed.diagnostics
            );
            let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
            let product = CanonicalSourceFrontend
                .compile_document_with_catalog(&syntax, catalog.clone())
                .unwrap();
            let artifact = product.compile_artifact().unwrap();
            let decoded = mech_engine::decode_program_artifact_bytecode_v1(
                &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
            )
            .unwrap();
            for artifact in [artifact, decoded] {
                let mut instance = activate(
                    ReactiveInstanceId::new(0x425546, 0),
                    &artifact,
                    &catalog,
                    &ActivationFacts::default(),
                )
                .unwrap();
                instance
                    .prepare_turn_values(&[])
                    .unwrap_or_else(|error| panic!("{source}: {error:?}"))
                    .publish()
                    .unwrap();
                let actual = instance
                    .copied_output(artifact.outputs().len() - 1)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap();
                assert_eq!(actual, expected, "{source}");
                let value = instance
                    .copied_output(artifact.outputs().len() - 1)
                    .unwrap();
                let mech_core::ValueData::Matrix(matrix) = value.data() else {
                    panic!("matrix result")
                };
                assert_eq!(
                    matrix.unsigned_buffer_capacity(),
                    Some(if expression.starts_with("sequence/reserve") {
                        33
                    } else {
                        32
                    })
                );
            }
        }
    }
}

#[test]
fn source_buffer_operations_reject_wrong_scalar_kinds() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    for expression in [
        "sequence/with-capacity(true, 1u64)",
        "sequence/with-capacity(0u8, 1u8)",
        "sequence/append(sequence/with-capacity(0u8, 1u64), 1u16)",
        "sequence/clear(1u64)",
        "sequence/append-le-bytes(sequence/with-capacity(0u16, 8u64), 1u64, 1u8)",
        "sequence/append-le-bytes(sequence/with-capacity(0u8, 8u64), 1u32, 1u8)",
        "sequence/append-le-bytes(sequence/with-capacity(0u8, 8u64), 1u64, 1u64)",
    ] {
        let source = format!("+> sequence\n{expression}\n");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x425546), Revision(1), source.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.is_strictly_clean(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
        assert!(
            CanonicalSourceFrontend
                .compile_document_with_catalog(&syntax, catalog.clone())
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn managed_buffer_factories_keep_current_capacity_and_preserve_inputs() {
    use mech_core::{
        DimensionExpr as D, ExecutionTarget, FunctionInvocation, FunctionValueRepresentation,
        IntegerWidth as W, OperationId, ResolvedOperationDescriptor, RuntimeFunctionId,
        SchemaBody as S, SpecializedFunction, ValueCell,
    };
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for width in [W::W8, W::W16, W::W32, W::W64] {
        let scalar = |n| match width {
            W::W8 => Data::U8(n as u8),
            W::W16 => Data::U16(n as u16),
            W::W32 => Data::U32(n as u32),
            _ => Data::U64(n),
        };
        let row = |values: Vec<Data>| {
            let cell = ValueCell::from_schema_data(
                S::Matrix {
                    element: Box::new(S::UnsignedInteger(width)),
                    dimensions: Box::new([D::Constant(1), D::Constant(values.len() as u64)]),
                },
                Data::Matrix(values.into()),
            )
            .unwrap();
            let output = ValueCell::allocate_for_descriptor(
                &cell.resolved_descriptor().unwrap(),
                FunctionValueRepresentation::AnyValue,
            )
            .unwrap();
            output.replace(&cell.snapshot().unwrap()).unwrap();
            output
        };
        for item in [
            "with-capacity",
            "reserve",
            "append",
            "clear",
            "append-le-bytes",
        ] {
            if item == "append-le-bytes" && width != W::W8 {
                continue;
            }
            let prototype =
                ValueCell::from_schema_data(S::UnsignedInteger(width), scalar(0)).unwrap();
            let source = row(vec![scalar(17)]);
            let argument = if item == "append" {
                ValueCell::from_schema_data(S::UnsignedInteger(width), scalar(129)).unwrap()
            } else {
                ValueCell::from_schema_data(
                    S::UnsignedInteger(W::W64),
                    Data::U64(if item == "append-le-bytes" { 129 } else { 32 }),
                )
                .unwrap()
            };
            let prototype = ValueCell::from_snapshot(prototype.snapshot().unwrap()).unwrap();
            let argument = ValueCell::from_snapshot(argument.snapshot().unwrap()).unwrap();
            let expected = match item {
                "with-capacity" | "clear" => vec![],
                "reserve" => vec![scalar(17)],
                "append-le-bytes" => vec![scalar(17), scalar(129), scalar(0)],
                _ => vec![scalar(17), scalar(129)],
            };
            let output = row(expected.clone());
            let name = format!("sequence/{item}");
            let operation = OperationId::from_name(&name);
            let entry = catalog
                .runtime_entry(RuntimeFunctionId::from_name(&name))
                .unwrap();
            let invocation = match item {
                "clear" => FunctionInvocation::unary(output.clone(), source.clone()),
                "append-le-bytes" => FunctionInvocation::ternary(
                    output.clone(),
                    source.clone(),
                    argument.clone(),
                    ValueCell::from_snapshot(
                        ValueCell::from_schema_data(S::UnsignedInteger(W::W8), Data::U8(2))
                            .unwrap()
                            .snapshot()
                            .unwrap(),
                    )
                    .unwrap(),
                ),
                "with-capacity" => {
                    FunctionInvocation::binary(output.clone(), prototype, argument.clone())
                }
                _ => FunctionInvocation::binary(output.clone(), source.clone(), argument.clone()),
            };
            let parts = entry
                .bind_resolved_invocation(operation, ExecutionTarget::DirectRuntime, invocation)
                .unwrap();
            let function = SpecializedFunction::syntax_directed(
                parts,
                ResolvedOperationDescriptor::from_name(
                    name.as_str(),
                    entry.operation_contract(operation).unwrap().clone(),
                )
                .unwrap(),
                entry.id,
                ExecutionTarget::DirectRuntime,
                entry.implementation_memory_class(),
            )
            .unwrap();
            function
                .instance()
                .solve_result()
                .unwrap_or_else(|error| panic!("{name} {width:?}: {error:?}"));
            assert_eq!(
                output.snapshot().unwrap().canonical_data_draft().unwrap(),
                Data::Matrix(expected.into())
            );
            assert_eq!(
                source.snapshot().unwrap().canonical_data_draft().unwrap(),
                Data::Matrix(vec![scalar(17)].into())
            );
            if item == "with-capacity" {
                argument
                    .replace(
                        &ValueCell::from_schema_data(S::UnsignedInteger(W::W64), Data::U64(64))
                            .unwrap()
                            .snapshot()
                            .unwrap(),
                    )
                    .unwrap();
                function.instance().solve_result().unwrap();
                let value = output.snapshot().unwrap();
                let mech_core::ValueData::Matrix(matrix) = value.data() else {
                    panic!()
                };
                assert_eq!(matrix.unsigned_buffer_capacity(), Some(64));
            }
        }
    }
}

#[test]
fn generic_buffer_row_checks_keep_published_resident_values_and_recover() {
    use mech_core::{ValueDraft, snapshot::SnapshotValidationContext};
    use mech_engine::resident::CapturedValueInput;
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    for item in ["reserve", "append", "clear"] {
        let suffix = match item {
            "reserve" => ", 4u64",
            "append" => ", 129u8",
            _ => "",
        };
        let source = format!("+> sequence\nsequence/{item}(values<[u8]>{suffix})\n");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x425546), Revision(1), source.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean());
        let product = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap();
        let original = product.compile_artifact().unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        for artifact in [original, decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x425546, 0),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let input = &artifact.inputs()[0];
            let slot = instance.plan.inputs[0].slot;
            for (rows, columns) in [(1, 2), (2, 1), (1, 3)] {
                let shape = mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(input.schema).unwrap(),
                    &[rows, columns],
                )
                .unwrap();
                let words = vec![Data::U8(17); (rows * columns) as usize];
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values: shape.parameter_values().to_vec().into(),
                    data: Data::Matrix(words.clone().into()),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                if rows == 2 {
                    let prior = instance
                        .copied_output(artifact.outputs().len() - 1)
                        .unwrap();
                    assert!(
                        instance
                            .prepare_turn_values(&[CapturedValueInput {
                                slot,
                                value: &value
                            }])
                            .is_err()
                    );
                    let after = instance
                        .copied_output(artifact.outputs().len() - 1)
                        .unwrap();
                    assert!(
                        after
                            .snapshot_eq(artifact.schemas(), &prior, artifact.schemas())
                            .unwrap()
                    );
                } else {
                    instance
                        .prepare_turn_values(&[CapturedValueInput {
                            slot,
                            value: &value,
                        }])
                        .unwrap()
                        .publish()
                        .unwrap();
                    let mut expected = words;
                    if item == "clear" {
                        expected.clear();
                    }
                    if item == "append" {
                        expected.push(Data::U8(129));
                    }
                    assert_eq!(
                        instance
                            .copied_output(artifact.outputs().len() - 1)
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Matrix(expected.into())
                    );
                }
                assert_eq!(
                    value.canonical_data_draft().unwrap(),
                    Data::Matrix(vec![Data::U8(17); (rows * columns) as usize].into())
                );
            }
        }
    }
}

#[test]
fn capacity_only_changes_propagate_through_reserve_clear_and_append() {
    use mech_core::{ValueDraft, snapshot::SnapshotValidationContext};
    use mech_engine::resident::CapturedValueInput;
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let source = "+> sequence\nsequence/append(sequence/clear(sequence/reserve(sequence/with-capacity(0u8, capacity<u64>), 3u64)), 7u8)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x425546), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let product = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap();
    let original = product.compile_artifact().unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x425546, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        let mut prior = None;
        for capacity in [32, 64, u64::MAX, 16] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: Data::U64(capacity),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            if capacity == u64::MAX {
                assert!(
                    instance
                        .prepare_turn_values(&[CapturedValueInput {
                            slot,
                            value: &value
                        }])
                        .is_err()
                );
                let current = instance
                    .copied_output(artifact.outputs().len() - 1)
                    .unwrap();
                assert!(current.shares_frozen_storage(prior.as_ref().unwrap()));
                continue;
            }
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot,
                    value: &value,
                }])
                .unwrap()
                .publish()
                .unwrap();
            let current = instance
                .copied_output(artifact.outputs().len() - 1)
                .unwrap();
            assert_eq!(
                current.canonical_data_draft().unwrap(),
                Data::Matrix(vec![Data::U8(7)].into())
            );
            let mech_core::ValueData::Matrix(matrix) = current.data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
            if let Some(previous) = &prior {
                assert!(
                    current
                        .snapshot_eq(artifact.schemas(), previous, artifact.schemas())
                        .unwrap()
                );
                assert!(!current.shares_frozen_storage(previous));
            }
            prior = Some(current);
        }
    }
}

#[test]
fn resident_bulk_byte_append_rejects_count_without_replacing_prior_output_and_recovers() {
    use mech_core::{ValueDraft, snapshot::SnapshotValidationContext};
    use mech_engine::resident::CapturedValueInput;
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let source = "+> sequence\nsequence/append-le-bytes(sequence/append(sequence/with-capacity(0u8, 16u64), 99u8), 578437695752307201u64, count<u8>)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x425546), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, catalog.clone())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x425546, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        let last = artifact.outputs().len() - 1;
        for count in [0u8, 8, 9, 3, 255, 0, 7] {
            let input = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: Data::U8(count),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            if count > 8 {
                let prior = instance.copied_output(last).unwrap();
                assert!(
                    instance
                        .prepare_turn_values(&[CapturedValueInput {
                            slot,
                            value: &input
                        }])
                        .is_err()
                );
                let after = instance.copied_output(last).unwrap();
                assert!(after.shares_frozen_storage(&prior));
                assert_eq!(
                    after.canonical_data_draft().unwrap(),
                    prior.canonical_data_draft().unwrap()
                );
            } else {
                instance
                    .prepare_turn_values(&[CapturedValueInput {
                        slot,
                        value: &input,
                    }])
                    .unwrap()
                    .publish()
                    .unwrap();
                let expected = std::iter::once(Data::U8(99))
                    .chain((1..=count).map(Data::U8))
                    .collect();
                assert_eq!(
                    instance
                        .copied_output(last)
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected)
                );
            }
        }
    }
}

#[test]
fn zero_byte_append_preserves_generic_input_shape_and_backing() {
    use mech_core::{ValueDraft, snapshot::SnapshotValidationContext};
    use mech_engine::resident::CapturedValueInput;
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let source = "+> sequence\nsequence/append-le-bytes(bytes<[u8]>, 123u64, 0u8)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x425546), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, catalog.clone())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x425546, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let schema = artifact.inputs()[0].schema;
    let shape =
        mech_core::shape_for_resolved_extents(artifact.schemas().get(schema).unwrap(), &[1, 2])
            .unwrap();
    let value = ValueDraft {
        schema,
        shape_values: shape.parameter_values().into(),
        data: Data::Matrix(vec![Data::U8(17), Data::U8(34)].into()),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    instance
        .prepare_turn_values(&[CapturedValueInput {
            slot: instance.plan.inputs[0].slot,
            value: &value,
        }])
        .unwrap()
        .publish()
        .unwrap();
    let output = instance
        .copied_output(artifact.outputs().len() - 1)
        .unwrap();
    assert!(
        output.shares_frozen_storage(&value),
        "input={:?} output={:?}",
        artifact.schemas().get(value.schema()),
        artifact.schemas().get(output.schema())
    );
    assert_eq!(output.resident_token(), value.resident_token());
    assert_eq!(output.shape(), value.shape());
}

#[test]
#[cfg(feature = "native-artifact")]
fn native_bulk_byte_append_current_counts_and_real_capacity_identity() {
    use mech_core::{
        ManagedMemoryBudget, MemoryBudgetLimits, ResolvedOperationContract, Value, ValueDraft,
        snapshot::{SnapshotValidationContext, UnsignedMatrixBuffer},
    };
    use mech_engine::native_artifact::{
        NativeCollectionPrimitive as P, NativeIterationAction as A, NativeLibraryWorkBudget,
        NativePreparedLibraryCallCache, lower_native_program_artifact,
        prepare_native_program_library_call,
    };
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_compiler_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = Arc::new(builder.build().unwrap());
    let text = "+> sequence\nsequence/append-le-bytes(bytes<[u8]>, word<u64>, count<u8>)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x425546), Revision(1), text).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, catalog.clone())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
    let template = plan
        .blocks()
        .iter()
        .find_map(|block| match block.action() {
            A::LibraryCall { template, .. } if template.primitive() == P::SequenceAppendLeBytes => {
                Some(template)
            }
            _ => None,
        })
        .unwrap();
    let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
    let ResolvedOperationContract::Declared(contract) = template.contract() else {
        panic!()
    };
    let schemas = Arc::new(artifact.schemas().clone());
    let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
    let limits = MemoryBudgetLimits {
        max_output_elements: Some(8_000_000),
        max_output_bytes: Some(64 * 1024 * 1024),
        max_temporary_bytes: Some(64 * 1024 * 1024),
        max_cloned_bytes: Some(64 * 1024 * 1024),
        max_retained_nodes: Some(1_000_000),
        max_comparison_work: Some(1_000_000_000_000),
        max_compute_work: Some(1_000_000_000_000),
        ..Default::default()
    };
    let admit = |value: Value| {
        let mut reservation = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        value.into_memory_budget(&mut reservation).unwrap()
    };
    let scalar = |index: usize, data| {
        admit(
            ValueDraft {
                schema: contract.inputs[index].schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
            .unwrap(),
        )
    };
    let shape = mech_core::shape_for_resolved_extents(&template.inputs()[0], &[1, 1]).unwrap();
    let mut bytes = Vec::with_capacity(6633832);
    bytes.push(99);
    let source = admit(
        Value::from_unsigned_matrix_buffer(
            contract.inputs[0].schema,
            shape,
            UnsignedMatrixBuffer::U8(bytes),
            schemas.clone(),
        )
        .unwrap(),
    );
    let word = scalar(1, Data::U64(578437695752307201));
    let work = NativeLibraryWorkBudget::new(
        limits.max_compute_work.unwrap(),
        limits.max_comparison_work.unwrap(),
        &memory,
    )
    .unwrap();
    let identity = call
        .unsigned_buffer_with_count(
            Some(source.clone()),
            0,
            Some(0),
            false,
            limits,
            &work,
            Default::default(),
        )
        .unwrap();
    assert!(identity.shares_frozen_storage(&source));
    drop(identity);
    for cached in [false, true] {
        let mut output = None;
        let mut cache = NativePreparedLibraryCallCache::default();
        for count in [3u8, 9, 0, 8, 2] {
            let inputs = [
                Some(source.clone()),
                Some(word.clone()),
                Some(scalar(2, Data::U8(count))),
            ];
            let prior = output.clone();
            let result = if cached {
                call.invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
            } else {
                call.invoke_current(&inputs, &mut output, limits, &work)
            };
            if count > 8 {
                assert!(result.is_err());
                assert!(
                    output
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(prior.as_ref().unwrap())
                );
            } else {
                result.unwrap();
                let expected = std::iter::once(Data::U8(99))
                    .chain((1..=count).map(Data::U8))
                    .collect();
                assert_eq!(
                    output.as_ref().unwrap().canonical_data_draft().unwrap(),
                    Data::Matrix(expected)
                );
                if count == 0 {
                    assert!(output.as_ref().unwrap().shares_frozen_storage(&source));
                }
            }
        }
    }
    drop((source, word, work));
    assert_eq!(memory.used_bytes(), 0);
}

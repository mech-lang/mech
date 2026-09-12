#![cfg(feature = "source")]

use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn find(node: SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if node.kind() == kind {
        return Some(node);
    }
    node.children().find_map(|child| find(child, kind))
}

fn expression(source: &str) -> ExpressionSyntax {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x544), Revision(1), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source:?}");
    assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source:?}");
    find(parsed.syntax(), SyntaxKind::Expression)
        .and_then(ExpressionSyntax::cast)
        .expect("canonical Expression")
}

use mech_core::{
    AliasPolicy, CardinalitySpec, ChangeDetectionPolicy, ExternalInteraction, InputPortLayout,
    IntegerWidth, OutputConstruction, SchemaBody, ShapeRule,
};

fn compile(source: &str) -> mech_engine::CanonicalSourceProgram {
    CanonicalSourceFrontend
        .compile_expression(&expression(source))
        .unwrap()
}

#[test]
fn collection_contracts_and_exact_schemas_reach_artifacts_without_providers() {
    for (source, operation, change) in [
        (
            "{1} ∪ {2}",
            "set/union",
            ChangeDetectionPolicy::AlwaysChanged,
        ),
        (
            "(|a<u8>|1u8|) ⋈ (|a<u8>|1u8|)",
            "table/join",
            ChangeDetectionPolicy::KernelReported,
        ),
    ] {
        let compiled = compile(source);
        let node = compiled.program().nodes.last().unwrap();
        assert_eq!(
            node.operation()
                .expect("ordinary operation fixture")
                .canonical_name(),
            operation
        );
        let contract = compiled.contracts().last().unwrap().as_ref().unwrap();
        assert_eq!(
            contract,
            &mech_core::maintained_operation_contract(operation, 2, false).unwrap()
        );
        assert!(matches!(&contract.inputs, InputPortLayout::Fixed(inputs) if inputs.len() == 2));
        assert_eq!(contract.interaction, ExternalInteraction::Pure);
        assert_eq!(contract.outputs.len(), 1);
        assert_eq!(
            contract.outputs[0].construction,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared
            }
        );
        assert_eq!(contract.outputs[0].alias, AliasPolicy::NoAlias);
        assert_eq!(contract.outputs[0].change_detection, change);
        let output = compiled
            .schemas()
            .get(compiled.program().outputs[0].schema)
            .unwrap()
            .body();
        match output {
            SchemaBody::Set {
                element,
                cardinality,
            } => {
                assert_eq!(
                    element.as_ref(),
                    &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
                );
                assert_eq!(
                    cardinality,
                    &CardinalitySpec::Dynamic {
                        upper_bound: Some(mech_core::DimensionExpr::Constant(2))
                    }
                );
            }
            SchemaBody::Table { columns, rows } => {
                assert_eq!(columns.len(), 1);
                assert_eq!(columns[0].name, "a");
                assert_eq!(
                    columns[0].schema,
                    SchemaBody::UnsignedInteger(IntegerWidth::W8)
                );
                assert_eq!(
                    rows,
                    &CardinalitySpec::Exact(mech_core::DimensionExpr::Parameter(
                        mech_core::DimensionParameterId::new(0)
                    ))
                );
            }
            other => panic!("unexpected collection schema: {other:?}"),
        }
        let artifact = compiled.compile_artifact().unwrap();
        assert_eq!(
            artifact
                .schemas()
                .get(artifact.outputs()[0].schema)
                .unwrap()
                .body(),
            output
        );
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let parsed = mech_core::ParsedProgram::from_bytes(&bytes).unwrap();
        let decoded = mech_engine::decode_program_artifact_sections(&parsed.artifact).unwrap();
        assert_eq!(
            decoded
                .schemas()
                .get(decoded.outputs()[0].schema)
                .unwrap()
                .body(),
            output
        );
        assert_eq!(decoded.contracts(), artifact.contracts());
    }
}

#[test]
fn shared_collection_families_have_only_their_declared_binary_arity() {
    for name in [
        "set/union",
        "set/intersection",
        "set/difference",
        "set/symmetric-difference",
        "set/cartesian-product",
        "table/join",
        "table/left-outer-join",
        "table/right-outer-join",
        "table/full-outer-join",
        "table/left-semi-join",
        "table/left-anti-join",
    ] {
        let contract = mech_core::maintained_operation_contract(name, 2, false).unwrap();
        let family = if name.starts_with("set/") {
            "set/union"
        } else {
            "table/join"
        };
        assert_eq!(
            contract,
            mech_core::maintained_operation_contract(family, 2, false).unwrap()
        );
        for arity in [0, 1, 3] {
            assert!(mech_core::maintained_operation_contract(name, arity, false).is_none());
        }
    }
    assert!(mech_core::maintained_operation_contract("set/undeclared", 2, false).is_none());
    assert!(mech_core::maintained_operation_contract("table/undeclared", 2, false).is_none());
}

#[cfg(all(feature = "table", feature = "semantic-compiler"))]
#[test]
fn table_source_catalog_contracts_equal_the_source_authority() {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for (name, factory) in [
        ("table/join", "TableJoinFxn::Inner"),
        ("table/left-outer-join", "TableJoinFxn::LeftOuter"),
        ("table/right-outer-join", "TableJoinFxn::RightOuter"),
        ("table/full-outer-join", "TableJoinFxn::FullOuter"),
        ("table/left-semi-join", "TableJoinFxn::LeftSemi"),
        ("table/left-anti-join", "TableJoinFxn::LeftAnti"),
    ] {
        let operation = mech_core::OperationId::from_name(name);
        let expected = mech_core::maintained_operation_contract(name, 2, false).unwrap();
        assert_eq!(
            catalog.specializer(operation).unwrap().operation.contract,
            expected
        );
        assert!(
            catalog
                .runtime_entry(mech_core::RuntimeFunctionId::from_name(factory))
                .is_some(),
            "{name} must retain its concrete provider"
        );
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn set_union_roundtrip_executes_canonical_values_and_changed_inputs() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for (source, expected_inputs) in [("{1} ∪ {2}", 0), ("{signal<f64>} ∪ {2}", 1)] {
        let compiled = compile(source);
        assert_eq!(compiled.program().inputs.len(), expected_inputs, "{source}");
        let artifact = compiled.compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let parsed = mech_core::ParsedProgram::from_bytes(&bytes).unwrap();
        let decoded = mech_engine::decode_program_artifact_sections(&parsed.artifact).unwrap();
        for artifact in [&artifact, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x552, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            assert_eq!(instance.plan.inputs.len(), expected_inputs, "{source}");
            for signal in [1.0, 2.0, 3.0] {
                let data = [signal];
                let inputs = instance
                    .plan
                    .inputs
                    .first()
                    .map(|input| CapturedSignalInput {
                        slot: input.slot,
                        value: ResidentValueRef::F64(&data),
                    })
                    .into_iter()
                    .collect::<Vec<_>>();
                instance.turn(&inputs).unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::Set(values) = output.data() else {
                    panic!("union must return a canonical set")
                };
                let values = values
                    .elements()
                    .iter()
                    .map(|value| match value.data() {
                        ValueData::F64(value) => value.to_f64(),
                        other => panic!("unexpected element: {other:?}"),
                    })
                    .collect::<Vec<_>>();
                let expected = if expected_inputs == 0 || signal == 1.0 {
                    vec![1.0, 2.0]
                } else if signal == 2.0 {
                    vec![2.0]
                } else {
                    vec![2.0, 3.0]
                };
                assert_eq!(values, expected, "{source}");
            }
        }
    }
}

#[cfg(all(feature = "resident-artifact", feature = "table"))]
#[test]
fn table_join_roundtrip_executes_literal_and_changed_u8_rows() {
    use mech_core::snapshot::{SequenceView, SnapshotValidationContext};
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData, ValueDataDraft,
        ValueDraft,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    for (source, expected_inputs) in [
        ("(|a<u8>|1u8|) ⋈ (|a<u8>|1u8|)", 0),
        ("(|a<u8>|signal<u8>|) ⋈ (|a<u8>|1u8|)", 1),
    ] {
        let compiled = compile(source);
        assert_eq!(compiled.program().inputs.len(), expected_inputs);
        let artifact = compiled.compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let parsed = mech_core::ParsedProgram::from_bytes(&bytes).unwrap();
        let decoded = mech_engine::decode_program_artifact_sections(&parsed.artifact).unwrap();
        for artifact in [&artifact, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x553, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            assert_eq!(instance.plan.inputs.len(), expected_inputs);
            for signal in [1, 2, 1] {
                let snapshot = if expected_inputs == 1 {
                    Some(
                        ValueDraft {
                            schema: artifact.inputs()[0].schema,
                            shape_values: Box::new([]),
                            data: ValueDataDraft::U8(signal),
                        }
                        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                        .unwrap(),
                    )
                } else {
                    None
                };
                let captured = [snapshot];
                let inputs = if expected_inputs == 1 {
                    vec![CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::Snapshot(&captured),
                    }]
                } else {
                    vec![]
                };
                instance
                    .turn(&inputs)
                    .unwrap_or_else(|error| panic!("{source}: {error:?}"));
                let output = instance.copied_output(0).unwrap();
                let ValueData::Table(table) = output.data() else {
                    panic!("join must return canonical table")
                };
                let SequenceView::U8(values) = table.column(0).unwrap() else {
                    panic!("column must retain u8")
                };
                let expected: &[u8] = if expected_inputs == 0 || signal == 1 {
                    &[1]
                } else {
                    &[]
                };
                assert_eq!(values, expected);
                assert_eq!(output.shape().parameter_values(), &[expected.len() as u64]);
            }
        }
    }
}

#[cfg(all(feature = "resident-artifact", feature = "table"))]
mod resident_table_cases {
    use super::compile;
    use mech_core::snapshot::{F64Bits, OptionDraft, SnapshotValidationContext, TableColumnDraft};
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, Value, ValueDataDraft as D,
        ValueDraft,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};

    fn artifact(source: &str) -> mech_engine::ProgramArtifact {
        let compiled = compile(source);
        let artifact = compiled.compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let parsed = mech_core::ParsedProgram::from_bytes(&bytes).unwrap();
        mech_engine::decode_program_artifact_sections(&parsed.artifact).unwrap()
    }

    fn catalog() -> mech_core::FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut builder).unwrap();
        builder.build().unwrap()
    }

    fn table_value(
        artifact: &mech_engine::ProgramArtifact,
        schema: mech_core::SchemaId,
        columns: Vec<(&str, Vec<D>)>,
    ) -> Value {
        let data = D::Table(
            columns
                .into_iter()
                .map(|(name, values)| TableColumnDraft {
                    name: name.to_owned(),
                    values: values.into_boxed_slice(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let shape = mech_core::shape_for_value_data(
            artifact.schemas().get(schema).unwrap(),
            &data,
            &[],
            None,
        )
        .unwrap();
        ValueDraft {
            schema,
            shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap()
    }

    fn optional(value: Option<u8>) -> D {
        D::Option(OptionDraft {
            present: value.is_some(),
            value: value.map(|value| Box::new(D::U8(value))),
        })
    }

    #[test]
    fn all_join_modes_preserve_columns_absence_duplicates_and_row_order() {
        let left = "(|a<u8> l<u8>|1u8 7u8|2u8 8u8|)";
        let right = "(|a<u8> r<u8>|1u8 9u8|3u8 10u8|)";
        let catalog = catalog();
        for (name, expected) in [
            (
                "join",
                vec![
                    ("a", vec![D::U8(1)]),
                    ("l", vec![D::U8(7)]),
                    ("r", vec![D::U8(9)]),
                ],
            ),
            (
                "left-outer-join",
                vec![
                    ("a", vec![D::U8(1), D::U8(2)]),
                    ("l", vec![D::U8(7), D::U8(8)]),
                    ("r", vec![optional(Some(9)), optional(None)]),
                ],
            ),
            (
                "right-outer-join",
                vec![
                    ("a", vec![D::U8(1), D::U8(3)]),
                    ("l", vec![optional(Some(7)), optional(None)]),
                    ("r", vec![D::U8(9), D::U8(10)]),
                ],
            ),
            (
                "full-outer-join",
                vec![
                    ("a", vec![D::U8(1), D::U8(2), D::U8(3)]),
                    (
                        "l",
                        vec![optional(Some(7)), optional(Some(8)), optional(None)],
                    ),
                    (
                        "r",
                        vec![optional(Some(9)), optional(None), optional(Some(10))],
                    ),
                ],
            ),
            (
                "left-semi-join",
                vec![("a", vec![D::U8(1)]), ("l", vec![D::U8(7)])],
            ),
            (
                "left-anti-join",
                vec![("a", vec![D::U8(2)]), ("l", vec![D::U8(8)])],
            ),
        ] {
            let source = format!("table/{name}({left}, {right})");
            let artifact = artifact(&source);
            let expected = table_value(&artifact, artifact.outputs()[0].schema, expected);
            let mut instance = activate(
                ReactiveInstanceId::new(0x554, 0),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            assert!(instance.plan.inputs.is_empty());
            for _ in 0..2 {
                instance
                    .turn(&[])
                    .unwrap_or_else(|error| panic!("{name}: {error:?}"));
                let actual = instance.copied_output(0).unwrap();
                assert!(
                    actual
                        .language_eq(artifact.schemas(), &expected, artifact.schemas())
                        .unwrap(),
                    "{name}: {actual:?}"
                );
                assert_eq!(actual.shape(), expected.shape());
            }
        }
        let artifact =
            artifact("(|a<u8> l<u8>|1u8 7u8|1u8 8u8|) ⋈ (|a<u8> r<u8>|1u8 9u8|1u8 10u8|)");
        let expected = table_value(
            &artifact,
            artifact.outputs()[0].schema,
            vec![
                ("a", vec![D::U8(1); 4]),
                ("l", vec![D::U8(7), D::U8(7), D::U8(8), D::U8(8)]),
                ("r", vec![D::U8(9), D::U8(10), D::U8(9), D::U8(10)]),
            ],
        );
        let mut instance = activate(
            ReactiveInstanceId::new(0x555, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert!(
            instance
                .copied_output(0)
                .unwrap()
                .language_eq(artifact.schemas(), &expected, artifact.schemas())
                .unwrap()
        );
    }

    #[test]
    fn table_join_admission_failure_preserves_publication_and_next_turn_recovers() {
        let artifact = artifact("left<|a<u8>|> ⋈ right<|a<u8>|>");
        assert_eq!(artifact.inputs().len(), 2);
        let mut instance = activate(
            ReactiveInstanceId::new(0x556, 0),
            &artifact,
            &catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 2);
        let input = |name: &str, count: usize| {
            let input = artifact
                .inputs()
                .iter()
                .find(|input| input.name == mech_engine::encode_source_input_name(name))
                .unwrap();
            let slot = instance
                .plan
                .inputs
                .iter()
                .find(|slot| slot.artifact_slot == input.slot)
                .unwrap()
                .slot;
            (
                slot,
                table_value(&artifact, input.schema, vec![("a", vec![D::U8(1); count])]),
            )
        };
        let (left, small_left) = input("left", 1);
        let (right, small_right) = input("right", 1);
        let (_, large_left) = input("left", 300);
        let (_, large_right) = input("right", 300);
        let small_left = [Some(small_left)];
        let small_right = [Some(small_right)];
        let large_left = [Some(large_left)];
        let large_right = [Some(large_right)];
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: left,
                    value: ResidentValueRef::Snapshot(&small_left),
                },
                CapturedSignalInput {
                    slot: right,
                    value: ResidentValueRef::Snapshot(&small_right),
                },
            ])
            .unwrap();
        let expected = instance.copied_output(0).unwrap();
        let error = instance
            .turn(&[
                CapturedSignalInput {
                    slot: left,
                    value: ResidentValueRef::Snapshot(&large_left),
                },
                CapturedSignalInput {
                    slot: right,
                    value: ResidentValueRef::Snapshot(&large_right),
                },
            ])
            .unwrap_err();
        assert!(
            matches!(
                error,
                mech_engine::resident::ResidentExecutionError::Kernel {
                    error: mech_core::ResidentKernelError::InvalidShape,
                    ..
                }
            ),
            "{error:?}"
        );
        assert!(
            instance
                .copied_output(0)
                .unwrap()
                .language_eq(artifact.schemas(), &expected, artifact.schemas())
                .unwrap()
        );
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: left,
                    value: ResidentValueRef::Snapshot(&small_left),
                },
                CapturedSignalInput {
                    slot: right,
                    value: ResidentValueRef::Snapshot(&small_right),
                },
            ])
            .unwrap();
        assert!(
            instance
                .copied_output(0)
                .unwrap()
                .language_eq(artifact.schemas(), &expected, artifact.schemas())
                .unwrap()
        );
        let empty = [Some(table_value(
            &artifact,
            artifact.inputs()[0].schema,
            vec![("a", vec![])],
        ))];
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::Snapshot(&empty),
                },
                CapturedSignalInput {
                    slot: instance.plan.inputs[1].slot,
                    value: ResidentValueRef::Snapshot(&small_right),
                },
            ])
            .unwrap();
        let empty_expected =
            table_value(&artifact, artifact.outputs()[0].schema, vec![("a", vec![])]);
        assert!(
            instance
                .copied_output(0)
                .unwrap()
                .language_eq(artifact.schemas(), &empty_expected, artifact.schemas())
                .unwrap()
        );
    }

    #[test]
    fn table_join_preserves_nested_optional_payloads_and_float_language_equality() {
        let artifact = artifact(
            "table/left-outer-join((|a<u8>|1u8|2u8|), (|a<u8> value<(u8,bool)?>|1u8 (7u8,true)|))",
        );
        let expected = table_value(
            &artifact,
            artifact.outputs()[0].schema,
            vec![
                ("a", vec![D::U8(1), D::U8(2)]),
                (
                    "value",
                    vec![
                        D::Option(OptionDraft {
                            present: true,
                            value: Some(Box::new(D::Tuple(
                                vec![D::U8(7), D::Bool(true)].into_boxed_slice(),
                            ))),
                        }),
                        optional(None),
                    ],
                ),
            ],
        );
        let mut instance = activate(
            ReactiveInstanceId::new(0x557, 0),
            &artifact,
            &catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert!(
            instance
                .copied_output(0)
                .unwrap()
                .language_eq(artifact.schemas(), &expected, artifact.schemas())
                .unwrap()
        );

        let artifact = self::artifact("(|a<f64>|signal<f64>|) ⋈ (|a<f64>|0|)");
        let mut instance = activate(
            ReactiveInstanceId::new(0x558, 0),
            &artifact,
            &catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        for (key, expected) in [
            (-0.0, vec![D::F64(F64Bits::from_f64(-0.0))]),
            (f64::NAN, vec![]),
            (0.0, vec![D::F64(F64Bits::from_f64(0.0))]),
        ] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[key]),
                }])
                .unwrap();
            let expected = table_value(
                &artifact,
                artifact.outputs()[0].schema,
                vec![("a", expected)],
            );
            assert!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .language_eq(artifact.schemas(), &expected, artifact.schemas())
                    .unwrap()
            );
        }
    }
    #[test]
    fn table_join_transports_nested_matrix_shape_witnesses() {
        use mech_core::snapshot::SequenceView;
        use mech_core::{DimensionExpr, SchemaBody, ValueData};
        let artifact = artifact(
            "╭─────────╮\n│ a │ original │ v │\n├─────────┤\n│ 1u8 │ (signal<[f64]:2,2>) │ (signal[[false true],:]) │\n╰─────────╯ ⋈ (|a<u8>|1u8|)",
        );
        let mut instance = activate(
            ReactiveInstanceId::new(0x559, 0),
            &artifact,
            &catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        for offset in [0.0, 10.0] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[
                        1.0 + offset,
                        3.0 + offset,
                        2.0 + offset,
                        4.0 + offset,
                    ]),
                }])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let SchemaBody::Table { columns, .. } = artifact
                .schemas()
                .get(output.schema())
                .unwrap()
                .closed_body(output.shape())
                .unwrap()
            else {
                panic!()
            };
            assert!(
                matches!(&columns[2].schema, SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [DimensionExpr::Constant(1), DimensionExpr::Constant(2)])
            );
            let ValueData::Table(table) = output.data() else {
                panic!()
            };
            let SequenceView::Values(values) = table.column(2).unwrap() else {
                panic!()
            };
            let [ValueData::Matrix(matrix)] = values else {
                panic!()
            };
            let SequenceView::F64(values) = matrix.elements() else {
                panic!()
            };
            assert_eq!(
                values
                    .iter()
                    .map(|value| value.to_f64())
                    .collect::<Vec<_>>(),
                [3.0 + offset, 4.0 + offset]
            );
        }
    }
    #[test]
    fn table_join_preserves_dynamic_payload_identity_from_another_schema_arena() {
        use mech_core::snapshot::SequenceView;
        use mech_core::{SchemaBody, SchemaDraft, SchemaTableBuilder, ValueData};
        let artifact = artifact("left<|a<u8> value<*>|> ⋈ (|a<u8> marker<*>|1u8 (payload<*>)|)");
        let mut builder = SchemaTableBuilder::new();
        let dynamic = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Dynamic,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let tuple = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(
                        vec![
                            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
                            SchemaBody::Bool,
                        ]
                        .into_boxed_slice(),
                    ),
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let dynamic = built.resolve(dynamic).unwrap();
        let tuple = built.resolve(tuple).unwrap();
        let (foreign, _) = built.into_parts();
        assert!(
            artifact
                .schemas()
                .find_by_key(foreign.entry(tuple).unwrap().key())
                .is_none(),
            "foreign payload schema must not be registered in the artifact"
        );
        let u8_schema = artifact
            .schemas()
            .entries()
            .position(|entry| {
                matches!(
                    entry.schema().body(),
                    SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8)
                )
            })
            .map(|id| mech_core::SchemaId::new(id as u32))
            .unwrap();
        let dynamic_schema = artifact
            .schemas()
            .find_by_key(foreign.entry(dynamic).unwrap().key())
            .unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x55a, 0),
            &artifact,
            &catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 2);
        for number in [7.0, 9.0] {
            let value = ValueDraft {
                schema: dynamic,
                shape_values: Box::new([]),
                data: D::Dynamic(Some(Box::new(ValueDraft {
                    schema: tuple,
                    shape_values: Box::new([]),
                    data: D::Tuple(
                        vec![D::F64(F64Bits::from_f64(number)), D::Bool(true)].into_boxed_slice(),
                    ),
                }))),
            }
            .finalize(&SnapshotValidationContext::new(&foreign))
            .unwrap();
            let key = ValueDraft {
                schema: u8_schema,
                shape_values: Box::new([]),
                data: D::U8(1),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let children = [
                (u8_schema, key.shape().clone()),
                (dynamic_schema, value.shape().clone()),
            ];
            let table_shape =
                mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                    artifact.inputs()[0].schema,
                    &children,
                    artifact.schemas(),
                )
                .unwrap();
            let constructor = mech_core::snapshot::CompositeSnapshotConstructor::bind(
                artifact.inputs()[0].schema,
                table_shape,
                &children,
                std::sync::Arc::new(artifact.schemas().clone()),
            )
            .unwrap();
            let value = constructor
                .construct(vec![key, value].into_boxed_slice(), None)
                .unwrap();
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::Snapshot(&[Some(value)]),
                    },
                    CapturedSignalInput {
                        slot: instance.plan.inputs[1].slot,
                        value: ResidentValueRef::Snapshot(&[Some(
                            ValueDraft {
                                schema: dynamic_schema,
                                shape_values: Box::new([]),
                                data: D::Dynamic(None),
                            }
                            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                            .unwrap(),
                        )]),
                    },
                ])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Table(table) = output.data() else {
                panic!()
            };
            let SequenceView::Values(values) = table.column(1).unwrap() else {
                panic!()
            };
            let [ValueData::Dynamic(payload)] = values else {
                panic!()
            };
            let payload = payload.value().unwrap();
            assert_eq!(payload.schema_key(), foreign.entry(tuple).unwrap().key());
            let ValueData::Tuple(items) = payload.data() else {
                panic!()
            };
            assert!(
                matches!(items.as_ref(), [ValueData::F64(actual), ValueData::Bool(true)] if actual.to_f64() == number)
            );
        }
    }
}

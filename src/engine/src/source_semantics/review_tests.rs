use super::*;
use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
use mech_syntax::document::{ParseConfig, RuleId, TextSnapshot};

fn parse<T: AstNode>(source: &str, rule: RuleId) -> T {
    fn find<T: AstNode>(node: SyntaxNode) -> Option<T> {
        T::cast(node.clone()).or_else(|| node.children().find_map(find::<T>))
    }
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x544), Revision(1), source).unwrap(),
        rule,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{source}");
    assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source}");
    find(parsed.syntax()).unwrap()
}

fn selected(definition: &str, expression: &str) -> CanonicalSourceProgram {
    let definition: VariableDefineSyntax = parse(definition, rules::VARIABLE_DEFINE);
    let expression: ExpressionSyntax = parse(expression, rules::EXPRESSION);
    let mut builder = SemanticBuilder::new(SourceSemanticAnchor::for_node(expression.syntax()));
    builder
        .declare_definition_input_annotations(&definition, &BTreeSet::new())
        .unwrap();
    builder.definition(&definition).unwrap();
    let value = builder.expression(&expression).unwrap().0;
    builder.publish("result", None, value, expression.syntax());
    builder.finish().unwrap()
}

#[test]
fn selection_semantics_preserve_element_geometry_and_chaining() {
    for (source, operation, dimensions) in [
        ("a[2]", "access/scalar", vec![]),
        ("a[1,2]", "access/scalar", vec![]),
        ("a[1,:]", "access/rows", vec![1, 2]),
        ("a[:,2]", "access/columns", vec![2, 1]),
        ("a[[1 2]]", "access/range", vec![2, 1]),
        ("a[[1 2],[1 2]]", "access/rectangle", vec![2, 2]),
        ("a[:,2][1]", "access/scalar", vec![]),
        ("a[:]", "access/range", vec![4, 1]),
        ("a[:,:]", "matrix/vertcat", vec![2, 2]),
    ] {
        let compiled = selected("a := [1 2; 3 4]", source);
        let node = compiled.program.nodes.last().unwrap();
        assert_eq!(node.operation.canonical_name(), operation, "{source}");
        let body = compiled
            .schemas
            .get(compiled.program.outputs[0].schema)
            .unwrap()
            .body();
        if dimensions.is_empty() {
            assert_eq!(body, &SchemaBody::FloatingPoint(FloatWidth::W64));
        } else {
            let SchemaBody::Matrix {
                element,
                dimensions: actual,
            } = body
            else {
                panic!("{source}: {body:?}")
            };
            assert_eq!(
                element.as_ref(),
                &SchemaBody::FloatingPoint(FloatWidth::W64)
            );
            assert_eq!(
                actual.as_ref(),
                dimensions
                    .into_iter()
                    .map(DimensionExpr::Constant)
                    .collect::<Vec<_>>(),
                "{source}"
            );
        }
        assert!(
            !compiled
                .program
                .nodes
                .iter()
                .any(|node| node.operation.canonical_name() == "access/index")
        );
        compiled.compile_artifact().unwrap();
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn selections_activate_and_return_selected_values() {
    use crate::resident::{ActivationFacts, activate};
    use mech_core::snapshot::SequenceView;
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ValueData};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, expected) in [
        ("a[2]", vec![3.0]),
        ("a[1,2]", vec![2.0]),
        ("a[1,:]", vec![1.0, 2.0]),
        ("a[:,2]", vec![2.0, 4.0]),
        ("a[[1 2]]", vec![1.0, 3.0]),
        ("a[1..=2]", vec![1.0, 3.0]),
        ("a[[2 1],[1 2]]", vec![3.0, 4.0, 1.0, 2.0]),
        ("a[[1 2],[1 2]]", vec![1.0, 2.0, 3.0, 4.0]),
        ("a[:,2][1]", vec![2.0]),
        ("a[:]", vec![1.0, 3.0, 2.0, 4.0]),
    ] {
        let compiled = selected("a := [1 2; 3 4]", source);
        let artifact = compiled.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x544, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        instance
            .turn(&[])
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let value = instance.copied_output(0).unwrap();
        let actual = match value.data() {
            ValueData::F64(value) => vec![value.to_f64()],
            ValueData::Matrix(matrix) => {
                let SequenceView::F64(values) = matrix.elements() else {
                    panic!()
                };
                values.iter().map(|value| value.to_f64()).collect()
            }
            other => panic!("{source}: {other:?}"),
        };
        assert_eq!(actual, expected, "{source}");
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn optional_runtime_inputs_states_and_computed_values_preserve_present_payloads() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::snapshot::SequenceView;
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (definition, expression, expected) in [
        ("a := signal<f64>", "[a _]", 4.0),
        ("a := signal<f64> + 1", "[a _]", 5.0),
        ("~a := 3", "[a _]", 3.0),
    ] {
        let compiled = selected(definition, expression);
        let artifact = compiled.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x544, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{definition}: {error:?}"));
        for signal in [4.0, 9.0] {
            let data = [signal];
            let inputs = instance
                .plan
                .inputs
                .iter()
                .map(|input| CapturedSignalInput {
                    slot: input.slot,
                    value: ResidentValueRef::F64(&data),
                })
                .collect::<Vec<_>>();
            instance
                .turn(&inputs)
                .unwrap_or_else(|error| panic!("{definition}: {error:?}"));
            let value = instance.copied_output(0).unwrap();
            let ValueData::Matrix(matrix) = value.data() else {
                panic!()
            };
            let SequenceView::Values(values) = matrix.elements() else {
                panic!()
            };
            let ValueData::Option(present) = &values[0] else {
                panic!()
            };
            let ValueData::Option(absent) = &values[1] else {
                panic!()
            };
            let expected = if definition.starts_with('~') {
                expected
            } else {
                expected + signal - 4.0
            };
            assert!(
                matches!(present.as_deref(), Some(ValueData::F64(value)) if value.to_f64() == expected)
            );
            assert!(absent.is_none());
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn optional_sets_activate_with_present_and_absent_keys() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    let compiled = selected("a := signal<f64>", "{a _}");
    let artifact = compiled.compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x544, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    for signal in [4.0, 9.0] {
        let data = [signal];
        let inputs = [CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value: ResidentValueRef::F64(&data),
        }];
        instance.turn(&inputs).unwrap();
        let value = instance.copied_output(0).unwrap();
        let ValueData::Set(set) = value.data() else {
            panic!()
        };
        assert_eq!(set.elements().len(), 2);
        assert!(
            set.elements()
                .iter()
                .any(|element| matches!(element.data(), ValueData::Option(None)))
        );
        assert!(set.elements().iter().any(|element| matches!(element.data(), ValueData::Option(Some(value)) if matches!(value.as_ref(), ValueData::F64(value) if value.to_f64() == signal))));
    }
    let duplicate = selected("a := 1", "{a a}").compile_artifact().unwrap();
    // Constant kernels execute during activation; canonical duplicate keys fail
    // before either an activation or a turn can publish the invalid set.
    assert!(
        activate(
            ReactiveInstanceId::new(0x545, 0),
            &duplicate,
            &catalog,
            &ActivationFacts::default()
        )
        .is_err()
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn field_selections_activate_heterogeneous_records_and_table_columns() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::snapshot::{NamedValueDraft, SequenceView, TableColumnDraft};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (definition, expression, data, expected) in [
        (
            "a := signal<(u8,bool)>",
            "a{2}",
            ValueDataDraft::Tuple(
                vec![ValueDataDraft::U8(7), ValueDataDraft::Bool(true)].into_boxed_slice(),
            ),
            vec![true],
        ),
        (
            "a := signal<{first<u8>,second<bool>}>",
            "a.second",
            ValueDataDraft::Record(
                vec![
                    NamedValueDraft {
                        name: "first".to_owned(),
                        value: ValueDataDraft::U8(7),
                    },
                    NamedValueDraft {
                        name: "second".to_owned(),
                        value: ValueDataDraft::Bool(true),
                    },
                ]
                .into_boxed_slice(),
            ),
            vec![true],
        ),
        (
            "a := signal<|first<u8>,second<bool>|:2>",
            "a.second",
            ValueDataDraft::Table(
                vec![
                    TableColumnDraft {
                        name: "first".to_owned(),
                        values: vec![ValueDataDraft::U8(7), ValueDataDraft::U8(8)]
                            .into_boxed_slice(),
                    },
                    TableColumnDraft {
                        name: "second".to_owned(),
                        values: vec![ValueDataDraft::Bool(true), ValueDataDraft::Bool(false)]
                            .into_boxed_slice(),
                    },
                ]
                .into_boxed_slice(),
            ),
            vec![true, false],
        ),
    ] {
        let compiled = selected(definition, expression);
        let selector = compiled.program.nodes.last().unwrap().inputs[1];
        let SourceValue::Constant(selector) = selector else {
            panic!()
        };
        if expression == "a.second" {
            assert!(
                matches!(compiled.constants.get(selector).unwrap().data(), ValueData::Id(id) if *id == mech_core::hash_str("second"))
            );
        } else {
            assert_eq!(
                mech_core::canonical_positional_ordinal(
                    compiled.constants.get(selector).unwrap().data()
                )
                .unwrap(),
                2
            );
        }
        let data = [Some(
            ValueDraft {
                schema: compiled.program.inputs[0].schema,
                shape_values: Box::new([]),
                data,
            }
            .finalize(&SnapshotValidationContext::new(&compiled.schemas))
            .unwrap(),
        )];
        let artifact = compiled.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x545, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{definition}: {error:?}"));
        instance
            .turn(&[CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::Snapshot(&data),
            }])
            .unwrap();
        let result = instance.copied_output(0).unwrap();
        let actual = match result.data() {
            ValueData::Bool(value) => vec![*value],
            ValueData::Matrix(value) => {
                let SequenceView::Bool(value) = value.elements() else {
                    panic!()
                };
                value.to_vec()
            }
            other => panic!("{other:?}"),
        };
        assert_eq!(actual, expected);
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn optional_runtime_peers_preserve_existing_layers_and_box_dynamic_payloads() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for expression in ["[a _]", "{a _}"] {
        let compiled = selected("a := signal<f64?>", expression);
        let artifact = compiled.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x546, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let data = [Some(
            ValueDraft {
                schema: compiled.program.inputs[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(ValueDataDraft::F64(F64Bits::from_f64(5.0)))),
                }),
            }
            .finalize(&SnapshotValidationContext::new(&compiled.schemas))
            .unwrap(),
        )];
        instance
            .turn(&[CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::Snapshot(&data),
            }])
            .unwrap();
        let value = instance.copied_output(0).unwrap();
        let elements = match value.data() {
            ValueData::Matrix(matrix) => {
                let mech_core::snapshot::SequenceView::Values(values) = matrix.elements() else {
                    panic!()
                };
                values.iter().collect::<Vec<_>>()
            }
            ValueData::Set(set) => set.elements().iter().map(|key| key.data()).collect(),
            other => panic!("{other:?}"),
        };
        assert_eq!(elements.len(), 2);
        assert!(
            elements
                .iter()
                .any(|value| matches!(value, ValueData::Option(None)))
        );
        assert!(elements.iter().any(|value| matches!(value, ValueData::Option(Some(payload)) if matches!(payload.as_ref(), ValueData::F64(value) if value.to_f64() == 5.0))));
    }
    let compiled = selected("a<*?> := signal<f64>", "a");
    let artifact = compiled.compile_artifact().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x547, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    for signal in [4.0, 9.0] {
        let data = [signal];
        instance
            .turn(&[CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::F64(&data),
            }])
            .unwrap();
        let value = instance.copied_output(0).unwrap();
        assert!(
            matches!(value.data(), ValueData::Option(Some(payload)) if matches!(payload.as_ref(), ValueData::Dynamic(value) if matches!(value.value().unwrap().data(), ValueData::F64(value) if value.to_f64() == signal)))
        );
    }
}

#[test]
fn latest_review_selection_rejects_nonpositional_and_mismatched_map_keys() {
    for (definition, source) in [
        ("a := [1 2]", "a[\"x\"]"),
        ("a := [1 2]", "a[[\"x\"]]"),
        ("a := (1,2)", "a[\"x\"]"),
        ("a := {1: true}", "a{\"x\"}"),
    ] {
        let definition: VariableDefineSyntax = parse(definition, rules::VARIABLE_DEFINE);
        let expression: ExpressionSyntax = parse(source, rules::EXPRESSION);
        let mut builder = SemanticBuilder::new(SourceSemanticAnchor::for_node(expression.syntax()));
        builder.definition(&definition).unwrap();
        let error = builder
            .expression(&expression)
            .err()
            .unwrap_or_else(|| panic!("{source} accepted an incompatible selector"));
        assert_eq!(
            error.code, "source-semantics/incompatible-selection-kind",
            "{source}: {error}"
        );
        assert_eq!(error.anchor.document, DocumentId(0x544));
        assert_eq!(error.anchor.revision, Revision(1));
    }
    for (definition, source) in [
        ("a := [1 2]", "a[1]"),
        ("a := [1 2]", "a[[1u8 2u8]]"),
        ("a := [1 2]", "a[[true false]]"),
        ("a := (1,true)", "a[1u8]"),
        ("a := {\"x\": true}", "a{\"x\"}"),
        ("a := {(true): 1}", "a{true}"),
    ] {
        selected(definition, source).compile_artifact().unwrap();
    }
}

#[test]
fn latest_review_select_all_roundtrips_linear_order_and_zero_extents() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (kind, snapshot) in [("f64", false), ("u8", true)] {
        for (rows, columns) in [(0, 3), (2, 0), (0, 0), (2, 3), (1, 3), (3, 1)] {
            for suffix in ["[:]", "[:,:]"] {
                let compiled = selected(
                    &format!("a := signal<[{kind}]:{rows},{columns}>"),
                    &format!("a{suffix}"),
                );
                let artifact = compiled.compile_artifact().unwrap();
                let bytes = crate::encode_program_artifact_bytecode_v1(&artifact).unwrap();
                let decoded = crate::decode_program_artifact_bytecode_v1(&bytes).unwrap();
                assert_eq!(
                    crate::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
                    bytes
                );
                for (route, artifact) in [("direct", &artifact), ("decoded", &decoded)] {
                    let witness = format!("{route} {kind} {rows}x{columns} {suffix}");
                    let mut instance = activate(
                        ReactiveInstanceId::new(0x54f, 0),
                        artifact,
                        &catalog,
                        &ActivationFacts::default(),
                    )
                    .unwrap_or_else(|error| panic!("{witness}: {error:?}"));
                    let body = artifact
                        .schemas()
                        .get(artifact.outputs()[0].schema)
                        .unwrap()
                        .body();
                    let SchemaBody::Matrix { dimensions, .. } = body else {
                        panic!("{body:?}")
                    };
                    let shape = if suffix == "[:]" {
                        [rows * columns, 1]
                    } else {
                        [rows, columns]
                    };
                    assert_eq!(
                        dimensions.as_ref(),
                        &shape.map(DimensionExpr::Constant),
                        "{witness}"
                    );
                    for offset in [0u8, 10] {
                        let canonical = (1..=rows * columns)
                            .map(|n| u8::try_from(n).unwrap() + offset)
                            .collect::<Vec<_>>();
                        let linear = (0..columns)
                            .flat_map(|column| {
                                (0..rows).map(move |row| {
                                    u8::try_from(row * columns + column + 1).unwrap() + offset
                                })
                            })
                            .collect::<Vec<_>>();
                        let dense = linear.iter().map(|n| f64::from(*n)).collect::<Vec<_>>();
                        let captured = if snapshot {
                            vec![Some(
                                ValueDraft {
                                    schema: artifact.inputs()[0].schema,
                                    shape_values: Box::new([]),
                                    data: ValueDataDraft::Matrix(
                                        canonical
                                            .iter()
                                            .map(|n| ValueDataDraft::U8(*n))
                                            .collect::<Vec<_>>()
                                            .into_boxed_slice(),
                                    ),
                                }
                                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                                .unwrap(),
                            )]
                        } else {
                            vec![]
                        };
                        instance
                            .turn(&[CapturedSignalInput {
                                slot: instance.plan.inputs[0].slot,
                                value: if snapshot {
                                    ResidentValueRef::Snapshot(&captured)
                                } else {
                                    ResidentValueRef::F64(&dense)
                                },
                            }])
                            .unwrap_or_else(|error| panic!("{witness}: {error:?}"));
                        let expected = if suffix == "[:]" { linear } else { canonical };
                        let expected = ValueDataDraft::Matrix(
                            expected
                                .into_iter()
                                .map(|n| {
                                    if snapshot {
                                        ValueDataDraft::U8(n)
                                    } else {
                                        ValueDataDraft::F64(F64Bits::from_f64(f64::from(n)))
                                    }
                                })
                                .collect::<Vec<_>>()
                                .into_boxed_slice(),
                        );
                        assert_eq!(
                            instance
                                .copied_output(0)
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            expected,
                            "{witness}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn select_all_derives_its_shape_from_a_symbolic_selected_source() {
    use crate::resident::{ActivationFacts, CapturedSignalInput, activate};
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef};
    let mut catalog = FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (mask, expected) in [
        ("[false false]", vec![]),
        ("[false true]", vec![4., 5., 6.]),
        ("[true true]", vec![1., 4., 2., 5., 3., 6.]),
    ] {
        let compiled = selected("a := signal<[f64]:2,3>", &format!("a[{mask},:][:]"));
        let artifact = compiled.compile_artifact().unwrap();
        let bytes = crate::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = crate::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        for artifact in [&artifact, &decoded] {
            assert!(
                !artifact
                    .schemas()
                    .get(artifact.outputs()[0].schema)
                    .unwrap()
                    .dimension_parameters()
                    .is_empty()
            );
            let mut instance = activate(
                ReactiveInstanceId::new(0x550, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            for offset in [0., 10.] {
                let input = [1., 4., 2., 5., 3., 6.].map(|value| value + offset);
                instance
                    .turn(&[CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::F64(&input),
                    }])
                    .unwrap();
                let output = instance.copied_output(0).unwrap();
                assert!(
                    matches!(output.schemas().unwrap().get(output.schema()).unwrap().closed_body(output.shape()).unwrap(), SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [DimensionExpr::Constant(expected.len() as u64), DimensionExpr::Constant(1)])
                );
                assert_eq!(
                    output.canonical_data_draft().unwrap(),
                    ValueDataDraft::Matrix(
                        expected
                            .iter()
                            .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value + offset)))
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    ),
                    "{mask}"
                );
            }
        }
    }
}

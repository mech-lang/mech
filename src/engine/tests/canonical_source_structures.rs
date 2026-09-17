#![cfg(all(feature = "source", feature = "resident-artifact"))]

use mech_core::{
    FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, SchemaBody, ValueData,
};
use mech_engine::__resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram};
use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, SyntaxNode, TextSnapshot, VariableDefineSyntax,
};

fn definition(source: &str) -> CanonicalSourceProgram {
    fn find(node: SyntaxNode) -> Option<VariableDefineSyntax> {
        VariableDefineSyntax::cast(node.clone()).or_else(|| node.children().find_map(find))
    }
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x550), Revision(1), source).unwrap(),
        rules::VARIABLE_DEFINE,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source}");
    CanonicalSourceFrontend
        .compile_definition(&find(parsed.syntax()).unwrap())
        .unwrap()
}

#[test]
fn dynamic_options_preserve_live_matrix_payloads_and_canonical_element_order() {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (rows, columns, initial, expected) in [
        (1, 2, vec![1.0, 2.0], vec![1.0, 2.0]),
        (2, 2, vec![1.0, 3.0, 2.0, 4.0], vec![1.0, 2.0, 3.0, 4.0]),
        (0, 2, vec![], vec![]),
    ] {
        let compiled = definition(&format!("x<*?> := signal<[f64]:{rows},{columns}>"));
        let artifact = compiled.compile_artifact().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x550, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for offset in [0.0, 10.0] {
            let data = initial
                .iter()
                .map(|value| value + offset)
                .collect::<Vec<_>>();
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&data),
                }])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Option(Some(payload)) = output.data() else {
                panic!("{output:?}")
            };
            let ValueData::Dynamic(payload) = payload.as_ref() else {
                panic!()
            };
            let payload = payload.value().unwrap();
            assert!(
                matches!(artifact.schemas().get(payload.schema()).unwrap().body(), SchemaBody::Matrix {element,dimensions} if element.as_ref() == &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) && dimensions.as_ref() == [mech_core::DimensionExpr::Constant(rows), mech_core::DimensionExpr::Constant(columns)])
            );
            let ValueData::Matrix(matrix) = payload.data() else {
                panic!()
            };
            let mech_core::snapshot::SequenceView::F64(values) = matrix.elements() else {
                panic!()
            };
            assert_eq!(
                values
                    .iter()
                    .map(|value| value.to_f64())
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|value| value + offset)
                    .collect::<Vec<_>>()
            );
        }
    }
}

fn roundtrip(source: &str) -> mech_engine::ProgramArtifact {
    let compiled = definition(source);
    assert!(
        compiled
            .program()
            .nodes
            .iter()
            .all(|node| !node.operation.canonical_name().starts_with("source/")),
        "{source}"
    );
    let artifact = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
        bytes
    );
    decoded
}

#[test]
fn canonical_composite_construction_roundtrips_and_tracks_live_children() {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in [
        "x := (signal<f64>, true)",
        "x := {z: signal<f64>, a: true}",
        "x := {2: signal<f64>, 1: 9}",
        "x := :some(signal<f64>)",
        "x := (|a<f64> b<bool>|signal<f64> true|9 false|)",
        "x := ({value: signal<f64>}, {2: signal<f64>, 1: 9})",
    ] {
        let artifact = roundtrip(source);
        let mut instance = activate(
            ReactiveInstanceId::new(0x551, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        for value in [3.0, 7.0] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[value]),
                }])
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let output = instance.copied_output(0).unwrap();
            let data = output.canonical_data_draft().unwrap();
            use mech_core::ValueDataDraft as D;
            let f = |value| D::F64(mech_core::snapshot::F64Bits::from_f64(value));
            if source == "x := (signal<f64>, true)" {
                assert_eq!(
                    data,
                    D::Tuple(vec![f(value), D::Bool(true)].into_boxed_slice())
                );
            } else if source == "x := {z: signal<f64>, a: true}" {
                let D::Record(fields) = data else {
                    panic!("{data:?}")
                };
                assert_eq!(
                    fields
                        .iter()
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>(),
                    ["z", "a"]
                );
                assert_eq!(fields[0].value, f(value));
                assert_eq!(fields[1].value, D::Bool(true));
            } else if source == "x := {2: signal<f64>, 1: 9}" {
                let D::Map(entries) = data else {
                    panic!("{data:?}")
                };
                assert_eq!(entries[0].items.as_ref(), &[f(1.0), f(9.0)]);
                assert_eq!(entries[1].items.as_ref(), &[f(2.0), f(value)]);
            } else if source == "x := :some(signal<f64>)" {
                let SchemaBody::Tuple(fields) =
                    artifact.schemas().get(output.schema()).unwrap().body()
                else {
                    panic!()
                };
                let path = mech_core::CanonicalNominalPath::new(vec!["some".to_owned()]).unwrap();
                assert_eq!(
                    fields[0],
                    SchemaBody::Atom(mech_core::NominalKey::from_path(
                        mech_core::NominalKind::Atom,
                        &path
                    ))
                );
                assert_eq!(data, D::Tuple(vec![D::Atom, f(value)].into_boxed_slice()));
            } else if source.contains("|a<") {
                let D::Table(columns) = data else {
                    panic!("{data:?}")
                };
                assert_eq!(columns[0].name, "a");
                assert_eq!(columns[0].values.as_ref(), &[f(value), f(9.0)]);
                assert_eq!(columns[1].name, "b");
                assert_eq!(columns[1].values.as_ref(), &[D::Bool(true), D::Bool(false)]);
            } else {
                let D::Tuple(children) = data else {
                    panic!("{data:?}")
                };
                let D::Record(fields) = &children[0] else {
                    panic!()
                };
                assert_eq!(fields[0].value, f(value));
                let D::Map(entries) = &children[1] else {
                    panic!()
                };
                assert_eq!(entries[1].items[1], f(value));
            }
        }
    }
}

#[test]
fn canonical_map_duplicate_live_keys_fail_before_publication() {
    let artifact = roundtrip("x := {key<f64>: 10, 2: 20}");
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x552, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::F64(&[1.0]),
        }])
        .unwrap();
    let previous = instance.copied_output(0).unwrap();
    assert!(
        matches!(instance.turn(&[CapturedSignalInput {slot, value:ResidentValueRef::F64(&[2.0])}]), Err(mech_engine::__resident::ResidentExecutionError::Kernel { node, error: mech_core::ResidentKernelError::InvalidInput }) if node.get() == 0)
    );
    assert!(
        previous
            .language_eq(
                artifact.schemas(),
                &instance.copied_output(0).unwrap(),
                artifact.schemas()
            )
            .unwrap()
    );
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::F64(&[3.0]),
        }])
        .unwrap();
    let output = instance.copied_output(0).unwrap();
    let ValueData::Map(map) = output.data() else {
        panic!()
    };
    assert!(matches!(map.entries()[0].key().data(),ValueData::F64(value) if value.to_f64() == 2.0));
    assert!(matches!(map.entries()[1].key().data(),ValueData::F64(value) if value.to_f64() == 3.0));
}

#[test]
fn compound_record_and_table_annotations_preserve_live_matrix_children() {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in [
        "x := {value<[f64]:2,2>: signal<[f64]:2,2>}",
        "x := (|value<[f64]:2,2>|signal<[f64]:2,2>|)",
    ] {
        let artifact = roundtrip(source);
        let mut instance = activate(
            ReactiveInstanceId::new(0x553, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for offset in [0.0, 10.0] {
            let values = [1.0 + offset, 3.0 + offset, 2.0 + offset, 4.0 + offset];
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&values),
                }])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let (child, schema) = match (
                output.data(),
                artifact.schemas().get(output.schema()).unwrap().body(),
            ) {
                (ValueData::Record(record), SchemaBody::Record(fields)) => {
                    (&record.fields()[0], &fields[0].schema)
                }
                (ValueData::Table(_), SchemaBody::Table { columns, .. }) => {
                    let mech_core::ValueDataDraft::Table(table) =
                        output.canonical_data_draft().unwrap()
                    else {
                        panic!()
                    };
                    let mech_core::ValueDataDraft::Matrix(matrix) = &table[0].values[0] else {
                        panic!()
                    };
                    assert_eq!(
                        matrix
                            .iter()
                            .map(|value| match value {
                                mech_core::ValueDataDraft::F64(value) => value.to_f64(),
                                _ => panic!(),
                            })
                            .collect::<Vec<_>>(),
                        [1.0 + offset, 2.0 + offset, 3.0 + offset, 4.0 + offset]
                    );
                    assert!(
                        matches!(&columns[0].schema,SchemaBody::Matrix { dimensions,.. } if dimensions.as_ref()==[mech_core::DimensionExpr::Constant(2),mech_core::DimensionExpr::Constant(2)])
                    );
                    continue;
                }
                _ => panic!("{output:?}"),
            };
            assert!(
                matches!(schema,SchemaBody::Matrix { dimensions,.. } if dimensions.as_ref()==[mech_core::DimensionExpr::Constant(2),mech_core::DimensionExpr::Constant(2)])
            );
            let ValueData::Matrix(matrix) = child else {
                panic!()
            };
            let mech_core::snapshot::SequenceView::F64(values) = matrix.elements() else {
                panic!()
            };
            assert_eq!(
                values
                    .iter()
                    .map(|value| value.to_f64())
                    .collect::<Vec<_>>(),
                [1.0 + offset, 2.0 + offset, 3.0 + offset, 4.0 + offset]
            );
        }
    }
}

#[test]
fn empty_tuple_construction_has_no_synthetic_template_dependency() {
    let artifact = roundtrip("x := ()");
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x554, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    instance.turn(&[]).unwrap();
    assert!(
        matches!(instance.copied_output(0).unwrap().data(),ValueData::Tuple(items) if items.is_empty())
    );
}

#[test]
fn dynamic_table_children_preserve_nested_values_and_wrap_existing_dynamic_once() {
    use mech_core::snapshot::{F64Bits, SequenceView, SnapshotValidationContext};
    use mech_core::{SchemaDraft, SchemaTableBuilder, ValueDataDraft as D, ValueDraft};
    let artifact = roundtrip("x := (|value<*>|(signal<f64>, true)|(payload<*>)|)");
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x555, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = |name: &str| {
        let input = artifact
            .inputs()
            .iter()
            .find(|input| input.name == mech_engine::encode_source_input_name(name))
            .unwrap();
        instance
            .plan
            .inputs
            .iter()
            .find(|bound| bound.artifact_slot == input.slot)
            .unwrap()
            .slot
    };
    let signal = slot("signal");
    let payload = slot("payload");
    let mut builder = SchemaTableBuilder::new();
    let mut add = |body| {
        builder
            .insert(
                SchemaDraft {
                    body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    };
    let dynamic = add(SchemaBody::Dynamic);
    let tuple = add(SchemaBody::Tuple(
        vec![
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
            SchemaBody::Bool,
        ]
        .into_boxed_slice(),
    ));
    add(SchemaBody::String);
    let built = builder.finish().unwrap();
    let dynamic = built.resolve(dynamic).unwrap();
    let tuple = built.resolve(tuple).unwrap();
    let (schemas, _) = built.into_parts();
    for number in [3.0, 7.0] {
        let value = ValueDraft {
            schema: dynamic,
            shape_values: Box::new([]),
            data: D::Dynamic(Some(Box::new(ValueDraft {
                schema: tuple,
                shape_values: Box::new([]),
                data: D::Tuple(
                    vec![D::F64(F64Bits::from_f64(number * 10.0)), D::Bool(false)]
                        .into_boxed_slice(),
                ),
            }))),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let captured = [Some(value)];
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: signal,
                    value: ResidentValueRef::F64(&[number]),
                },
                CapturedSignalInput {
                    slot: payload,
                    value: ResidentValueRef::Snapshot(&captured),
                },
            ])
            .unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::Table(table) = output.data() else {
            panic!()
        };
        let SequenceView::Values(values) = table.column(0).unwrap() else {
            panic!()
        };
        assert_eq!(values.len(), 2);
        for (value, expected, flag) in [
            (&values[0], number, true),
            (&values[1], number * 10.0, false),
        ] {
            let ValueData::Dynamic(dynamic) = value else {
                panic!("{value:?}")
            };
            let payload = dynamic.value().unwrap();
            assert!(
                matches!(payload.schemas().unwrap().get(payload.schema()).unwrap().body(),SchemaBody::Tuple(items) if items.as_ref()==[SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),SchemaBody::Bool])
            );
            assert!(
                matches!(payload.data(),ValueData::Tuple(items) if matches!(&items[0],ValueData::F64(value) if value.to_f64()==expected) && matches!(&items[1],ValueData::Bool(value) if *value==flag))
            );
        }
    }
}

#[test]
fn composed_selection_shapes_survive_artifact_roundtrip_and_changing_inputs() {
    use mech_core::snapshot::SequenceView;
    let source = "x := (signal<[f64]:2,2>, signal[[false true],:])";
    let direct = definition(source).compile_artifact().unwrap();
    let decoded = roundtrip(source);
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let activations = [&direct, &decoded].map(|artifact| {
        activate(
            ReactiveInstanceId::new(0x556, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
    });
    assert!(
        activations.iter().all(Result::is_ok),
        "direct/decoded activation: {:?}",
        activations
            .iter()
            .map(|result| result.as_ref().err())
            .collect::<Vec<_>>()
    );
    for (artifact, activation) in [&direct, &decoded].into_iter().zip(activations) {
        let mut instance = activation.unwrap();
        for offset in [0.0, 10.0] {
            let data = [1.0 + offset, 3.0 + offset, 2.0 + offset, 4.0 + offset];
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&data),
                }])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Tuple(items) = output.data() else {
                panic!("{output:?}")
            };
            let SchemaBody::Tuple(schemas) = artifact
                .schemas()
                .get(output.schema())
                .unwrap()
                .closed_body(output.shape())
                .unwrap()
            else {
                panic!()
            };
            for (index, expected, rows) in [
                (
                    0,
                    vec![1.0 + offset, 2.0 + offset, 3.0 + offset, 4.0 + offset],
                    2,
                ),
                (1, vec![3.0 + offset, 4.0 + offset], 1),
            ] {
                let ValueData::Matrix(matrix) = &items[index] else {
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
                    expected
                );
                assert!(
                    matches!(&schemas[index], SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [mech_core::DimensionExpr::Constant(rows), mech_core::DimensionExpr::Constant(2)])
                );
            }
        }
    }
}

#[test]
fn composed_live_mask_shapes_require_facts_and_reject_extent_changes_atomically() {
    use mech_core::snapshot::SequenceView;
    use mech_engine::__resident::{ResidentActivationError, ResidentExecutionError};
    let source = "x := (signal<[f64]:2,2>, signal[mask<[bool]:1,2>,:])";
    let direct = definition(source).compile_artifact().unwrap();
    let decoded = roundtrip(source);
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&direct, &decoded] {
        let selected = artifact
            .slots()
            .iter()
            .find(|slot| {
                let schema = artifact.schemas().get(slot.schema).unwrap();
                matches!(schema.body(), SchemaBody::Matrix { .. })
                    && !schema.dimension_parameters().is_empty()
            })
            .unwrap();
        assert!(
            matches!(activate(ReactiveInstanceId::new(0x557, 0), artifact, &catalog, &ActivationFacts::default()),
            Err(ResidentActivationError::UnresolvedShape { slot }) if slot == selected.slot)
        );
        let mut facts = ActivationFacts::default();
        facts.slot_shapes.insert(
            selected.slot,
            artifact
                .schemas()
                .get(selected.schema)
                .unwrap()
                .instantiate_shape(Box::new([1]))
                .unwrap(),
        );
        // Only the selected child has a supplied fact. The tuple must derive
        // its independently numbered parameters from that concrete child.
        assert_eq!(facts.slot_shapes.len(), 1);
        let mut instance = activate(
            ReactiveInstanceId::new(0x557, 0),
            artifact,
            &catalog,
            &facts,
        )
        .unwrap();
        let slot = |name: &str| {
            let declaration = artifact
                .inputs()
                .iter()
                .find(|input| input.name == mech_engine::encode_source_input_name(name))
                .unwrap();
            instance
                .plan
                .inputs
                .iter()
                .find(|input| input.artifact_slot == declaration.slot)
                .unwrap()
                .slot
        };
        let signal_slot = slot("signal");
        let mask_slot = slot("mask");
        let source_data = [1.0, 3.0, 2.0, 4.0];
        let selected_values = |value: &mech_core::Value| {
            let ValueData::Tuple(children) = value.data() else {
                panic!()
            };
            let ValueData::Matrix(selected) = &children[1] else {
                panic!()
            };
            let SequenceView::F64(values) = selected.elements() else {
                panic!()
            };
            values
                .iter()
                .map(|value| value.to_f64())
                .collect::<Vec<_>>()
        };
        for (mask, offset, expected) in [([1u8, 0], 0.0, [1.0, 2.0]), ([0, 1], 10.0, [13.0, 14.0])]
        {
            let data = source_data.map(|value| value + offset);
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: signal_slot,
                        value: ResidentValueRef::F64(&data),
                    },
                    CapturedSignalInput {
                        slot: mask_slot,
                        value: ResidentValueRef::Bool(&mask),
                    },
                ])
                .unwrap();
            assert_eq!(
                selected_values(&instance.copied_output(0).unwrap()),
                expected
            );
        }
        let previous = instance.copied_output(0).unwrap();
        for mask in [[1u8, 1], [0, 0]] {
            assert!(matches!(
                instance.turn(&[
                    CapturedSignalInput {
                        slot: signal_slot,
                        value: ResidentValueRef::F64(&source_data)
                    },
                    CapturedSignalInput {
                        slot: mask_slot,
                        value: ResidentValueRef::Bool(&mask)
                    },
                ]),
                Err(ResidentExecutionError::Kernel {
                    error: mech_core::ResidentKernelError::InvalidShape,
                    ..
                })
            ));
            assert!(
                previous
                    .language_eq(
                        artifact.schemas(),
                        &instance.copied_output(0).unwrap(),
                        artifact.schemas()
                    )
                    .unwrap()
            );
        }
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: signal_slot,
                    value: ResidentValueRef::F64(&source_data),
                },
                CapturedSignalInput {
                    slot: mask_slot,
                    value: ResidentValueRef::Bool(&[1, 0]),
                },
            ])
            .unwrap();
        assert_eq!(
            selected_values(&instance.copied_output(0).unwrap()),
            [1.0, 2.0]
        );
    }
}

#[test]
fn nested_constructors_preserve_selected_child_dimensions_and_values() {
    use mech_core::ValueDataDraft as D;
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let cases = [
        "x := (signal<[f64]:2,2>, {row: signal[[false true],:]})",
        "x := (signal<[f64]:2,2>, {1: signal[[false true],:]})",
        "x := (signal<[f64]:2,2>, :some(signal[[false true],:]))",
        "x := (signal<[f64]:2,2>, ╭─────────╮\n│ value │\n├─────────┤\n│ (signal[[false true],:]) │\n╰─────────╯)",
    ];
    for (case, source) in cases.into_iter().enumerate() {
        let artifact = roundtrip(source);
        let mut instance = activate(
            ReactiveInstanceId::new(0x558, case as u32),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        for offset in [0.0, 10.0] {
            let data = [1.0 + offset, 3.0 + offset, 2.0 + offset, 4.0 + offset];
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&data),
                }])
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let output = instance.copied_output(0).unwrap();
            let D::Tuple(children) = output.canonical_data_draft().unwrap() else {
                panic!()
            };
            let matrix = match &children[1] {
                D::Record(fields) => &fields[0].value,
                D::Map(entries) => &entries[0].items[1],
                D::Tuple(items) => &items[1],
                D::Table(columns) => &columns[0].values[0],
                other => panic!("{other:?}"),
            };
            let D::Matrix(values) = matrix else {
                panic!("{matrix:?}")
            };
            assert_eq!(
                values
                    .iter()
                    .map(|value| match value {
                        D::F64(value) => value.to_f64(),
                        _ => panic!(),
                    })
                    .collect::<Vec<_>>(),
                [3.0 + offset, 4.0 + offset]
            );
            let SchemaBody::Tuple(schemas) = artifact
                .schemas()
                .get(output.schema())
                .unwrap()
                .closed_body(output.shape())
                .unwrap()
            else {
                panic!()
            };
            let selected = match &schemas[1] {
                SchemaBody::Record(fields) => &fields[0].schema,
                SchemaBody::Map { value, .. } => value.as_ref(),
                SchemaBody::Tuple(items) => &items[1],
                SchemaBody::Table { columns, .. } => &columns[0].schema,
                _ => panic!(),
            };
            assert!(
                matches!(selected,SchemaBody::Matrix {dimensions,..} if dimensions.as_ref()==[mech_core::DimensionExpr::Constant(1),mech_core::DimensionExpr::Constant(2)])
            );
        }
    }
}

#[test]
fn hexadecimal_and_large_rational_values_survive_artifact_roundtrip_and_execution() {
    use mech_core::ValueDataDraft as D;
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, expected) in [
        ("x := 0xf64", D::I64(0xf64)),
        ("x := 0xf32", D::I64(0xf32)),
        ("x := 0xc32", D::I64(0xc32)),
        ("x := -0xf64", D::I64(-0xf64)),
        (
            "x := -170141183460469231731687303715884105728/170141183460469231731687303715884105728",
            D::Rational64 {
                numerator: -1,
                denominator: 1,
            },
        ),
        (
            "x := 340282366920938463463374607431768211455/340282366920938463463374607431768211455",
            D::Rational64 {
                numerator: 1,
                denominator: 1,
            },
        ),
    ] {
        let artifact = roundtrip(source);
        let mut instance = activate(
            ReactiveInstanceId::new(0x55b, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert!(instance.plan.inputs.is_empty());
        for _ in 0..2 {
            instance
                .turn(&[])
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected,
                "{source}"
            );
        }
    }
}

#[test]
fn contextual_composite_empty_values_survive_codec_and_two_resident_turns() {
    use mech_core::snapshot::{MapEntryDraft, NamedValueDraft, OptionDraft, TableColumnDraft};
    use mech_core::{
        CardinalitySpec, DimensionExpr, IntegerWidth, SchemaField, ValueDataDraft as D,
    };
    fn none() -> D {
        D::Option(OptionDraft {
            present: false,
            value: None,
        })
    }
    fn some(value: D) -> D {
        D::Option(OptionDraft {
            present: true,
            value: Some(Box::new(value)),
        })
    }
    fn number(value: f64) -> D {
        D::F64(mech_core::snapshot::F64Bits::from_f64(value))
    }
    fn record(value: D) -> D {
        D::Record(
            vec![NamedValueDraft {
                name: "missing".to_owned(),
                value,
            }]
            .into_boxed_slice(),
        )
    }
    let u8_schema = SchemaBody::UnsignedInteger(IntegerWidth::W8);
    let optional_u8 = SchemaBody::Option(Box::new(u8_schema.clone()));
    let f64_schema = SchemaBody::FloatingPoint(mech_core::FloatWidth::W64);
    let missing_record = SchemaBody::Record(
        vec![SchemaField {
            name: "missing".to_owned(),
            schema: optional_u8.clone(),
        }]
        .into_boxed_slice(),
    );
    let matrix = |element, rows, columns| SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    };
    let path = mech_core::CanonicalNominalPath::new(vec!["some".to_owned()]).unwrap();
    let tag = SchemaBody::Atom(mech_core::NominalKey::from_path(
        mech_core::NominalKind::Atom,
        &path,
    ));
    let cases: Vec<(&str, SchemaBody, fn(f64) -> D)> = vec![
        (
            "x<(f64,{missing<u8?>})> := (signal<f64>,{missing:_})",
            SchemaBody::Tuple(vec![f64_schema.clone(), missing_record.clone()].into_boxed_slice()),
            |value| D::Tuple(vec![number(value), record(none())].into_boxed_slice()),
        ),
        ("x<u8?> := (_)", optional_u8.clone(), |_| none()),
        (
            "x<[u8?]:1,2> := [_ _]",
            matrix(optional_u8.clone(), 1, 2),
            |_| D::Matrix(vec![none(), none()].into_boxed_slice()),
        ),
        (
            "x<[u8?]:1,4> := [[_ _] [_ _]]",
            matrix(optional_u8.clone(), 1, 4),
            |_| {
                D::Matrix(
                    (0..4)
                        .map(|_| none())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
            },
        ),
        (
            "x<[u8?]:2,2> := [[_ _]; [_ _]]",
            matrix(optional_u8.clone(), 2, 2),
            |_| {
                D::Matrix(
                    (0..4)
                        .map(|_| none())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
            },
        ),
        (
            "x<[u8?]:2,4> := [[[_ _] [_ _]]; [[_ _] [_ _]]]",
            matrix(optional_u8.clone(), 2, 4),
            |_| {
                D::Matrix(
                    (0..8)
                        .map(|_| none())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
            },
        ),
        (
            "x<[u8?]:1,4> := [([_ _]) ([_ _])]",
            matrix(optional_u8.clone(), 1, 4),
            |_| {
                D::Matrix(
                    (0..4)
                        .map(|_| none())
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                )
            },
        ),
        (
            "x<[f64?]:2,2> := [_ signal<f64>; signal _]",
            matrix(SchemaBody::Option(Box::new(f64_schema.clone())), 2, 2),
            |value| {
                D::Matrix(
                    vec![none(), some(number(value)), some(number(value)), none()]
                        .into_boxed_slice(),
                )
            },
        ),
        (
            "x<[u8?]:2,2> := [[_ 1u8]; [2u8 _]]",
            matrix(optional_u8.clone(), 2, 2),
            |_| D::Matrix(vec![none(), some(D::U8(1)), some(D::U8(2)), none()].into_boxed_slice()),
        ),
        (
            "x<{missing<u8?>}> := {missing:_}",
            missing_record.clone(),
            |_| record(none()),
        ),
        (
            "x := {missing<[u8?]:1,2>: [_ _]}",
            SchemaBody::Record(
                vec![SchemaField {
                    name: "missing".to_owned(),
                    schema: matrix(optional_u8.clone(), 1, 2),
                }]
                .into_boxed_slice(),
            ),
            |_| record(D::Matrix(vec![none(), none()].into_boxed_slice())),
        ),
        (
            "x<{u8:{missing<u8?>}}> := {2u8:{missing:_},1u8:{missing:3u8}}",
            SchemaBody::Map {
                key: Box::new(u8_schema),
                value: Box::new(missing_record.clone()),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
            |_| {
                D::Map(
                    vec![
                        MapEntryDraft {
                            items: vec![D::U8(1), record(some(D::U8(3)))].into_boxed_slice(),
                        },
                        MapEntryDraft {
                            items: vec![D::U8(2), record(none())].into_boxed_slice(),
                        },
                    ]
                    .into_boxed_slice(),
                )
            },
        ),
        (
            "x<[{missing<u8?>}?]:1,2> := [{missing:_} _]",
            matrix(SchemaBody::Option(Box::new(missing_record.clone())), 1, 2),
            |_| D::Matrix(vec![some(record(none())), none()].into_boxed_slice()),
        ),
        (
            "x<|value<{missing<u8?>}>|:1> := ╭─────────╮\n│ value │\n├─────────┤\n│ {missing:_} │\n╰─────────╯",
            SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "value".to_owned(),
                    schema: missing_record.clone(),
                }]
                .into_boxed_slice(),
                rows: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
            },
            |_| {
                D::Table(
                    vec![TableColumnDraft {
                        name: "value".to_owned(),
                        values: vec![record(none())].into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                )
            },
        ),
        (
            "x := (|value<{missing<u8?>}>|{missing:_}|)",
            SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "value".to_owned(),
                    schema: missing_record.clone(),
                }]
                .into_boxed_slice(),
                rows: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
            },
            |_| {
                D::Table(
                    vec![TableColumnDraft {
                        name: "value".to_owned(),
                        values: vec![record(none())].into_boxed_slice(),
                    }]
                    .into_boxed_slice(),
                )
            },
        ),
        (
            "x<(:some,{missing<u8?>})> := :some({missing:_})",
            SchemaBody::Tuple(vec![tag, missing_record.clone()].into_boxed_slice()),
            |_| D::Tuple(vec![D::Atom, record(none())].into_boxed_slice()),
        ),
        (
            "x<{missing<u8?>}?> := {missing:_}",
            SchemaBody::Option(Box::new(missing_record)),
            |_| some(record(none())),
        ),
    ];
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, expected_schema, expected_value) in cases {
        let direct = definition(source).compile_artifact().unwrap();
        let decoded = roundtrip(source);
        for artifact in [&direct, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x55a, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            for value in [3.0, 7.0] {
                let data = [value];
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
                    .unwrap_or_else(|error| panic!("{source}: {error:?}"));
                let output = instance.copied_output(0).unwrap();
                assert_eq!(
                    artifact
                        .schemas()
                        .get(output.schema())
                        .unwrap()
                        .closed_body(output.shape())
                        .unwrap(),
                    expected_schema,
                    "{source}"
                );
                assert_eq!(
                    output.canonical_data_draft().unwrap(),
                    expected_value(value),
                    "{source}"
                );
            }
        }
    }
}

#[cfg(feature = "table")]
#[test]
fn composite_current_join_row_shapes_reach_nested_tuple_outputs() {
    use mech_core::snapshot::{SequenceView, SnapshotValidationContext};
    use mech_core::{ValueDataDraft as D, ValueDraft};
    let source = "x := ((|a<u8>|signal<u8>|) ⋈ (|a<u8>|1u8|), true)";
    let direct = definition(source).compile_artifact().unwrap();
    let decoded = roundtrip(source);
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&direct, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x570, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        for (key, expected) in [(1, vec![1]), (2, vec![]), (1, vec![1])] {
            let captured = [Some(
                ValueDraft {
                    schema: artifact.inputs()[0].schema,
                    shape_values: Box::new([]),
                    data: D::U8(key),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap(),
            )];
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::Snapshot(&captured),
                }])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Tuple(items) = output.data() else {
                panic!()
            };
            let [ValueData::Table(table), ValueData::Bool(true)] = items.as_ref() else {
                panic!()
            };
            assert!(
                matches!(table.column(0).unwrap(), SequenceView::U8(values) if values == expected)
            );
            let SchemaBody::Tuple(items) = artifact
                .schemas()
                .get(output.schema())
                .unwrap()
                .closed_body(output.shape())
                .unwrap()
            else {
                panic!()
            };
            assert!(
                matches!(&items[0], SchemaBody::Table { rows: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(rows)), .. } if *rows == expected.len() as u64)
            );
        }
    }
}

#[cfg(feature = "table")]
#[test]
fn composite_foreign_dynamic_payload_reaches_canonical_table_and_join() {
    use mech_core::snapshot::{F64Bits, SequenceView, SnapshotValidationContext};
    use mech_core::{SchemaDraft, SchemaTableBuilder, ValueDataDraft as D, ValueDraft};
    let mut builder = SchemaTableBuilder::new();
    let mut add = |body| {
        builder
            .insert(
                SchemaDraft {
                    body,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    };
    let dynamic = add(SchemaBody::Dynamic);
    let tuple = add(SchemaBody::Tuple(
        vec![
            SchemaBody::FloatingPoint(mech_core::FloatWidth::W64),
            SchemaBody::Bool,
        ]
        .into_boxed_slice(),
    ));
    let built = builder.finish().unwrap();
    let (dynamic, tuple) = (
        built.resolve(dynamic).unwrap(),
        built.resolve(tuple).unwrap(),
    );
    let (foreign, _) = built.into_parts();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in [
        "x := (|a<u8> value<*>|1u8 (payload<*>)|)",
        "x := (|a<u8> value<*> sample<(f64,bool)>|1u8 (payload<*>) (0,false)|)",
        "x := (|a<u8> value<*> sample<(f64,bool)>|1u8 (payload<*>) (0,false)|) ⋈ (|a<u8>|1u8|)",
    ] {
        let direct = definition(source).compile_artifact().unwrap();
        let decoded = roundtrip(source);
        for artifact in [&direct, &decoded] {
            if source.contains("sample") {
                assert_ne!(
                    dynamic,
                    artifact.inputs()[0].schema,
                    "{source}: fixture must use another schema ID for Dynamic"
                );
            } else {
                assert!(
                    artifact
                        .schemas()
                        .find_by_key(foreign.entry(tuple).unwrap().key())
                        .is_none(),
                    "payload schema must be foreign-only"
                );
            }
            let mut instance = activate(
                ReactiveInstanceId::new(0x571, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            assert_eq!(instance.plan.inputs.len(), 1);
            for number in [7.0, 9.0] {
                let value = ValueDraft {
                    schema: dynamic,
                    shape_values: Box::new([]),
                    data: D::Dynamic(Some(Box::new(ValueDraft {
                        schema: tuple,
                        shape_values: Box::new([]),
                        data: D::Tuple(
                            vec![D::F64(F64Bits::from_f64(number)), D::Bool(true)]
                                .into_boxed_slice(),
                        ),
                    }))),
                }
                .finalize(&SnapshotValidationContext::new(&foreign))
                .unwrap();
                instance
                    .turn(&[CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::Snapshot(&[Some(value)]),
                    }])
                    .unwrap_or_else(|error| panic!("{source}: {error:?}"));
                let output = instance.copied_output(0).unwrap();
                let ValueData::Table(table) = output.data() else {
                    panic!()
                };
                let SequenceView::Values([ValueData::Dynamic(payload)]) = table.column(1).unwrap()
                else {
                    panic!()
                };
                let payload = payload.value().unwrap();
                assert_eq!(payload.schema_key(), foreign.entry(tuple).unwrap().key());
                assert!(
                    matches!(payload.data(), ValueData::Tuple(items) if matches!(items.as_ref(), [ValueData::F64(value), ValueData::Bool(true)] if value.to_f64() == number))
                );
            }
        }
    }
}

#[cfg(feature = "table")]
#[test]
fn composite_nested_join_matrix_shapes_survive_current_child_rebinding() {
    use mech_core::snapshot::SequenceView;
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, selected_column) in [
        (
            "x := (signal<[f64]:2,2>, ╭─────────╮\n│ a │ v │\n├─────────┤\n│ 1u8 │ (signal[[false true],:]) │\n╰─────────╯ ⋈ (|a<u8>|1u8|))",
            1,
        ),
        (
            "x := (signal<[f64]:2,2>, ╭─────────╮\n│ value │\n├─────────┤\n│ (signal[[false true],:]) │\n╰─────────╯ ⋈ (|a<u8>|1u8|))",
            0,
        ),
    ] {
        let direct = definition(source).compile_artifact().unwrap();
        let decoded = roundtrip(source);
        for artifact in [&direct, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x572, 0),
                artifact,
                &catalog,
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
                assert_eq!(output.schema(), artifact.outputs()[0].schema);
                let ValueData::Tuple(items) = output.data() else {
                    panic!()
                };
                let [ValueData::Matrix(original), ValueData::Table(table)] = items.as_ref() else {
                    panic!()
                };
                assert!(
                    matches!(original.elements(), SequenceView::F64(values) if values.iter().map(|value| value.to_f64()).collect::<Vec<_>>() == [1.0+offset, 2.0+offset, 3.0+offset, 4.0+offset])
                );
                let SequenceView::Values([ValueData::Matrix(selected)]) =
                    table.column(selected_column).unwrap()
                else {
                    panic!()
                };
                assert!(
                    matches!(selected.elements(), SequenceView::F64(values) if values.iter().map(|value| value.to_f64()).collect::<Vec<_>>() == [3.0+offset, 4.0+offset])
                );
                let SchemaBody::Tuple(items) = artifact
                    .schemas()
                    .get(output.schema())
                    .unwrap()
                    .closed_body(output.shape())
                    .unwrap()
                else {
                    panic!()
                };
                let SchemaBody::Table { columns, rows } = &items[1] else {
                    panic!()
                };
                assert_eq!(
                    *rows,
                    mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(1))
                );
                assert!(
                    matches!(&columns[selected_column].schema, SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [mech_core::DimensionExpr::Constant(1), mech_core::DimensionExpr::Constant(2)])
                );
            }
        }
    }
}

#[test]
fn composite_memory_admission_failure_preserves_publication_and_recovers() {
    let source = "x := (signal<string>, true)";
    let direct = definition(source).compile_artifact().unwrap();
    let decoded = roundtrip(source);
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    // Each native String input, native draft, and output is individually
    // smaller than the byte limit; their admitted coexistence exceeds it.
    let large = "x".repeat((mech_core::RESIDENT_MAX_BYTES / 2) as usize);
    for artifact in [&direct, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x573, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["before".into()]),
            }])
            .unwrap();
        let prior = instance.copied_output(0).unwrap();
        let error = instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(std::slice::from_ref(&large)),
            }])
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
        let after = instance.copied_output(0).unwrap();
        assert_eq!(after.schema_key(), prior.schema_key());
        assert_eq!(after.shape(), prior.shape());
        assert!(
            after
                .language_eq(artifact.schemas(), &prior, artifact.schemas())
                .unwrap()
        );
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["after".into()]),
            }])
            .unwrap();
        let after = instance.copied_output(0).unwrap();
        assert!(
            matches!(after.data(), ValueData::Tuple(items) if matches!(items.as_ref(), [ValueData::String(text), ValueData::Bool(true)] if text.as_ref() == "after"))
        );
    }
}

#[test]
fn wide_composites_admit_actual_metadata_and_preserve_changed_values() {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for count in [32, 64] {
        for record in [false, true] {
            let values = (0..count)
                .map(|index| {
                    let value = if index == 0 { "signal<f64>" } else { "1" };
                    if record {
                        format!("field{index:02}: {value}")
                    } else {
                        value.into()
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let source = if record {
                format!("x := {{{values}}}")
            } else {
                format!("x := ({values})")
            };
            let direct = definition(&source).compile_artifact().unwrap();
            let decoded = roundtrip(&source);
            for artifact in [&direct, &decoded] {
                let mut instance = activate(
                    ReactiveInstanceId::new(0x574, 0),
                    artifact,
                    &catalog,
                    &ActivationFacts::default(),
                )
                .unwrap();
                for changed in [2., 3.] {
                    instance
                        .turn(&[CapturedSignalInput {
                            slot: instance.plan.inputs[0].slot,
                            value: ResidentValueRef::F64(&[changed]),
                        }])
                        .unwrap();
                    let output = instance.copied_output(0).unwrap();
                    assert_eq!(output.schema(), artifact.outputs()[0].schema);
                    let children = match output.data() {
                        ValueData::Tuple(items) => items.as_ref(),
                        ValueData::Record(record) => record.fields(),
                        other => panic!("{other:?}"),
                    };
                    assert_eq!(children.len(), count);
                    for (index, value) in children.iter().enumerate() {
                        assert!(
                            matches!(value, ValueData::F64(value) if value.to_f64() == if index == 0 { changed } else { 1. })
                        );
                    }
                }
            }
        }
    }
}

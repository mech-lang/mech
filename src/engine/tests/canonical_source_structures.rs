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

//! Source regressions from the v0.4 homepage, exercised through resident artifacts.
#![cfg(feature = "full_compiler")]

use mech_core::snapshot::{F64Bits, OptionDraft, SnapshotValidationContext, ValueDraft};
use mech_core::{ReactiveInstanceId, SchemaBody, Value, ValueData, ValueDataDraft};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, ReactiveInstance, activate};
use mech_engine::{
    CanonicalSourceFrontend, CanonicalSourceProgram, ProgramArtifact, SourceDocumentOutputKind,
    SourceSemanticError,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4f535454), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}\n{:?}",
        parsed.diagnostics
    );
    CanonicalSourceFrontend.compile_document_with_catalog(
        &DocumentSyntax::cast(parsed.syntax()).unwrap(),
        mech_stdlib::source_catalog(),
    )
}

fn artifacts(source: &str) -> ([ProgramArtifact; 2], usize) {
    let compiled = compile(source).unwrap_or_else(|error| panic!("{source}\n{error:?}"));
    let output = compiled
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn instance(artifact: &ProgramArtifact) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x4f535454, 0),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn check(source: &str, expected: impl Fn(&Value)) {
    let ([original, decoded], output) = artifacts(source);
    for artifact in [&original, &decoded] {
        let mut instance = instance(artifact);
        instance
            .turn(&[])
            .unwrap_or_else(|error| panic!("{source}\n{error:?}"));
        expected(&instance.copied_output(output).unwrap());
    }
}

fn input(artifact: &ProgramArtifact, data: ValueDataDraft) -> Value {
    assert_eq!(artifact.inputs().len(), 1);
    let schema_id = artifact.inputs()[0].schema;
    let schema = artifact.schemas().get(schema_id).unwrap();
    let shape = if let ValueDataDraft::Matrix(values) = &data {
        mech_core::shape_for_resolved_extents(schema, &[values.len() as u64, 1]).unwrap()
    } else {
        mech_core::shape_for_value_data(schema, &data, &[], None).unwrap()
    };
    ValueDraft {
        schema: schema_id,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data,
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap()
}

fn turn(instance: &mut ReactiveInstance, value: &Value) -> Result<(), String> {
    let slot = instance.plan.inputs[0].slot;
    instance
        .prepare_turn_values(&[CapturedValueInput { slot, value }])
        .and_then(|prepared| prepared.publish())
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn number(value: f64) -> ValueDataDraft {
    ValueDataDraft::F64(F64Bits::from_f64(value))
}

fn assert_set(value: &Value, expected: &[f64]) {
    let ValueData::Set(set) = value.data() else {
        panic!("{value:?}")
    };
    assert_eq!(set.elements().len(), expected.len());
    for (actual, expected) in set.elements().iter().zip(expected) {
        assert!(
            matches!(actual.data(), ValueData::F64(number) if number.to_f64() == *expected),
            "{actual:?}"
        );
    }
}

#[test]
fn optional_match_binds_present_payload_before_evaluating_guard() {
    for (reading, expected) in [("0.3", "recharge"), ("0.8", "patrol"), ("_", "no reading")] {
        check(
            &format!(
                r#"battery<f64?> := {reading}
status := battery?
  ├ charge, charge < 0.4 => "recharge"
  ├ charge => "patrol"
  └ * => "no reading".
status
"#
            ),
            |value| {
                assert!(
                    matches!(value.data(), ValueData::String(text) if text.as_ref() == expected),
                    "{value:?}"
                )
            },
        );
    }
}

#[test]
fn set_literal_removes_duplicate_elements() {
    check("known := {1, 2, 2, 3}\nknown\n", |value| {
        assert_set(value, &[1.0, 2.0, 3.0]);
        let schemas = value.schemas().unwrap();
        assert!(matches!(
            schemas.get(value.schema()).unwrap().body(),
            SchemaBody::Set {
                cardinality: mech_core::CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(
                    3
                )),
                ..
            }
        ));
    });
    check("{3, 1, 3, 1, 1}\n", |value| assert_set(value, &[1.0, 3.0]));
    check("{0, -0, 0}\n", |value| assert_set(value, &[0.0]));
    check("known<{f64}:3> := {1, 2, 2, 3}\nknown\n", |value| {
        assert_set(value, &[1.0, 2.0, 3.0])
    });
    check("{\"robot\", \"camera\", \"robot\"}\n", |value| {
        let ValueData::Set(set) = value.data() else {
            panic!("{value:?}")
        };
        assert_eq!(set.elements().len(), 2);
        assert!(
            matches!(set.elements()[0].data(), ValueData::String(text) if text.as_ref() == "camera")
        );
        assert!(
            matches!(set.elements()[1].data(), ValueData::String(text) if text.as_ref() == "robot")
        );
    });
}

#[test]
fn table_boolean_mask_selects_rows_with_their_original_column_kinds() {
    check(
        "robots := |id<f64> battery<f64> ready<bool>| 1 0.9 true | 2 0.3 false | 3 0.7 true |\nselected := robots[robots.ready]\nselected\n",
        |value| {
            let schemas = value.schemas().unwrap();
            let SchemaBody::Table { columns, .. } = schemas.get(value.schema()).unwrap().body()
            else {
                panic!("{value:?}")
            };
            assert_eq!(
                columns
                    .iter()
                    .map(|column| column.name.as_str())
                    .collect::<Vec<_>>(),
                ["id", "battery", "ready"]
            );
            let ValueData::Table(table) = value.data() else {
                panic!("{value:?}")
            };
            assert_eq!(table.column(0).unwrap().len(), 2);
            let mech_core::snapshot::SequenceView::F64(ids) = table.column(0).unwrap() else {
                panic!("{value:?}")
            };
            assert_eq!(
                ids.iter().map(|number| number.to_f64()).collect::<Vec<_>>(),
                [1.0, 3.0]
            );
            assert!(matches!(&columns[2].schema, SchemaBody::Bool));
            let mech_core::snapshot::SequenceView::F64(battery) = table.column(1).unwrap() else {
                panic!("{value:?}")
            };
            assert_eq!(
                battery
                    .iter()
                    .map(|number| number.to_f64())
                    .collect::<Vec<_>>(),
                [0.9, 0.7]
            );
            assert!(matches!(
                table.column(2).unwrap(),
                mech_core::snapshot::SequenceView::Bool([true, true])
            ));
        },
    );
}

#[test]
fn live_optional_match_tracks_absence_and_payload_changes() {
    let (artifacts, output) = artifacts(
        r#"battery := input<f64?>
status := battery?
  ├ charge, charge < 0.4 => "recharge"
  ├ charge => "patrol"
  └ * => "no reading".
status
"#,
    );
    for artifact in artifacts {
        let mut instance = instance(&artifact);
        for (reading, expected) in [
            (None, "no reading"),
            (Some(0.8), "patrol"),
            (Some(0.3), "recharge"),
            (None, "no reading"),
            (Some(0.8), "patrol"),
        ] {
            let value = input(
                &artifact,
                ValueDataDraft::Option(OptionDraft {
                    present: reading.is_some(),
                    value: reading.map(|value| Box::new(number(value))),
                }),
            );
            turn(&mut instance, &value).unwrap();
            let result = instance.copied_output(output).unwrap();
            assert!(
                matches!(result.data(), ValueData::String(text) if text.as_ref() == expected),
                "{result:?}"
            );
        }
    }
}

#[test]
fn optional_payload_binding_is_refutable_and_mixed_result_kinds_are_rejected() {
    let source = "battery<f64?> := 40\nstatus := battery?\n  ├ charge, charge < 0.4 => \"recharge\"\n  ├ charge => 1234\n  └ * => \"no reading\".\n";
    for reading in ["40", "0.2", "_"] {
        assert_eq!(
            compile(&source.replacen("40", reading, 1))
                .err()
                .expect("mixed arms must fail")
                .code,
            "source-semantics/incompatible-match-result-kind"
        );
    }
    assert_eq!(
        compile("battery<f64?> := 0.8\nstatus := battery?\n  └ charge => charge.\n")
            .err()
            .expect("absence needs coverage")
            .code,
        "source-semantics/non-exhaustive-match"
    );
}

#[test]
fn live_set_literal_can_shrink_and_grow_as_elements_become_equal() {
    let (artifacts, output) = artifacts("reading := input<f64>\n{1, reading, 2, reading}\n");
    for artifact in artifacts {
        let mut instance = instance(&artifact);
        for (reading, expected) in [
            (3.0, &[1.0, 2.0, 3.0][..]),
            (1.0, &[1.0, 2.0][..]),
            (2.0, &[1.0, 2.0][..]),
            (4.0, &[1.0, 2.0, 4.0][..]),
        ] {
            turn(&mut instance, &input(&artifact, number(reading))).unwrap();
            assert_set(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn table_mask_can_be_empty_full_and_reused_after_a_rejected_turn() {
    let (artifacts, output) = artifacts(
        "robots := |id<f64> ready<bool>| 1 true | 2 false | 3 true |\nmask := input<[bool]>\nselected := robots[mask,:]\nselected.id\n",
    );
    for artifact in artifacts {
        let mut instance = instance(&artifact);
        for (mask, expected) in [
            (&[true, false, true][..], &[1.0, 3.0][..]),
            (&[false, false, false][..], &[][..]),
            (&[true, true, true][..], &[1.0, 2.0, 3.0][..]),
            (&[false, true, false][..], &[2.0][..]),
        ] {
            let value = input(
                &artifact,
                ValueDataDraft::Matrix(mask.iter().copied().map(ValueDataDraft::Bool).collect()),
            );
            turn(&mut instance, &value).unwrap();
            let result = instance.copied_output(output).unwrap();
            let ValueData::Matrix(matrix) = result.data() else {
                panic!("{result:?}")
            };
            assert_eq!(matrix.elements().len(), expected.len());
            for (index, expected) in expected.iter().enumerate() {
                assert!(
                    matches!(matrix.elements().value_at(index), Some(ValueData::F64(number)) if number.to_f64() == *expected)
                );
            }
        }
        let accepted = instance.copied_output(output).unwrap();
        let malformed = input(
            &artifact,
            ValueDataDraft::Matrix(vec![ValueDataDraft::Bool(true); 2].into_boxed_slice()),
        );
        assert!(turn(&mut instance, &malformed).is_err());
        assert!(
            accepted
                .snapshot_eq(
                    artifact.schemas(),
                    &instance.copied_output(output).unwrap(),
                    artifact.schemas()
                )
                .unwrap()
        );
        let valid = input(
            &artifact,
            ValueDataDraft::Matrix(vec![ValueDataDraft::Bool(true); 3].into_boxed_slice()),
        );
        turn(&mut instance, &valid).unwrap();
        let result = instance.copied_output(output).unwrap();
        assert!(matches!(result.data(), ValueData::Matrix(matrix) if matrix.elements().len() == 3));
    }
}

#[test]
fn optional_matrix_payloads_retain_their_shape_and_explicit_optional_bindings_retain_the_wrapper() {
    for (reading, expected) in [("[11u64 22u64]", 22), ("_", 0)] {
        check(
            &format!(
                "<row> := <[u64]:1,2>\nreading<row?> := {reading}\nresult := reading?\n  ├ values => values[2u64]\n  └ * => 0u64.\nresult\n"
            ),
            |value| {
                assert!(
                    matches!(value.data(), ValueData::U64(number) if *number == expected),
                    "{value:?}"
                );
            },
        );
    }
    check(
        "reading<f64?> := _\nresult := reading?\n  └ value<f64?> => value.\nresult\n",
        |value| assert!(matches!(value.data(), ValueData::Option(None)), "{value:?}"),
    );
    check(
        "<row> := <[u64]:1,2>\nreading<row?> := [11u64 22u64]\nresult := reading?\n  └ value<[u64]?> => value.\nresult\n",
        |value| {
            assert!(
                matches!(value.data(), ValueData::Option(Some(payload)) if matches!(payload.as_ref(), ValueData::Matrix(matrix) if matrix.elements().len() == 2)),
                "{value:?}"
            );
        },
    );
}

#[test]
fn table_positional_row_selection_preserves_order_and_duplicate_rows() {
    check(
        "robots := |id<f64> ready<bool>| 1 true | 2 false | 3 true |\nselected := robots[[3 1 3],:]\nselected.id\n",
        |value| {
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("{value:?}")
            };
            let mech_core::snapshot::SequenceView::F64(ids) = matrix.elements() else {
                panic!("{value:?}")
            };
            assert_eq!(
                ids.iter().map(|number| number.to_f64()).collect::<Vec<_>>(),
                [3.0, 1.0, 3.0]
            );
        },
    );
}

#[test]
fn table_filter_preserves_current_matrix_column_shapes() {
    let (artifacts, output) = artifacts(
        "payload := input<[u64]>\nrobots := |ready<bool> samples<[u64]>|true payload|\nrobots[robots.ready]\n",
    );
    for artifact in artifacts {
        let mut instance = instance(&artifact);
        for count in [2, 0, 4] {
            let payload = ValueDataDraft::Matrix((0..count).map(ValueDataDraft::U64).collect());
            turn(&mut instance, &input(&artifact, payload)).unwrap();
            let value = instance.copied_output(output).unwrap();
            let ValueData::Table(table) = value.data() else {
                panic!("{value:?}")
            };
            let Some(ValueData::Matrix(matrix)) = table.column(1).unwrap().value_at(0) else {
                panic!("{value:?}")
            };
            assert_eq!(matrix.elements().len(), count as usize);
        }
    }
}

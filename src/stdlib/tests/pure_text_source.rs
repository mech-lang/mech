#![cfg(feature = "full_compiler")]

use mech_core::{
    ExecutionTarget, OperationId, ReactiveInstanceId, ResidentValueRef, RuntimeBindingSelector,
    RuntimeOperationBinding, SchemaBody, ValueData,
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> (ProgramArtifact, usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x57a), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    // Exercise the actual exported library surface, not catalog-free semantic
    // declarations that could hide a missing specializer or provider.
    let compiled = CanonicalSourceFrontend
        .compile_document_with_catalog(&document, mech_stdlib::source_catalog())
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = compiled
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let artifact = compiled.compile_artifact().unwrap();
    assert!(artifact.requirements().is_empty());
    (artifact, output)
}

fn check(source: &str, expected: impl Fn(&ValueData)) {
    check_value(source, |value| expected(value.data()));
}

fn check_value(source: &str, expected: impl Fn(&mech_core::Value)) {
    let (original, output) = compile(source);
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x57a, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance
                .turn(&[])
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            expected(&instance.copied_output(output).unwrap());
        }
    }
}

#[test]
fn ordinary_pure_text_operations_have_real_exports_contracts_and_native_linkage() {
    let catalog = mech_stdlib::source_catalog();
    for (module, item, arity, providers) in [
        ("number", "parse-u64", 1, 1),
        ("number", "format", 2, 2),
        ("sequence", "length", 1, 1),
    ] {
        let name = format!("{module}/{item}");
        let id = OperationId::from_name(&name);
        assert_eq!(catalog.module_export(module, item).unwrap().operation, id);
        assert!(catalog.specializer(id).is_some());
        let entries = catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(id),
                ExecutionTarget::DirectRuntime,
            )
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), providers);
        for entry in entries {
            assert!(
                matches!(entry.operation_binding(), RuntimeOperationBinding::Fixed(ids) if ids.contains(&id))
            );
            assert!(entry.native_linkage.is_some(), "{name}: {}", entry.name);
            let contract = entry.operation_contract(id).unwrap();
            assert!([false, true].into_iter().any(|matrix| contract
                == &mech_core::maintained_operation_contract(&name, arity, matrix).unwrap()));
        }
        assert!(
            catalog
                .resident_factory(&[module.to_owned()], item)
                .is_some()
        );
    }
}

#[test]
fn ordinary_decimal_parser_is_strict_checked_and_atomic_across_live_turns() {
    let (original, output) = compile("+> number\nnumber/parse-u64(text<string>)\n");
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x57b, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        for (text, expected) in [("0", 0), ("0001", 1), ("18446744073709551615", u64::MAX)] {
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&[text.to_owned()]),
                }])
                .unwrap();
            assert!(
                matches!(instance.copied_output(output).unwrap().data(), ValueData::U64(value) if *value == expected)
            );
        }
        let prior = instance.copied_output(output).unwrap();
        for text in [
            "",
            "-1",
            "+1",
            " 1",
            "1 ",
            "1.0",
            "1e3",
            "１",
            "18446744073709551616",
        ] {
            assert!(
                instance
                    .turn(&[CapturedSignalInput {
                        slot,
                        value: ResidentValueRef::String(&[text.to_owned()])
                    }])
                    .is_err(),
                "{text:?}"
            );
            assert!(
                instance
                    .copied_output(output)
                    .unwrap()
                    .language_eq(artifact.schemas(), &prior, artifact.schemas())
                    .unwrap()
            );
        }
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["7".to_owned()]),
            }])
            .unwrap();
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::U64(7)
        ));
    }
}

#[test]
fn ordinary_number_format_preserves_exact_integer_values_and_fixed_float_rounding() {
    check("+> number\nnumber/format(1.25, 9)\n", |value| {
        assert!(matches!(value, ValueData::String(text) if text.as_ref() == "1.250000000"));
    });
    for (literal, expected) in [
        ("255<u8>", "255"),
        ("65535<u16>", "65535"),
        ("4294967295<u32>", "4294967295"),
        ("18446744073709551615<u64>", "18446744073709551615"),
        (
            "340282366920938463463374607431768211455<u128>",
            "340282366920938463463374607431768211455",
        ),
        ("-128<i8>", "-128"),
        ("-32768<i16>", "-32768"),
        ("-2147483648<i32>", "-2147483648"),
        ("-9223372036854775808<i64>", "-9223372036854775808"),
        (
            "-170141183460469231731687303715884105728<i128>",
            "-170141183460469231731687303715884105728",
        ),
    ] {
        check(
            &format!("+> number\nnumber/format({literal}, 0<u64>)\n"),
            |value| {
                assert!(matches!(value, ValueData::String(text) if text.as_ref() == expected));
            },
        );
    }
    for (literal, precision, expected) in [
        ("1.25<f32>", 9, "1.250000000"),
        ("1.25<f64>", 9, "1.250000000"),
        ("1.125", 2, "1.12"),
        ("2.375", 2, "2.38"),
        ("-1.125", 2, "-1.12"),
        ("-0.0", 9, "-0.000000000"),
    ] {
        check(
            &format!("+> number\nnumber/format({literal}, {precision}<u64>)\n"),
            |value| {
                assert!(
                    matches!(value, ValueData::String(text) if text.as_ref() == expected),
                    "{value:?}"
                );
            },
        );
    }
}

#[test]
fn ordinary_matrix_format_broadcasts_scalar_precision_and_preserves_geometry() {
    for source in [
        "+> number\nnumber/format([1.25 2.5 5.0; -3.75 0.0 6.25], 3<u64>)\n",
        "+> number\nnumber/format([1.25<f32> 2.5<f32> 5.0<f32>; -3.75<f32> 0.0<f32> 6.25<f32>], 3<u64>)\n",
    ] {
        check_value(source, |value| {
            assert!(matches!(
                value.schemas().unwrap().get(value.schema()).unwrap().closed_body(value.shape()).unwrap(),
                SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [mech_core::DimensionExpr::Constant(2), mech_core::DimensionExpr::Constant(3)]
            ));
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("{value:?}")
            };
            let mech_core::snapshot::SequenceView::String(values) = matrix.elements() else {
                panic!()
            };
            assert_eq!(
                values.iter().map(|text| text.as_ref()).collect::<Vec<_>>(),
                ["1.250", "2.500", "5.000", "-3.750", "0.000", "6.250"]
            );
        });
    }
    check(
        "+> number\nnumber/format([1<u64> 2<u64>; 3<u64> 4<u64>], 0<u64>)\n",
        |value| {
            let ValueData::Matrix(matrix) = value else {
                panic!("{value:?}")
            };
            let mech_core::snapshot::SequenceView::String(values) = matrix.elements() else {
                panic!()
            };
            assert_eq!(
                values.iter().map(|text| text.as_ref()).collect::<Vec<_>>(),
                ["1", "2", "3", "4"]
            );
        },
    );
}

#[test]
fn ordinary_sequence_length_counts_graphemes_or_matrix_elements() {
    for (value, expected) in [
        ("\"\"", 0),
        ("\"e\u{301}👩🏽‍💻🇺🇸\"", 3),
        ("[1 2; 3 4]", 4),
        ("[\"e\u{301}\" \"👩🏽‍💻\"]", 2),
        ("[(1<u8>,2<u8>) (3<u8>,4<u8>)]", 2),
    ] {
        check(
            &format!("+> sequence\nsequence/length({value})\n"),
            |value| {
                assert!(matches!(value, ValueData::U64(count) if *count == expected));
            },
        );
    }
}

#[test]
fn ordinary_empty_matrix_format_still_checks_integer_precision() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    for precision in [0, 1, 13] {
        let (artifact, output) = compile(&format!(
            "+> number\nnumber/format(values<[i64]:0,3>, {precision}<u64>)\n"
        ));
        let mut instance = activate(
            ReactiveInstanceId::new(0x57c, 0),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let values = [Some(
            ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(Box::new([])),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap(),
        )];
        let result = instance.turn(&[CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value: ResidentValueRef::Snapshot(&values),
        }]);
        if precision == 0 {
            result.unwrap();
            let output = instance.copied_output(output).unwrap();
            assert!(
                matches!(output.data(), ValueData::Matrix(matrix) if matrix.elements().is_empty())
            );
            assert!(
                matches!(output.schemas().unwrap().get(output.schema()).unwrap().closed_body(output.shape()).unwrap(),
                SchemaBody::Matrix { dimensions, .. } if dimensions.as_ref() == [mech_core::DimensionExpr::Constant(0), mech_core::DimensionExpr::Constant(3)])
            );
        } else {
            assert!(result.is_err());
        }
    }
}

#[test]
fn ordinary_live_format_precision_errors_preserve_the_previous_output() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    let (original, output) = compile("+> number\nnumber/format(value<f64>, digits<u64>)\n");
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x57d, 0),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for (precision, expected) in [(9, Some("1.250000000")), (13, None), (2, Some("1.25"))] {
            let digits = [Some(
                ValueDraft {
                    schema: artifact.inputs()[1].schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::U64(precision),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap(),
            )];
            let previous = expected
                .is_none()
                .then(|| instance.copied_output(output).unwrap());
            let result = instance.turn(&[
                CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[1.25]),
                },
                CapturedSignalInput {
                    slot: instance.plan.inputs[1].slot,
                    value: ResidentValueRef::Snapshot(&digits),
                },
            ]);
            if let Some(expected) = expected {
                result.unwrap();
                assert!(
                    matches!(instance.copied_output(output).unwrap().data(), ValueData::String(text) if text.as_ref() == expected)
                );
            } else {
                assert!(result.is_err());
                assert!(
                    instance
                        .copied_output(output)
                        .unwrap()
                        .language_eq(artifact.schemas(), &previous.unwrap(), artifact.schemas())
                        .unwrap()
                );
            }
        }
    }
}

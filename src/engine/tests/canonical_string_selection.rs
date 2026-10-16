#![cfg(all(feature = "semantic-compiler", feature = "resident-artifact"))]

use mech_core::{
    FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, SchemaBody, ValueData,
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxNode, TextSnapshot,
};

fn artifact(source: &str) -> ProgramArtifact {
    fn expression(node: SyntaxNode) -> Option<ExpressionSyntax> {
        ExpressionSyntax::cast(node.clone()).or_else(|| node.children().find_map(expression))
    }
    let parsed = mech_syntax::document::parse_canonical_document(
        TextSnapshot::new(DocumentId(0x575), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let expression = expression(parsed.syntax()).unwrap();
    assert_eq!(expression.syntax().range(), parsed.source.full_range());
    let compiled = CanonicalSourceFrontend
        .compile_expression(&expression)
        .unwrap();
    compiled.compile_artifact().unwrap()
}

fn selection(selector: &str) -> ProgramArtifact {
    let artifact = artifact(&format!("(text<string>, text[{selector}])"));
    assert!(matches!(
        artifact.schemas().get(artifact.outputs()[0].schema).unwrap().body(),
        SchemaBody::Tuple(items) if items.as_ref() == [SchemaBody::String, SchemaBody::String]
    ));
    artifact
}

fn check(selector: &str, turns: &[(&str, &str)]) {
    let original = selection(selector);
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x575, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        for &(input, expected) in turns {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::String(&[input.to_owned()]),
                }])
                .unwrap_or_else(|error| panic!("selector {selector}, input {input:?}: {error:?}"));
            let output = instance.copied_output(0).unwrap();
            let ValueData::Tuple(items) = output.data() else {
                panic!("{output:?}")
            };
            assert!(matches!(&items[0], ValueData::String(value) if value.as_ref() == input));
            assert!(
                matches!(&items[1], ValueData::String(value) if value.as_ref() == expected),
                "selector {selector}, input {input:?}, expected {expected:?}: {output:?}",
            );
        }
    }
}

#[test]
fn canonical_string_scalar_selection_counts_graphemes() {
    check("1u64", &[("e\u{301}👩🏽‍💻Z", "e\u{301}"), ("👩‍🚀ab", "👩‍🚀")]);
    check("2u64", &[("e\u{301}👩🏽‍💻Z", "👩🏽‍💻"), ("a🇺🇸b", "🇺🇸")]);
    // Positional conversion is shared with matrices, including established
    // positive floating-point truncation rather than a String-only rule.
    check("2.9", &[("e\u{301}👩🏽‍💻Z", "👩🏽‍💻")]);
}

#[test]
fn canonical_string_ranges_and_gathers_preserve_order_and_duplicates() {
    check("2u64..=3u64", &[("e\u{301}👩🏽‍💻Z", "👩🏽‍💻Z"), ("a🇺🇸b", "🇺🇸b")]);
    check("1u64..3u64", &[("e\u{301}👩🏽‍💻Z", "e\u{301}👩🏽‍💻")]);
    check("[3u64 1u64 3u64 2u64]", &[("e\u{301}👩🏽‍💻Z", "Ze\u{301}Z👩🏽‍💻")]);
    check("[2u64; 1u64]", &[("e\u{301}👩🏽‍💻Z", "👩🏽‍💻e\u{301}")]);
    check("[true false true]", &[("e\u{301}👩🏽‍💻Z", "e\u{301}Z")]);
    for selector in ["[3u64 1u64; 2u64 4u64]", "[3.0 1.0; 2.0 4.0]"] {
        check(selector, &[("e\u{301}👩🏽‍💻Z🇺🇸", "Ze\u{301}👩🏽‍💻🇺🇸")]);
    }
    check(
        "[true false; true false]",
        &[("e\u{301}👩🏽‍💻Z🇺🇸", "e\u{301}Z")],
    );
}

#[test]
fn canonical_string_empty_selection_is_an_empty_string() {
    check("[false false false]", &[("e\u{301}👩🏽‍💻Z", "")]);
    check(":", &[("e\u{301}👩🏽‍💻Z", "e\u{301}👩🏽‍💻Z"), ("", "")]);
}

#[test]
fn canonical_string_repeated_gather_admits_every_payload_copy() {
    let grapheme = format!("e{}", "\u{301}".repeat(1024));
    let input = format!("{grapheme}Z");
    let expected = grapheme.repeat(3);
    assert!(expected.len() > input.len());
    check("[1u64 1u64 1u64]", &[(&input, &expected)]);
}

#[test]
fn canonical_string_invalid_selectors_fail_instead_of_truncating() {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for selector in [
        "0u64",
        "4u64",
        "-1<i64>",
        "[1u64 0u64]",
        "[1u64 4u64]",
        "1u64..=4u64",
        "[true false]",
    ] {
        let original = selection(selector);
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        for artifact in [&original, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x576, 0),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            assert!(
                instance
                    .turn(&[CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::String(&["e\u{301}👩🏽‍💻Z".to_owned()]),
                    }])
                    .is_err(),
                "selector {selector} must fail"
            );
        }
    }
}

#[test]
fn canonical_string_failed_selection_preserves_the_previous_turn() {
    let artifact = selection("2u64");
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x577, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&["a👩‍🚀".to_owned()]),
        }])
        .unwrap();
    let prior = instance.copied_output(0).unwrap();
    assert!(
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["e\u{301}".to_owned()]),
            }])
            .is_err()
    );
    let after = instance.copied_output(0).unwrap();
    assert!(
        after
            .language_eq(artifact.schemas(), &prior, artifact.schemas())
            .unwrap()
    );
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&["x🇺🇸".to_owned()]),
        }])
        .unwrap();
    let output = instance.copied_output(0).unwrap();
    assert!(matches!(output.data(), ValueData::Tuple(items)
        if matches!(&items[1], ValueData::String(value) if value.as_ref() == "🇺🇸")));
}

#[test]
fn canonical_string_empty_integer_gather_preserves_string_schema() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    let original = selection("indices<[u64]:0,1>");
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x578, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 2);
        let indices = [Some(
            ValueDraft {
                schema: artifact.inputs()[1].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(Box::new([])),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap(),
        )];
        for text in ["e\u{301}👩🏽‍💻Z", ""] {
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::String(&[text.to_owned()]),
                    },
                    CapturedSignalInput {
                        slot: instance.plan.inputs[1].slot,
                        value: ResidentValueRef::Snapshot(&indices),
                    },
                ])
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            assert!(matches!(output.data(), ValueData::Tuple(items)
                if matches!(&items[1], ValueData::String(value) if value.is_empty())));
        }
    }
}

#[test]
fn canonical_string_matrix_selection_keeps_whole_cell_semantics() {
    let original = artifact("(cells<[string]:1,1>, cells[1u64])");
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&original, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x579, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let text = "e\u{301}👩🏽‍💻Z";
        instance
            .turn(&[CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::String(&[text.to_owned()]),
            }])
            .unwrap();
        let output = instance.copied_output(0).unwrap();
        assert!(matches!(output.data(), ValueData::Tuple(items)
            if matches!(&items[1], ValueData::String(value) if value.as_ref() == text)));
    }
}

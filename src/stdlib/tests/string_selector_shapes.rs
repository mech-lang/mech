//! Canonical String selectors use current immutable Snapshot matrix geometry.
#![cfg(feature = "full_compiler")]

use mech_core::{ReactiveInstanceId, ResidentValueRef, Value, ValueData};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x53534850), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|value| value.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = program.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn tuple_last_string(value: &Value) -> &str {
    let ValueData::Tuple(values) = value.data() else {
        panic!("{value:?}")
    };
    let ValueData::String(text) = values.last().unwrap() else {
        panic!("{value:?}")
    };
    text
}

#[test]
fn dynamic_snapshot_gathers_use_current_count_and_preserve_previous_result_on_error() {
    let source = concat!(
        "+> text\n+> sequence\n",
        "(text<string>, indices<string>, text[[sequence/length(line) | line <- text/lines(indices)]])\n",
    );
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x53534850, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 2);
        let text_slot = instance.plan.inputs[0].slot;
        let index_slot = instance.plan.inputs[1].slot;
        let text = ["e\u{301}👩🏽‍💻Z🇺🇸".to_owned()];
        for (indices, expected) in [
            ("", ""),
            ("aaa\na\naaa\naa", "Ze\u{301}Z👩🏽‍💻"),
            ("aa", "👩🏽‍💻"),
            ("aaaa\na", "🇺🇸e\u{301}"),
            ("", ""),
        ] {
            let selector_input = [indices.to_owned()];
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: text_slot,
                        value: ResidentValueRef::String(&text),
                    },
                    CapturedSignalInput {
                        slot: index_slot,
                        value: ResidentValueRef::String(&selector_input),
                    },
                ])
                .unwrap_or_else(|error| panic!("indices {indices:?}: {error:?}"));
            assert_eq!(
                tuple_last_string(&instance.copied_output(output).unwrap()),
                expected
            );
        }
        let prior = instance.copied_output(output).unwrap();
        let epoch = instance.published_epoch();
        let invalid = ["a\naaaaa".to_owned()];
        assert!(
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: text_slot,
                        value: ResidentValueRef::String(&text)
                    },
                    CapturedSignalInput {
                        slot: index_slot,
                        value: ResidentValueRef::String(&invalid)
                    },
                ])
                .is_err()
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert!(
            instance
                .copied_output(output)
                .unwrap()
                .language_eq(artifact.schemas(), &prior, artifact.schemas())
                .unwrap()
        );
        let recovery = ["aa\na".to_owned()];
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: text_slot,
                    value: ResidentValueRef::String(&text),
                },
                CapturedSignalInput {
                    slot: index_slot,
                    value: ResidentValueRef::String(&recovery),
                },
            ])
            .unwrap();
        assert_eq!(
            tuple_last_string(&instance.copied_output(output).unwrap()),
            "👩🏽‍💻e\u{301}"
        );
    }
}

#[test]
fn dynamic_inclusive_ranges_inside_comprehensions_use_each_actual_selector_extent() {
    let source = concat!(
        "+> sequence\n",
        "Windows(text<string>, width<u64>) => (words<[string]>) :=\n",
        "  n := sequence/length(text)\n",
        "  words := [text[(i + 1u64)..=(i + width)] | i <- 0u64..n, width > 0u64, width <= n - i].\n",
        "Windows(signal<string>, 2u64)\n",
    );
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x53534851, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (text, expected) in [
            ("GGTA", vec!["GG", "GT", "TA"]),
            ("a", vec![]),
            ("e\u{301}👩🏽‍💻Z", vec!["e\u{301}👩🏽‍💻", "👩🏽‍💻Z"]),
            ("ab", vec!["ab"]),
        ] {
            let input = [text.to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap_or_else(|error| panic!("text {text:?}: {error:?}"));
            let value = instance.copied_output(output).unwrap();
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("{value:?}")
            };
            let mech_core::snapshot::SequenceView::String(strings) = matrix.elements() else {
                panic!("{value:?}")
            };
            let actual = strings
                .iter()
                .map(|text| text.to_string())
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn dynamic_snapshot_bool_masks_keep_equal_length_and_scalar_paths_unchanged() {
    let source = concat!(
        "+> text\n+> sequence\n",
        "(text<string>, mask<string>, text[[sequence/length(line) == 1u64 | line <- text/lines(mask)]])\n",
    );
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x53534852, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 2);
        let text_slot = instance.plan.inputs[0].slot;
        let mask_slot = instance.plan.inputs[1].slot;
        for (text, mask, expected) in [
            ("", "", ""),
            ("e\u{301}👩🏽‍💻Z", "x\nxx\nx", "e\u{301}Z"),
            ("a🇺🇸", "xx\nx", "🇺🇸"),
            ("Z", "x", "Z"),
        ] {
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: text_slot,
                        value: ResidentValueRef::String(&[text.to_owned()]),
                    },
                    CapturedSignalInput {
                        slot: mask_slot,
                        value: ResidentValueRef::String(&[mask.to_owned()]),
                    },
                ])
                .unwrap_or_else(|error| panic!("{text:?}, mask {mask:?}: {error:?}"));
            assert_eq!(
                tuple_last_string(&instance.copied_output(output).unwrap()),
                expected
            );
        }
        let prior = instance.copied_output(output).unwrap();
        assert!(
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: text_slot,
                        value: ResidentValueRef::String(&["abc".to_owned()])
                    },
                    CapturedSignalInput {
                        slot: mask_slot,
                        value: ResidentValueRef::String(&["x\nx".to_owned()])
                    },
                ])
                .is_err()
        );
        assert!(
            instance
                .copied_output(output)
                .unwrap()
                .language_eq(artifact.schemas(), &prior, artifact.schemas())
                .unwrap()
        );
    }
}

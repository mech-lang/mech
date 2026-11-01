//! Ordinary String generators preserve grapheme and lexical ownership semantics.
#![cfg(feature = "full_compiler")]

use mech_core::snapshot::{SequenceView, ValueDataDraft};
use mech_core::{
    DimensionExpr, ReactiveInstanceId, ResidentValueRef, SchemaBody, Value, ValueData,
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x47524150), Revision(1), source).unwrap(),
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
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = program.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    ([original, decoded], output)
}

fn assert_strings(value: &Value, expected: &[&str]) {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("{value:?}")
    };
    let SequenceView::String(strings) = matrix.elements() else {
        panic!("{value:?}")
    };
    let actual = strings.iter().map(|item| item.as_ref()).collect::<Vec<_>>();
    assert_eq!(actual, expected);
    let owner = value.schemas().unwrap();
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = owner
        .get(value.schema())
        .unwrap()
        .closed_body(value.shape())
        .unwrap()
    else {
        panic!("{value:?}")
    };
    assert_eq!(element.as_ref(), &SchemaBody::String);
    assert_eq!(
        dimensions.as_ref(),
        [
            DimensionExpr::Constant(1),
            DimensionExpr::Constant(expected.len() as u64)
        ]
    );
}

fn closed(source: &str, expected: &[&str]) {
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x47524150, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        for _ in 0..2 {
            instance
                .turn(&[])
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            assert_strings(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

fn live(source: &str, cases: &[(&str, &[&str])]) {
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x47524151, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for &(text, expected) in cases {
            let text = [text.to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&text),
                }])
                .unwrap_or_else(|error| panic!("{source}, {text:?}: {error:?}"));
            assert_strings(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn ordinary_string_generator_iterates_graphemes_and_string_matrix_iterates_elements() {
    closed("[part | part <- \"é🧑‍🚀A\"]\n", &["é", "🧑‍🚀", "A"]);
    closed("[part | part <- \"\"]\n", &[]);
    closed(
        "[part | part <- [\"ab\" \"é\"; \"éZ\" \"🧑‍🚀A\"]]\n",
        &["ab", "é", "éZ", "🧑‍🚀A"],
    );
}

#[test]
fn ordinary_live_string_generator_preserves_combining_zwj_crlf_and_raw_utf8_bytes() {
    let text = "Ae\u{301}👩🏽‍💻\r\n🇺🇸\t\0B";
    let expected = ["A", "e\u{301}", "👩🏽‍💻", "\r\n", "🇺🇸", "\t", "\0", "B"];
    live(
        "[part | part <- text<string>]\n",
        &[(text, &expected), ("", &[])],
    );
    // The independent literal oracle preserves every original byte, including
    // CRLF as one grapheme and decomposed text without normalization.
    assert_eq!(expected.concat().as_bytes(), text.as_bytes());
}

#[test]
fn ordinary_string_generator_refreshes_cardinality_when_utf8_byte_length_is_unchanged() {
    assert_eq!("éx".len(), "e\u{301}".len());
    assert_eq!("e\u{301}".len(), "abc".len());
    live(
        "[part | part <- text<string>]\n",
        &[
            ("éx", &["é", "x"]),
            ("e\u{301}", &["e\u{301}"]),
            ("abc", &["a", "b", "c"]),
            ("", &[]),
        ],
    );
}

#[test]
fn ordinary_nested_string_generators_resnapshot_each_lexical_source_scope() {
    live(
        "[left + right | left <- text<string>, right <- left + \"é\"]\n",
        &[
            ("ab", &["aa", "aé", "bb", "bé"]),
            ("Z", &["ZZ", "Zé"]),
            ("", &[]),
        ],
    );
}

#[test]
fn ordinary_string_set_generator_deduplicates_exact_grapheme_payloads() {
    let (artifacts, output) = compile("{part | part <- \"ééAéA\"}\n");
    let expected = ValueDataDraft::Set(
        ["A", "e\u{301}", "é"]
            .into_iter()
            .map(|item| ValueDataDraft::String(item.to_owned()))
            .collect(),
    );
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x47524152, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected
        );
    }
}

#[test]
fn ordinary_string_generator_false_filter_skips_body_and_preserves_empty_string_matrix_kind() {
    live(
        "[part[99u64] | part <- text<string>, false]\n",
        &[("e\u{301}🧑‍🚀", &[]), ("", &[]), ("a", &[])],
    );
}

#[test]
fn ordinary_nested_string_generator_denial_preserves_output_and_allows_recovery() {
    let source = "[left + right | left <- text<string>, right <- text]\n";
    let (artifacts, output) = compile(source);
    // The input is small and valid. Its Cartesian expansion exceeds the
    // maintained output-element limit, so denial belongs to execution work.
    let denied = "a".repeat(
        usize::try_from(mech_core::RESIDENT_MAX_OUTPUT_ELEMENTS)
            .unwrap()
            .isqrt()
            + 1,
    );
    assert!(denied.len() * denied.len() > mech_core::RESIDENT_MAX_OUTPUT_ELEMENTS as usize);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x47524153, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["ab".to_owned()]),
            }])
            .unwrap();
        let before = instance.copied_output(output).unwrap();
        assert_strings(&before, &["aa", "ab", "ba", "bb"]);
        let epoch = instance.published_epoch();
        assert!(
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(core::slice::from_ref(&denied))
                }])
                .is_err()
        );
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before.canonical_data_draft().unwrap()
        );
        assert_eq!(instance.published_epoch(), epoch);
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["é".to_owned()]),
            }])
            .unwrap();
        assert_strings(&instance.copied_output(output).unwrap(), &["éé"]);
    }
}

#[test]
fn ordinary_string_generator_in_declared_fsm_uses_the_retained_capture_after_suspend() {
    // This uses the maintained lexical-capture suspension syntax and a scalar
    // Bool result, avoiding an invented dynamic matrix return annotation.
    let source = "#CapturedText(value<string>) => <bool>\n  | :Start\n  | :Later.\n#CapturedText(value) -> :Start\n  :Start ~> :Later\n  :Later => \"é\" ∈ {part | part <- value}.\n#CapturedText(signal<string>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x47524154, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["éA".to_owned()]),
            }])
            .unwrap();
        assert!(instance.output_borrow(output).is_none());
        assert!(instance.has_ready_continuation());
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["Z".to_owned()]),
            }])
            .unwrap();
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::Bool(true)
        ));
    }
}

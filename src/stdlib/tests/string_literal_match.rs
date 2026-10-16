//! Literal String matches use canonical schemas and lazy raw UTF-8 equality.
#![cfg(feature = "full_compiler")]
use mech_core::{ReactiveInstanceId, ResidentValueRef, Value, ValueData};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn document(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5354524d), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    DocumentSyntax::cast(parsed.syntax()).unwrap()
}
fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(&document(source), mech_stdlib::source_catalog())
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
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}
fn check(source: &str, cases: &[(&str, u64)]) {
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x5354524d, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for &(text, expected) in cases {
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&[text.to_owned()]),
                }])
                .unwrap_or_else(|error| panic!("{source}, {text:?}: {error:?}"));
            assert_u64(&instance.copied_output(output).unwrap(), expected);
        }
    }
}
fn assert_u64(value: &Value, expected: u64) {
    assert!(
        matches!(value.data(),ValueData::U64(actual) if *actual==expected),
        "{value:?}"
    );
}

#[test]
fn closed_string_literal_match_roundtrips_and_preserves_decomposed_bytes() {
    let (artifacts, output) = compile("(\"é\"? | \"é\" => 1u64 | \"é\" => 2u64 | * => 3u64)\n");
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x5354524e, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert_u64(&instance.copied_output(output).unwrap(), 2);
    }
}

#[test]
fn live_literal_string_matches_are_raw_unicode_not_normalized() {
    check(
        "(signal<string>? | \"\" => 0u64 | \"é\" => 1u64 | \"é\" => 2u64 | \"👩🏽‍💻\" => 3u64 | \"\\r\\n\" => 4u64 | * => 9u64)\n",
        &[
            ("", 0),
            ("é", 1),
            ("e\u{301}", 2),
            ("👩🏽‍💻", 3),
            ("\r\n", 4),
            ("\n", 9),
            ("\0", 9),
            // Same UTF-8 byte length, with different raw payloads.
            ("éx", 9),
            ("e\u{301}", 2),
            ("abc", 9),
            ("é", 1),
        ],
    );
}

#[test]
fn selected_string_function_matches_normalize_unused_matrix_axis_drafts() {
    let function = "+> text\nSelected(line<string>) => (result<u64>) := result := (line[2u64]? | \"é\" => 1u64 | \"é\" => 2u64 | * => 3u64).\n";
    for call in [
        "Selected(signal<string>)\n",
        "lines := text/lines(signal<string>)\nSelected(lines[1u64])\n",
    ] {
        check(
            &format!("{function}{call}"),
            &[
                ("xé\nignored\n", 1),
                ("xe\u{301}\r\n", 2),
                ("xZ\n", 3),
                ("xé", 1),
            ],
        );
    }
}

#[test]
fn nested_function_string_literal_matches_keep_their_own_scrutinees() {
    check(
        "Nested(line<string>) => (result<u64>) := result := (line[1u64]? | \">\" => (line[2u64]? | \" \" => 1u64 | \"\\t\" => 2u64 | * => 3u64) | * => 4u64).\nNested(signal<string>)\n",
        &[("> ", 1), (">\t", 2), (">X", 3), ("plain", 4)],
    );
}

#[test]
fn string_match_inside_a_declared_fsm_uses_the_frozen_argument() {
    let source = "#Captured(value<string>) => <bool>\n  | :Start\n  | :Later.\n#Captured(value) -> :Start\n  :Start ~> :Later\n  :Later => (value? | \"é\" => true | * => false).\n#Captured(signal<string>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x5354524f, ordinal as u32),
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
                value: ResidentValueRef::String(&["e\u{301}".to_owned()]),
            }])
            .unwrap();
        assert!(instance.output_borrow(output).is_none());
        assert!(instance.has_ready_continuation());
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&["é".to_owned()]),
            }])
            .unwrap();
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::Bool(true)
        ));
    }
}

#[test]
fn string_literal_patterns_reject_wrong_schema_and_nonexhaustive_matches() {
    for source in [
        "(signal<u64>? | \"1\" => true | * => false)\n",
        "(signal<[string]:1,1>? | \"x\" => true | * => false)\n",
        "(signal<string>? | \"x\" => true)\n",
    ] {
        assert!(
            CanonicalSourceFrontend
                .compile_document_with_catalog(&document(source), mech_stdlib::source_catalog())
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn literal_comparison_denial_is_lazy_atomic_and_recovers() {
    let large = "x".repeat(mech_core::RESIDENT_MAX_COMPARISON_WORK as usize + 1);
    let source = format!(
        "(signal<string>? | \"\" => 7u64 | \"{large}\" => 1u64 / divisor<u64> | * => 0u64)\n"
    );
    let (artifacts, output) = compile(&source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x53545250, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let text = instance
            .plan
            .inputs
            .iter()
            .find(|input| {
                artifact.schemas().get(input.schema).unwrap().body()
                    == &mech_core::SchemaBody::String
            })
            .unwrap()
            .slot;
        let divisor = instance
            .plan
            .inputs
            .iter()
            .find(|input| input.slot != text)
            .unwrap()
            .clone();
        let zero = value_draft_zero(artifact, divisor.schema);
        // A zero divisor in an unselected arm must never execute. The first
        // match also avoids charging the later over-budget String comparison.
        let turn = |instance: &mut mech_engine::resident::ReactiveInstance, value: &str| {
            let string = mech_core::ValueCell::from_exact(value.to_owned())
                .unwrap()
                .snapshot()
                .unwrap();
            let text_input = instance
                .plan
                .inputs
                .iter()
                .find(|input| input.slot == text)
                .unwrap();
            let string = string
                .rebind(text_input.schema, &text_input.shape, artifact.schemas())
                .unwrap();
            instance
                .prepare_turn_values(&[
                    mech_engine::resident::CapturedValueInput {
                        slot: text,
                        value: &string,
                    },
                    mech_engine::resident::CapturedValueInput {
                        slot: divisor.slot,
                        value: &zero,
                    },
                ])
                .and_then(|prepared| prepared.publish())
        };
        turn(&mut instance, "").unwrap();
        assert_u64(&instance.copied_output(output).unwrap(), 7);
        let epoch = instance.published_epoch();
        let error = turn(&mut instance, &large).unwrap_err();
        assert!(
            matches!(
                error,
                mech_engine::resident::ResidentExecutionError::Kernel {
                    error: mech_core::ResidentKernelError::InvalidShape,
                    ..
                }
            ),
            "comparison admission must fail before division: {error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_u64(&instance.copied_output(output).unwrap(), 7);
        turn(&mut instance, "other").unwrap();
        assert_u64(&instance.copied_output(output).unwrap(), 0);
    }
}

fn value_draft_zero(artifact: &ProgramArtifact, schema: mech_core::SchemaId) -> Value {
    mech_core::snapshot::ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: mech_core::ValueDataDraft::U64(0),
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        artifact.schemas(),
    ))
    .unwrap()
}

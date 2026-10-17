//! Nested collections retain the execution owner of each lexical capture.
#![cfg(feature = "full_compiler")]

use mech_core::{ReactiveInstanceId, ResidentValueRef, ValueData};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x43415043), Revision(1), source).unwrap(),
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
    assert_eq!(original.revision(), decoded.revision());
    ([original, decoded], output)
}

fn string_resume(source: &str, captured: &str, fresh: &str) {
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x43415043, ordinal as u32),
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
                value: ResidentValueRef::String(&[captured.to_owned()]),
            }])
            .unwrap();
        assert!(instance.output_borrow(output).is_none());
        assert!(instance.has_ready_continuation());
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&[fresh.to_owned()]),
            }])
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::Bool(true)
        ));
    }
}

fn string_fsm(expression: &str) -> String {
    format!(
        "#CapturedText(value<string>) => <bool>\n  | :Start\n  | :Later.\n#CapturedText(value) -> :Start\n  :Start ~> :Later\n  :Later => {expression}.\n#CapturedText(signal<string>)\n"
    )
}

#[test]
fn generator_capture_owner_survives_one_and_two_collection_boundaries() {
    // Preserve the first reported failing source without grouping or helpers.
    let source = "#CapturedText(value<string>) => <bool>\n  | :Start\n  | :Later.\n#CapturedText(value) -> :Start\n  :Start ~> :Later\n  :Later => \"é\" ∈ {part | part <- value}.\n#CapturedText(signal<string>)\n";
    string_resume(source, "éA", "Z");
    string_resume(
        &string_fsm("\"é\" ∈ {part | part <- [piece | piece <- value]}"),
        "éA",
        "Z",
    );
}

#[test]
fn collection_yield_preserves_its_frozen_capture_read() {
    string_resume(&string_fsm("\"é\" ∈ {value | part <- \"x\"}"), "é", "Z");
}

#[test]
fn collection_equal_pattern_peer_preserves_its_frozen_capture_read() {
    string_resume(
        &string_fsm("\"matched\" ∈ {\"matched\" | value <- \"é\"}"),
        "é",
        "Z",
    );
}

#[test]
fn collection_filter_preserves_its_frozen_bool_capture_read() {
    let source = "#CapturedFlag(flag<bool>) => <bool>\n  | :Start\n  | :Later.\n#CapturedFlag(flag) -> :Start\n  :Start ~> :Later\n  :Later => \"matched\" ∈ {\"matched\" | part <- \"x\", flag}.\n#CapturedFlag(signal<bool>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x43415044, ordinal as u32),
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
                value: ResidentValueRef::Bool(&[1]),
            }])
            .unwrap();
        assert!(instance.has_ready_continuation());
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::Bool(&[0]),
            }])
            .unwrap();
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::Bool(true)
        ));
    }
}

#[test]
fn operation_inputs_keep_frozen_parameter_and_live_input_distinct_at_same_signal() {
    string_resume(
        &string_fsm("\"AB\" ∈ {value + signal | part <- \"x\"}"),
        "A",
        "B",
    );
    // Keep the large String comparison inside the collection body. A Bool
    // result isolates capture admission from String-key normalization work.
    let captured = "A".repeat(2048);
    let expected = format!("{captured}Z");
    let expression = format!("true ∈ {{(value + signal) == \"{expected}\" | part <- \"x\"}}");
    string_resume(&string_fsm(&expression), &captured, "Z");
}

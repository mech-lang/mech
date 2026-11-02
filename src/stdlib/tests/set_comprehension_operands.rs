//! Real canonical artifacts compose Set comprehensions inside formulas.
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
        TextSnapshot::new(DocumentId(0x5345544f), Revision(1), source).unwrap(),
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
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    ([original, decoded], output)
}

#[test]
fn membership_rhs_uses_live_exact_string_set_original_and_decoded() {
    let source = "\"é\" ∈ {part | part <- signal<string>}\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x5345544f, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (text, expected) in [
            ("éA", true),
            ("é", false),
            ("XYZ", false),
            ("éé", true),
            ("", false),
        ] {
            let input = [text.to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap_or_else(|error| panic!("{source}, {text:?}: {error:?}"));
            assert!(
                matches!(instance.copied_output(output).unwrap().data(), ValueData::Bool(actual) if *actual == expected)
            );
        }
    }
}

#[test]
fn ungrouped_rhs_comprehension_composes_with_the_maintained_set_union() {
    let source = "left := {part | part <- signal<string>}\nleft ∪ {part | part <- \"AB\"}\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x53455450, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert_eq!(instance.plan.inputs.len(), 1);
        let slot = instance.plan.inputs[0].slot;
        for (text, expected) in [("éAé", vec!["A", "B", "é"]), ("", vec!["A", "B"])] {
            let input = [text.to_owned()];
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::String(&input),
                }])
                .unwrap_or_else(|error| panic!("{source}, {text:?}: {error:?}"));
            let value = instance.copied_output(output).unwrap();
            let ValueData::Set(set) = value.data() else {
                panic!("{value:?}")
            };
            let actual = set
                .elements()
                .iter()
                .map(|entry| match entry.data() {
                    ValueData::String(text) => &**text,
                    data => panic!("{data:?}"),
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn ungrouped_set_operand_in_suspended_fsm_reads_the_retained_capture() {
    // Keep the original failing source intact: no grouping or helper binding.
    let source = "#CapturedText(value<string>) => <bool>\n  | :Start\n  | :Later.\n#CapturedText(value) -> :Start\n  :Start ~> :Later\n  :Later => \"é\" ∈ {part | part <- value}.\n#CapturedText(signal<string>)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = activate(
            ReactiveInstanceId::new(0x53455451, ordinal as u32),
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

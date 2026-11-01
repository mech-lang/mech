//! Retained-size planning must stay distinct from real copy/comparison work.
#![cfg(feature = "full_compiler")]

use mech_core::{
    ReactiveInstanceId, ResidentValueRef, Value, ValueData, ValueDataDraft, ValueDraft,
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, ReactiveInstance, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5254574b), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap();
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

fn instance(artifact: &ProgramArtifact, ordinal: usize) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x5254574b, ordinal as u32),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn string_turn(instance: &mut ReactiveInstance, text: String) -> Result<(), String> {
    assert_eq!(instance.plan.inputs.len(), 1);
    let values = [text];
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&values),
        }])
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn u64_value(artifact: &ProgramArtifact, number: u64) -> Value {
    let input = artifact.inputs().iter().next().unwrap();
    ValueDraft {
        schema: input.schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::U64(number),
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        artifact.schemas(),
    ))
    .unwrap()
}

#[test]
fn string_matrix_access_does_not_compare_the_unselected_payload() {
    let (artifacts, output) =
        compile("+> text\nparts := text/lines(signal<string>)\nparts[1u64]\n");
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        for (case, first) in ["kept", "é", "new"].into_iter().enumerate() {
            let mut instance = instance(artifact, ordinal * 3 + case);
            // One unselected element exceeds the entire comparison allowance.
            // Each case is a first publication: a later text/lines update would
            // genuinely compare the retained whole matrix before selection.
            string_turn(&mut instance, format!("{first}\n{}", "x".repeat(131_072))).unwrap();
            let value = instance.copied_output(output).unwrap();
            assert!(matches!(value.data(), ValueData::String(actual) if actual.as_ref() == first));
        }
    }
}

#[test]
fn checked_scalar_arithmetic_keeps_error_atomicity_and_recovery() {
    let (artifacts, output) = compile("signal<u64> + 1u64\n");
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        let run = |instance: &mut ReactiveInstance, number| {
            let values = [Some(u64_value(artifact, number))];
            let slot = instance.plan.inputs[0].slot;
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::Snapshot(&values),
                }])
                .map(|_| ())
        };
        for number in [0, u64::MAX - 1] {
            run(&mut instance, number).unwrap();
            assert!(
                matches!(instance.copied_output(output).unwrap().data(), ValueData::U64(actual) if *actual == number + 1)
            );
        }
        let before = instance.copied_output(output).unwrap();
        let epoch = instance.published_epoch();
        assert!(run(&mut instance, u64::MAX).is_err());
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            before.canonical_data_draft().unwrap(),
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap()
        );
        run(&mut instance, 9).unwrap();
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::U64(10)
        ));
    }
}

#[test]
fn actual_string_comparison_work_still_denies_and_recovers() {
    let (artifacts, output) =
        compile("+> text\nparts := text/lines(signal<string>)\nparts[1u64] == parts[2u64]\n");
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        string_turn(&mut instance, "same\nsame".into()).unwrap();
        let before = instance.copied_output(output).unwrap();
        let epoch = instance.published_epoch();
        let long = "x".repeat(70_000);
        assert!(
            string_turn(&mut instance, format!("{long}\n{long}")).is_err(),
            "real payload comparison must retain its finite allowance"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            before.canonical_data_draft().unwrap(),
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap()
        );
        string_turn(&mut instance, "a\nb".into()).unwrap();
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::Bool(false)
        ));
    }
}

#[test]
fn direct_live_string_equality_has_finite_comparison_and_atomic_recovery() {
    let (artifacts, output) = compile("left<string> == right<string>\n");
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        assert_eq!(instance.plan.inputs.len(), 2);
        let run = |instance: &mut ReactiveInstance, left: &str, right: &str| {
            let left = [left.to_owned()];
            let right = [right.to_owned()];
            let slots = [instance.plan.inputs[0].slot, instance.plan.inputs[1].slot];
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: slots[0],
                        value: ResidentValueRef::String(&left),
                    },
                    CapturedSignalInput {
                        slot: slots[1],
                        value: ResidentValueRef::String(&right),
                    },
                ])
                .map(|_| ())
        };
        run(&mut instance, "same", "same").unwrap();
        let before = instance
            .copied_output(output)
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        let epoch = instance.published_epoch();
        let common = "x".repeat(70_000);
        assert!(run(&mut instance, &common, &common).is_err());
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            before
        );
        run(&mut instance, "a", "b").unwrap();
        assert!(matches!(
            instance.copied_output(output).unwrap().data(),
            ValueData::Bool(false)
        ));
    }
}

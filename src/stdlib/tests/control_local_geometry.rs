//! Parameterized control locals retain runtime geometry through nested calls.
#![cfg(feature = "full_compiler")]

use mech_core::{ReactiveInstanceId, ResidentValueRef, Value, ValueData};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, ReactiveInstance, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4d434150), Revision(1), source).unwrap(),
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
    assert_eq!(
        encoded,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn instance(artifact: &ProgramArtifact, ordinal: usize) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x4d434150, ordinal as u32),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn flag_turn(
    instance: &mut ReactiveInstance,
    artifact: &ProgramArtifact,
    text: &str,
    flag: bool,
) -> Result<(), mech_engine::resident::ResidentExecutionError> {
    assert_eq!(instance.plan.inputs.len(), 2);
    let text_slot = instance
        .plan
        .inputs
        .iter()
        .find(|input| {
            matches!(
                artifact.schemas().get(input.schema).unwrap().body(),
                mech_core::SchemaBody::String
            )
        })
        .unwrap()
        .slot;
    let flag_slot = instance
        .plan
        .inputs
        .iter()
        .find(|input| input.slot != text_slot)
        .unwrap()
        .slot;
    instance
        .turn(&[
            CapturedSignalInput {
                slot: text_slot,
                value: ResidentValueRef::String(&[text.to_owned()]),
            },
            CapturedSignalInput {
                slot: flag_slot,
                value: ResidentValueRef::Bool(&[u8::from(flag)]),
            },
        ])
        .map(|_| ())
}

fn assert_u64(value: &Value, expected: u64) {
    assert!(
        matches!(value.data(), ValueData::U64(actual) if *actual == expected),
        "{value:?}"
    );
}

#[test]
fn nested_matrix_locals_report_current_geometry_without_activation_fixed_claims() {
    let source = "+> text\n+> sequence\n(flag<bool>? | true => sequence/length(text/lines(signal<string>)) | false => 9u64)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        let locals = instance
            .plan
            .slots
            .iter()
            .filter(|slot| {
                slot.artifact_id.get() as usize >= artifact.slots().len()
                    && matches!(
                        artifact.schemas().get(slot.schema).unwrap().body(),
                        mech_core::SchemaBody::Matrix { .. }
                    )
                    && !artifact
                        .schemas()
                        .get(slot.schema)
                        .unwrap()
                        .dimension_parameters()
                        .is_empty()
            })
            .collect::<Vec<_>>();
        assert!(
            !locals.is_empty(),
            "the nested text/lines result must remain an actual control local"
        );
        for local in locals {
            assert_eq!(local.region.kind, mech_core::ResidentValueKind::Snapshot);
            assert!(
                !local.activation_fixed_shape,
                "runtime matrix geometry is carried by its snapshot, not the lower-bound shape seed"
            );
        }
        for (text, flag, expected) in [
            ("", false, 9),
            ("a\nb", true, 2),
            ("c", true, 1),
            ("a\nb\nc\nd", true, 4),
            ("", true, 0),
            ("", false, 9),
        ] {
            flag_turn(&mut instance, artifact, text, flag).unwrap();
            assert_u64(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

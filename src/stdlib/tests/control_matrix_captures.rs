//! Turn-shaped lexical matrix captures retain canonical geometry and ownership.
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

fn text_turn(
    instance: &mut ReactiveInstance,
    text: &str,
) -> Result<(), mech_engine::resident::ResidentExecutionError> {
    assert_eq!(instance.plan.inputs.len(), 1);
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&[text.to_owned()]),
        }])
        .map(|_| ())
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

const HELPER: &str =
    "Has(value<string>) => (result<bool>) := result := (value? | \"x\" => true | * => false).\n";
fn guard_program(helper: bool) -> String {
    let guard = if helper {
        "Has(items[1u64])"
    } else {
        "items[1u64] == \"x\""
    };
    format!(
        "+> text\n{HELPER}#Guard(items<[string]>) => <u64>\n  | :Start\n  | :Done(result<u64>).\n#Guard(items) -> :Start\n  :Start\n    | {guard} -> :Done(1u64)\n    | * -> :Done(0u64)\n  :Done(result) => result.\n#Guard(text/lines(signal<string>))\n"
    )
}

#[test]
fn runtime_matrix_fsm_guards_support_direct_and_nested_function_matches() {
    for helper in [false, true] {
        let (artifacts, output) = compile(&guard_program(helper));
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let mut instance = instance(artifact, ordinal);
            for (text, expected) in [
                ("x\ny", 1),
                ("z", 0),
                ("x\ny\nz\nw", 1),
                ("y\nx", 0),
                ("x", 1),
            ] {
                text_turn(&mut instance, text).unwrap();
                assert!(!instance.has_ready_continuation());
                assert_u64(&instance.copied_output(output).unwrap(), expected);
            }
        }
    }
}

#[test]
fn ordinary_nested_match_captures_refresh_current_matrix_geometry_lazily() {
    let source = "+> text\nlines := text/lines(signal<string>)\n(flag<bool>? | true => (lines[1u64]? | \"x\" => 1u64 | * => 0u64) | false => 9u64)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (text, flag, expected) in [
            ("", false, 9),
            ("x\ny\nz", true, 1),
            ("y", true, 0),
            ("x", true, 1),
            ("", false, 9),
        ] {
            flag_turn(&mut instance, artifact, text, flag).unwrap();
            assert_u64(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn turn_shaped_u64_matrix_capture_uses_its_current_snapshot() {
    let source = "+> text\n+> map\ncounts := map/values(map/histogram(text/lines(signal<string>)))\n(flag<bool>? | true => counts[1u64] | false => 0u64)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (text, flag, expected) in [
            ("", false, 0),
            ("x\nx\nx", true, 3),
            ("y\nx\ny", true, 1),
            ("x", true, 1),
            ("", false, 0),
        ] {
            flag_turn(&mut instance, artifact, text, flag).unwrap();
            assert_u64(&instance.copied_output(output).unwrap(), expected);
        }
    }
}

#[test]
fn suspended_matrix_capture_preserves_old_shape_beside_fresh_live_shape() {
    let source = "+> text\n+> sequence\n#Captured(items<[string]>) => <(string,u64,u64,string)>\n  | :Start\n  | :Later.\n#Captured(items) -> :Start\n  :Start ~> :Later\n  :Later => (items[2u64], sequence/length(items), sequence/length(text/lines(signal<string>)), text/lines(signal<string>)[1u64]).\n#Captured(text/lines(signal<string>))\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (old, fresh, kept, old_count, fresh_count, first) in [
            ("a\nkeep", "fresh", "keep", 2, 1, "fresh"),
            ("a\né\nz", "new\nb\nc\nd", "é", 3, 4, "new"),
        ] {
            text_turn(&mut instance, old).unwrap();
            assert!(instance.has_ready_continuation());
            text_turn(&mut instance, fresh).unwrap();
            assert!(!instance.has_ready_continuation());
            let value = instance.copied_output(output).unwrap();
            let ValueData::Tuple(parts) = value.data() else {
                panic!("{value:?}")
            };
            assert_eq!(parts.len(), 4);
            assert!(matches!(&parts[0], ValueData::String(text) if text.as_ref() == kept));
            assert!(matches!(&parts[1], ValueData::U64(n) if *n == old_count));
            assert!(matches!(&parts[2], ValueData::U64(n) if *n == fresh_count));
            assert!(matches!(&parts[3], ValueData::String(text) if text.as_ref() == first));
        }
    }
}

#[test]
fn matrix_capture_budget_failure_preserves_publication_and_recovers() {
    let source = "+> text\nlines := text/lines(signal<string>)\n(flag<bool>? | true => (lines[1u64]? | \"x\" => 1u64 | * => 0u64) | false => 9u64)\n";
    let (artifacts, output) = compile(source);
    // The retained canonical matrix exceeds the finite canonical traversal
    // allowance. No output from that failed turn may become a publication.
    let large = "x".repeat(mech_core::RESIDENT_MAX_COMPARISON_WORK as usize + 1);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        flag_turn(&mut instance, artifact, "x", true).unwrap();
        let epoch = instance.published_epoch();
        let previous = instance.copied_output(output).unwrap();
        let error = flag_turn(&mut instance, artifact, &large, true).unwrap_err();
        assert!(
            matches!(
                error,
                mech_engine::resident::ResidentExecutionError::Kernel {
                    error: mech_core::ResidentKernelError::InvalidShape,
                    ..
                }
            ),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_u64(&previous, 1);
        assert_u64(&instance.copied_output(output).unwrap(), 1);
        flag_turn(&mut instance, artifact, "z\ny", true).unwrap();
        assert_u64(&instance.copied_output(output).unwrap(), 0);
    }
}

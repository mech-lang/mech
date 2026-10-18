//! Immutable constant-backed output aliases are not retained state.
#![cfg(all(feature = "full_compiler", feature = "native-artifact"))]
use mech_core::ConstantId;
use mech_engine::native_artifact::{
    NativeArtifactLoweringError, NativeValueRef, lower_pure_native_artifact,
};
use mech_engine::{
    ArtifactSource, CanonicalSourceFrontend, InitializerReference, ProducerReference,
    ProgramArtifact, ProgramArtifactDraft, SlotRole,
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ProgramArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4f555449), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap()
}
fn roundtrip(artifact: ProgramArtifact) -> [ProgramArtifact; 2] {
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(artifact.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [artifact, decoded]
}
fn draft(artifact: &ProgramArtifact) -> ProgramArtifactDraft {
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().into(),
        slots: artifact.slots().into(),
        nodes: artifact.nodes().into(),
        bindings: artifact.bindings().into(),
        outputs: artifact.outputs().into(),
        constraints: artifact.constraints().into(),
        compute_regions: artifact.compute_regions().into(),
    }
}
fn reject(artifact: ProgramArtifact) {
    for artifact in roundtrip(artifact) {
        assert!(matches!(
            lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()),
            Err(NativeArtifactLoweringError::UnsupportedProgram {
                feature: "state or initializers"
            }),
        ));
    }
}
#[test]
fn actual_constant_outputs_and_lexical_aliases_preserve_every_checked_node() {
    for source in [
        "true\n",
        "18446744073709551615u64\n",
        "answer := 42u64\nanswer\n",
        "first := 42u64\nsecond := first\nsecond\n",
        "unused := left<u64> / 0u64\nright<u64> % 0u64\n42u64\n",
    ] {
        for artifact in roundtrip(compile(source)) {
            let plan =
                lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.lets().len(), artifact.nodes().len());
            assert_eq!(plan.outputs().len(), artifact.outputs().len());
            let mut initializers = 0;
            for slot in artifact.slots() {
                if let Some(InitializerReference::Constant(id)) = slot.initializer {
                    initializers += 1;
                    assert_eq!(slot.role, SlotRole::Output);
                    let ProducerReference::Output {
                        output,
                        source: ArtifactSource::Constant(source),
                    } = slot.producer
                    else {
                        panic!("canonical direct constant output must name its exact constant");
                    };
                    assert_eq!(id, source);
                    assert_eq!(
                        plan.outputs()[output.get() as usize].value(),
                        NativeValueRef::Constant(id)
                    );
                    assert_eq!(artifact.constants().get(id).unwrap().schema(), slot.schema);
                }
            }
            assert!(initializers > 0, "{source}");
        }
    }
}
#[test]
fn validated_different_constant_initializer_is_rejected_even_with_matching_scalar_schema() {
    let artifact = compile("left<u64> + 1u64\n42u64\n");
    let mut changed = draft(&artifact);
    let slot = changed
        .slots
        .iter_mut()
        .find(|slot| slot.initializer.is_some())
        .unwrap();
    let Some(InitializerReference::Constant(original)) = slot.initializer else {
        panic!("constant output");
    };
    let other = (0..artifact.constants().len())
        .map(|index| ConstantId::new(index as u32))
        .find(|id| {
            *id != original && artifact.constants().get(*id).unwrap().schema() == slot.schema
        })
        .unwrap();
    slot.initializer = Some(InitializerReference::Constant(other));
    // This lifecycle representation is accepted by the canonical validator:
    // initialization and the later source can differ while retaining a type.
    // The stateless native slice deliberately does not support that lifecycle.
    reject(
        changed
            .finalize()
            .expect("same-schema output initializer is a valid canonical artifact"),
    );
}
#[test]
fn validated_constant_initializer_of_a_live_output_is_not_treated_as_a_literal_alias() {
    let artifact = compile("left<u64> + 1u64\n");
    let mut changed = draft(&artifact);
    let slot = changed
        .slots
        .iter_mut()
        .find(|slot| slot.role == SlotRole::Output)
        .unwrap();
    assert!(matches!(
        slot.producer,
        ProducerReference::Output {
            source: ArtifactSource::Slot(_),
            ..
        }
    ));
    let id = (0..artifact.constants().len())
        .map(|index| ConstantId::new(index as u32))
        .find(|id| artifact.constants().get(*id).unwrap().schema() == slot.schema)
        .unwrap();
    slot.initializer = Some(InitializerReference::Constant(id));
    reject(
        changed
            .finalize()
            .expect("typed output initializer remains valid metadata"),
    );
}
#[test]
fn genuine_state_and_computed_activation_still_fail_closed() {
    reject(compile("~state := 0u64\nstate\n"));
    for artifact in roundtrip(compile("1u64 / 0u64\n")) {
        assert!(matches!(
            lower_pure_native_artifact(&artifact, &mech_stdlib::source_catalog()),
            Err(NativeArtifactLoweringError::UnsupportedNode {
                feature: "Activation computation",
                ..
            }),
        ));
    }
}

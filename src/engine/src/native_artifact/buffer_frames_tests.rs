use super::*;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::Arc;

fn plans(source: &str) -> [Plan; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x42554646), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_runtime(&mut catalog).unwrap();
    crate::install_intrinsic_compiler_runtime(&mut catalog).unwrap();
    crate::install_intrinsic_resident(&mut catalog).unwrap();
    crate::install_intrinsic_source(&mut catalog).unwrap();
    let catalog = Arc::new(catalog.build().unwrap());
    let original = crate::CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            catalog.clone(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
        .map(|artifact| super::super::lower_native_program_artifact(&artifact, &catalog).unwrap())
}

fn scalar_span(frames: &NativeBufferFrames, field: NativeBufferFrameField) -> bool {
    match field.lane() {
        NativeBufferFrameLane::Scalar { .. } => true,
        NativeBufferFrameLane::Composite { .. } => frames
            .children(field)
            .unwrap()
            .iter()
            .all(|field| scalar_span(frames, *field)),
        NativeBufferFrameLane::Managed { .. } => false,
    }
}

#[test]
fn closed_scalar_children_are_spans_of_private_mixed_frames() {
    for source in [
        "frame := (input<[u8]>, (seed<u8>, 7u64))\n(buffer, state) := frame\nbyte<u64> := buffer[1u64]\nscore<u64> := state.1\nbyte + score + state.2\n",
        "frame := {buffer: input<[u8]>, state: {score: seed<u8>, rank: 7u64}}\nbuffer := frame.buffer\nstate := frame.state\nbyte<u64> := buffer[1u64]\nscore<u64> := state.score\nbyte + score + state.rank\n",
        "frame := (input<[u8]>, (seed<u8>, (7u64, true)))\n(buffer, state) := frame\nbyte<u64> := buffer[1u64]\nscore<u64> := state.1\nbyte + score + state.2.1\n",
    ] {
        for plan in plans(source) {
            let frames = plan.facts().buffer_frames();
            assert!(
                frames.slots.iter().enumerate().any(|(value, _)| {
                    let Some(fields) = frames.fields_at(value) else {
                        return false;
                    };
                    fields
                        .iter()
                        .any(|f| matches!(f.lane(), NativeBufferFrameLane::Managed { .. }))
                        && fields.iter().any(|field| {
                            matches!(field.lane(), NativeBufferFrameLane::Composite { .. })
                                && scalar_span(&frames, *field)
                        })
                }),
                "missing embedded scalar span: {source}"
            );
        }
    }
}

#[test]
fn standalone_scalar_aggregate_keeps_its_scalar_owner() {
    for plan in plans("state := (seed<u8>, 7u64)\nscore<u64> := state.1\nscore + state.2\n") {
        assert_eq!(plan.facts().buffer_frames().elements(), 0);
        assert!(plan.scalar_aggregates().value_count() > 0);
    }
}

#[test]
fn an_escaping_mixed_frame_cannot_hide_scalar_child_publication() {
    for plan in plans("frame := (input<[u8]>, (seed<u8>, 7u64))\nframe\n") {
        assert_eq!(plan.facts().buffer_frames().elements(), 0);
    }
}

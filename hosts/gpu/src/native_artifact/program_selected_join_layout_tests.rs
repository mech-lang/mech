use super::*;
use crate::native_artifact::{
    NATIVE_PROGRAM_TYPED_IR_VERSION, NATIVE_PROGRAM_WORK_POLICY_VERSION, emit_program_object,
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x52414e47454a), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

#[test]
fn joined_map_proof_requires_a_complete_unobserved_copy_chain_to_the_exact_generator() {
    let body = "+> text\n+> map\n+> sequence\nentries := {\"a\": \"A\", \"b\": \"B\"}\nparts := text/lines(seed<string>)\nsize := sequence/length(parts)\nrecord := text/join(parts[1u64..=size], \"\")\nnormalized := text/join([entries{ch} | ch <- record], \"\")\n";
    for (tail, expected) in [("normalized\n", 1), ("(record, normalized)\n", 0)] {
        let mut original = None;
        for artifact in artifacts(&(body.to_owned() + tail)) {
            let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let descriptor = object.descriptor();
            let maps = &descriptor.selected_joins.maps;
            assert_eq!(maps.iter().flatten().count(), expected);
            if let Some(proof) = maps.iter().flatten().next() {
                let region = descriptor
                    .string_map_regions
                    .iter()
                    .find(|r| r.collection() == proof.collection)
                    .unwrap();
                assert_eq!(proof.iteration, region.iteration());
                assert_eq!(proof.source, region.source().get());
                assert_eq!(proof.map, region.map().get());
                assert!(proof.sites.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(proof.sites.iter().any(|&site| matches!(descriptor.plan.blocks()[site as usize].action(), mech_engine::native_artifact::NativeIterationAction::Initialize { iteration } if *iteration == proof.iteration)));
            }
            if let Some(previous) = original.as_ref() {
                assert_eq!(previous, maps);
            } else {
                original = Some(maps.clone());
            }
            let encoded = descriptor.encode_build_owned_image().unwrap();
            let restored = unsafe {
                crate::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &encoded,
                    &mech_stdlib::source_catalog(),
                )
            }
            .unwrap();
            assert_eq!(maps, &restored.selected_joins.maps);
            if expected != 0 {
                let mut changed: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
                changed["descriptor"]["selected_joins"]["maps"][0]["source"] =
                    serde_json::json!(u32::MAX);
                assert!(
                    unsafe {
                        crate::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                            &serde_json::to_vec(&changed).unwrap(),
                            &mech_stdlib::source_catalog(),
                        )
                    }
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn compact_range_layout_and_identity_preserve_original_decoded_library_visits() {
    for (source, compact) in [
        (
            "+> text\ntext/join((text/lines(input<string>))[first<u64>..=last<u64>], \"|\")\n",
            true,
        ),
        (
            "+> text\ntext/join((text/lines(input<string>))[indices<[u64]>], \"|\")\n",
            false,
        ),
    ] {
        let mut original_identity = None;
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let descriptor = object.descriptor();
            let layout = &descriptor.selected_joins;
            assert_eq!(layout.regions.len(), 1);
            let region = &layout.regions[0];
            assert!(matches!(
                layout.sites[region.selection() as usize],
                Some(Site::Select(0))
            ));
            assert!(matches!(
                layout.sites[region.consumer() as usize],
                Some(Site::Join(0))
            ));
            assert_eq!(region.range().is_some(), compact);
            if let Some(range) = region.range() {
                assert!(matches!(
                    layout.sites[range.producer() as usize],
                    Some(Site::Range(0))
                ));
                assert!(
                    matches!(descriptor.plan().blocks()[range.producer() as usize].action(),
                    mech_engine::native_artifact::NativeIterationAction::LibraryCall { operands, output, .. }
                        if operands.as_ref() == [range.start(), range.end()] && *output == region.selector())
                );
            } else {
                assert!(
                    !layout
                        .sites
                        .iter()
                        .any(|site| matches!(site, Some(Site::Range(_))))
                );
            }
            let current = *object.identity().bytes();
            assert_eq!(
                u64::from_le_bytes(current[16..24].try_into().unwrap()),
                NATIVE_PROGRAM_TYPED_IR_VERSION
            );
            assert_eq!(u64::from_le_bytes(current[24..32].try_into().unwrap()), 31);
            assert_eq!(
                u64::from_le_bytes(current[32..40].try_into().unwrap()),
                NATIVE_PROGRAM_WORK_POLICY_VERSION
            );
            if let Some(original) = original_identity {
                assert_eq!(
                    current, original,
                    "decoding must retain the same strategy identity"
                );
            } else {
                original_identity = Some(current);
            }
            assert!(
                crate::native_artifact::program_linked::prepare_descriptor(
                    &artifact, &catalog, &current
                )
                .is_ok()
            );
            for (offset, preceding) in [(16, 26u64), (32, 35u64), (16, 47), (24, 24)] {
                let mut old = current;
                old[offset..offset + 8].copy_from_slice(&preceding.to_le_bytes());
                assert!(
                    crate::native_artifact::program_linked::prepare_descriptor(
                        &artifact, &catalog, &old
                    )
                    .is_err(),
                    "pre-elision identity must not be silently relabeled"
                );
            }
        }
    }
}

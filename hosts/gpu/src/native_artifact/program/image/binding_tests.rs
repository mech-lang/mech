use super::*;
use crate::native_artifact::{NativeProgramDescriptor, emit_program_object};
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn artifacts(source: &str) -> [mech_engine::ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x494d4750494e), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}

#[test]
fn object_image_binding_rejects_changed_bytes_identity_and_every_binding_byte() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(include_str!(
        "../../../../tests/fixtures/native/numeric-parent-child.mec"
    )) {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let image = object.descriptor().encode_build_owned_image().unwrap();
        let identity = object.identity().bytes();
        let binding = object.image_binding().bytes();
        // SAFETY: all metadata and symbols here come from the genuine checked
        // object. No entry or native-code execution authority is manufactured.
        assert!(unsafe { verify(&image, identity, binding) }.is_ok());
        for byte in 0..binding.len() {
            let mut changed = *binding;
            changed[byte] ^= 1;
            assert!(
                unsafe { verify(&image, identity, &changed) }.is_err(),
                "binding byte {byte}"
            );
        }
        for byte in 0..identity.len() {
            let mut changed = *identity;
            changed[byte] ^= 1;
            assert!(
                unsafe { verify(&image, &changed, binding) }.is_err(),
                "identity byte {byte}"
            );
        }
        for byte in [0, 1, image.len() / 2, image.len() - 1] {
            let mut changed = image.clone();
            changed[byte] ^= 1;
            assert!(
                unsafe { verify(&changed, identity, binding) }.is_err(),
                "image byte {byte}"
            );
        }
        let mut extended = image.clone();
        extended.push(b' ');
        assert!(unsafe { verify(&extended, identity, binding) }.is_err());
        assert!(unsafe { verify(&image[..image.len() - 1], identity, binding) }.is_err());
        let other = emit_program_object(&artifacts("input<u64> + 1u64\n")[0], &catalog).unwrap();
        assert!(
            unsafe {
                verify(
                    &image,
                    other.identity().bytes(),
                    other.image_binding().bytes(),
                )
            }
            .is_err()
        );
    }
}

#[test]
fn object_bound_and_independent_images_preserve_all_prepared_metadata() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "input<u64> + 1u64\n",
        include_str!("../../../../tests/fixtures/native/numeric-parent-child.mec"),
        "+> map\n+> sequence\nwords := input<[string]>\nmap/histogram(words)\n",
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let image = object.descriptor().encode_build_owned_image().unwrap();
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                    .unwrap();
            for bytes in [image.clone(), super::super::packing::pack(&image).unwrap()] {
                let restored = unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &bytes,
                        &catalog,
                        object.identity().bytes(),
                        object.image_binding().bytes(),
                    )
                }
                .unwrap();
                assert_eq!(restored.encode_build_owned_image().unwrap(), image);
                assert_eq!(independent.encode_build_owned_image().unwrap(), image);
                assert_eq!(restored.typed_ir_sha256(), independent.typed_ir_sha256());
            }
            // Proof bytes retain the independent loader's existing rejection,
            // while the bound path rejects any changed proof before decoding.
            let mut changed: serde_json::Value = serde_json::from_slice(&image).unwrap();
            if let Some(region) = changed["descriptor"]["closed_tails"]["regions"]
                .as_array_mut()
                .and_then(|regions| {
                    regions
                        .iter_mut()
                        .find(|region| region["numeric_tree"].is_object())
                })
            {
                let word = &mut region["numeric_tree"]["nodes"][0]["compute"];
                *word = (word.as_u64().unwrap() + 1).into();
                let bytes = serde_json::to_vec(&changed).unwrap();
                assert!(
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                        .is_err()
                );
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &bytes,
                            &catalog,
                            object.identity().bytes(),
                            object.image_binding().bytes(),
                        )
                    }
                    .is_err()
                );
            }
        }
    }
}

#[test]
fn object_bound_calls_preserve_independent_selections_and_reject_template_tampering() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "+> sequence\n[at + 1u64 | at <- 0u64..=limit<u64>]\n",
        include_str!("../../../../tests/fixtures/native/window-histogram.mec"),
        "+> text\n+> map\nentries := {\"a\": \"A\", \"b\": \"B\"}\ntext/join([entries{ch} | ch <- seed<string>], \"\")\n",
        include_str!("../../../../tests/fixtures/native/numeric-parent-child.mec"),
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let image = object.descriptor().encode_build_owned_image().unwrap();
            // SAFETY: both paths use the same genuine checked Object metadata;
            // no native entry or execution authority is manufactured here.
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }
                    .unwrap();
            let bound = unsafe {
                NativeProgramDescriptor::decode_object_bound_image(
                    &image,
                    &catalog,
                    object.identity().bytes(),
                    object.image_binding().bytes(),
                )
            }
            .unwrap();
            assert_eq!(bound.encode_build_owned_image().unwrap(), image);
            assert_eq!(independent.encode_build_owned_image().unwrap(), image);
            let plan = bound.plan();
            assert!(plan.prepared_library_template(plan.blocks().len()).is_err());
            for (site, (a, b)) in independent
                .metadata()
                .calls
                .iter()
                .zip(bound.metadata().calls.iter())
                .enumerate()
            {
                assert_eq!(
                    a.as_ref().map(|c| c.template()),
                    b.as_ref().map(|c| c.template())
                );
                assert_eq!(
                    b.as_ref().map(|c| c.template()),
                    plan.prepared_library_template(site).unwrap()
                );
            }
            let mut changed: serde_json::Value = serde_json::from_slice(&image).unwrap();
            let call = changed["descriptor"]["calls"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|call| !call.is_null())
                .expect("fixture must exercise a prepared call");
            call["template"]["origin"]["node"] = u32::MAX.into();
            let changed = serde_json::to_vec(&changed).unwrap();
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog) }
                    .is_err()
            );
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &changed,
                        &catalog,
                        object.identity().bytes(),
                        object.image_binding().bytes(),
                    )
                }
                .is_err()
            );
        }
    }
}

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
fn native_metadata_exports_preserve_the_exact_descriptor_and_object_binding() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        "input<u64> + 1u64\n",
        include_str!("../../../../tests/fixtures/native/numeric-parent-child.mec"),
        "+> map\n+> sequence\nwords := input<[string]>\nmap/histogram(words)\n",
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let json = object.descriptor().encode_build_owned_image().unwrap();
            let bytes = object.native_metadata();
            let binding = object.native_metadata_binding().bytes();
            assert!(bytes.starts_with(mech_engine::native_artifact::NATIVE_METADATA_IMAGE_MAGIC));
            // SAFETY: genuine data, identity and binding from the same checked
            // Object. No foreign entry or fabricated source proof is executed.
            let bound = unsafe {
                NativeProgramDescriptor::decode_object_bound_image(
                    bytes,
                    &catalog,
                    object.identity().bytes(),
                    binding,
                )
            }
            .unwrap();
            let independent =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(bytes, &catalog) }
                    .unwrap();
            assert_eq!(bound.encode_build_owned_image().unwrap(), json);
            assert_eq!(independent.encode_build_owned_image().unwrap(), json);
            assert!(
                unsafe {
                    verify(
                        bytes,
                        object.identity().bytes(),
                        object.image_binding().bytes(),
                    )
                }
                .is_err()
            );
            assert!(unsafe { verify(&json, object.identity().bytes(), binding) }.is_err());
            for byte in 0..binding.len() {
                let mut changed = *binding;
                changed[byte] ^= 1;
                assert!(unsafe { verify(bytes, object.identity().bytes(), &changed) }.is_err());
            }
            for byte in [0, 7, bytes.len() / 2, bytes.len() - 1] {
                let mut changed = bytes.to_vec();
                changed[byte] ^= 1;
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &changed,
                            &catalog,
                            object.identity().bytes(),
                            binding,
                        )
                    }
                    .is_err()
                );
            }
            assert!(
                unsafe {
                    verify(
                        &bytes[..bytes.len() - 1],
                        object.identity().bytes(),
                        binding,
                    )
                }
                .is_err()
            );
            let mut changed = bytes.to_vec();
            changed.push(0);
            assert!(unsafe { verify(&changed, object.identity().bytes(), binding) }.is_err());
        }
    }
}

#[test]
fn native_table_rows_reject_foreign_types_and_mismatched_binding_arity() {
    use serde::Deserialize;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts("+> sequence\nwords := input<[string]>\nsequence/length(words)\n") {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let mut reader = mech_engine::native_artifact::BuildOwnedImageReader::from_image(
            object.native_metadata(),
        )
        .unwrap();
        let image = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        assert_eq!(image["version"], 3);
        let rows = image["descriptor"]["plan"]["values"].as_array().unwrap();
        assert!(
            rows.iter()
                .all(|r| r.as_array().is_some_and(|r| r.len() == 2))
        );
        let scalar = rows.iter().position(|r| r[1] == 5).unwrap();
        let call = image["descriptor"]["calls"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| !r.is_null())
            .unwrap();
        assert_eq!(
            image["descriptor"]["calls"][call].as_array().unwrap().len(),
            12
        );
        for fault in 0..3 {
            let mut bad = image.clone();
            match fault {
                0 => bad["descriptor"]["plan"]["values"][scalar][0] = u32::MAX.into(),
                1 => bad["descriptor"]["plan"]["values"][scalar][1] = 1.into(),
                _ => bad["descriptor"]["calls"][call][3] = u64::MAX.into(),
            }
            let json = serde_json::to_vec(&bad).unwrap();
            let native = mech_engine::native_artifact::encode_native_metadata_image(&json).unwrap();
            // SAFETY: confined negative wire test. No changed descriptor or
            // fabricated proof can escape or reach any native executable.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&native, &catalog) }
                    .is_err()
            );
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_object_bound_image(
                        &native,
                        &catalog,
                        object.identity().bytes(),
                        object.native_metadata_binding().bytes(),
                    )
                }
                .is_err()
            );
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

#[test]
fn frozen_frames_preserve_metadata_and_independent_legacy_validation() {
    let catalog = mech_stdlib::source_catalog();
    for source in [
        include_str!("../../../../tests/fixtures/native/numeric-parent-named-managed.mec"),
        include_str!("../../../../tests/fixtures/native/private-tagged-map.mec"),
        include_str!("../../../../tests/fixtures/native/window-histogram.mec"),
    ] {
        for artifact in artifacts(source) {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let bytes = object.descriptor().encode_build_owned_image().unwrap();
            let image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(image["version"], 2);
            assert!(image["descriptor"]["buffer_frames"].is_object());
            for bound in [false, true] {
                // SAFETY: complete metadata and symbols come from one genuine
                // checked Object. No entry authority is fabricated here.
                let restored = if bound {
                    unsafe {
                        NativeProgramDescriptor::decode_object_bound_image(
                            &bytes,
                            &catalog,
                            object.identity().bytes(),
                            object.image_binding().bytes(),
                        )
                    }
                } else {
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                }
                .unwrap();
                assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
            }
            let mut legacy = image.clone();
            legacy["version"] = 1.into();
            legacy["descriptor"]
                .as_object_mut()
                .unwrap()
                .remove("buffer_frames");
            // Build-side compatibility conversion only; no changed image is
            // paired with an entry or allowed to bypass Object provenance.
            let restored = unsafe {
                NativeProgramDescriptor::decode_build_owned_image(
                    &serde_json::to_vec(&legacy).unwrap(),
                    &catalog,
                )
            }
            .unwrap();
            assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
            for mode in 0..4 {
                let mut changed = image.clone();
                match mode {
                    0 => {
                        let old = changed["descriptor"]["buffer_frames"]["scalar_fields"]
                            .as_u64()
                            .unwrap();
                        changed["descriptor"]["buffer_frames"]["scalar_fields"] = (old + 1).into();
                    }
                    1 => {
                        changed["descriptor"]
                            .as_object_mut()
                            .unwrap()
                            .remove("buffer_frames");
                    }
                    2 => {
                        changed["descriptor"]["buffer_frames"] = serde_json::Value::Null;
                    }
                    _ => {
                        changed["version"] = 1.into();
                    }
                }
                let changed = serde_json::to_vec(&changed).unwrap();
                assert!(
                    unsafe {
                        NativeProgramDescriptor::decode_build_owned_image(&changed, &catalog)
                    }
                    .is_err(),
                    "case {mode}"
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
                    .is_err(),
                    "case {mode}"
                );
            }
        }
    }
}

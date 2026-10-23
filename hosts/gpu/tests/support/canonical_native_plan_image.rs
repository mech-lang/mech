//! Build-side images are deterministic metadata, not an execution authority.
use super::*;
use mech_engine::native_artifact::{
    encode_native_program_plan_image, lower_native_program_artifact,
};

#[test]
fn build_owned_images_reject_wrong_registry_revision_and_broken_inventories() {
    let (artifacts, catalog) = artifacts(FULL);
    let artifact = &artifacts[0];
    let jit = JitProgramArtifact::compile(artifact, &catalog).unwrap();
    let bytes = jit.descriptor().encode_build_owned_image().unwrap();
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for (pointer, replacement) in [
        ("/version", serde_json::json!(2)),
        ("/descriptor/code", serde_json::json!(0)),
        ("/descriptor/calls", serde_json::json!([])),
        ("/descriptor/types/0/schema", serde_json::json!(u32::MAX)),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        // SAFETY: rejection probe only; no reconstructed metadata can execute.
        assert!(
            unsafe {
                mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &serde_json::to_vec(&changed).unwrap(),
                    &catalog,
                )
            }
            .is_err(),
            "{pointer}"
        );
    }
    let empty = mech_core::FunctionCatalogBuilder::new().build().unwrap();
    // SAFETY: rejection probe confined to this test, never an executable.
    assert!(
        unsafe {
            mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                &bytes, &empty,
            )
        }
        .is_err()
    );

    let resources = CanonicalNativeResourceMetadata::prepare(artifact, 16).unwrap();
    let bytes = resources.encode_build_owned_image().unwrap();
    let other_revision = self::artifacts(ECHO).0[0].revision();
    // SAFETY: mismatched metadata is inspected only, never used with a session.
    assert!(
        unsafe {
            CanonicalNativeResourceMetadata::decode_build_owned_image(
                &bytes,
                other_revision,
                artifact.schemas().clone(),
            )
        }
        .is_err()
    );
    let mut changed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    changed["plan"]["requirements"] = serde_json::json!([]);
    // SAFETY: rejection probe cannot enter resource admission or native code.
    assert!(
        unsafe {
            CanonicalNativeResourceMetadata::decode_build_owned_image(
                &serde_json::to_vec(&changed).unwrap(),
                artifact.revision(),
                artifact.schemas().clone(),
            )
        }
        .is_err()
    );
}

#[test]
fn generated_hosted_application_pins_the_same_object_descriptor() {
    use sha2::{Digest, Sha256};
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (variants, catalog) = artifacts(FULL);
    let mut images = Vec::new();
    for (index, artifact) in variants.iter().enumerate() {
        let project = mech_gpu::native_artifact::generate_linked_hosted_program_application(
            &workspace,
            &directory.path().join(format!("project-{index}")),
            "descriptor-image-test",
            artifact,
            &catalog,
            FULL.as_bytes(),
        )
        .unwrap();
        let image = std::fs::read(project.root().join("native-descriptor.json")).unwrap();
        let digest = Sha256::digest(&image)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(project.pins()["native-descriptor.json"], digest);
        let packed = std::fs::read(project.root().join("native-descriptor.bin")).unwrap();
        let packed_digest = Sha256::digest(&packed)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(project.pins()["native-descriptor.bin"], packed_digest);
        assert!(packed.len() < image.len());
        // SAFETY: this same checked compiler produced the image and Object;
        // the decoded metadata remains confined to round-trip inspection.
        let restored = unsafe {
            mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                &packed, &catalog,
            )
        }
        .unwrap();
        assert_eq!(image, restored.encode_build_owned_image().unwrap());
        // Header length/size checks precede allocation; checksum, truncation,
        // exact consumption and exact expanded length are enforced as well.
        let mut corruptions = vec![packed[..12].to_vec(), packed[..packed.len() - 1].to_vec()];
        for offset in [0, 8, 16, 24, packed.len() - 1] {
            let mut changed = packed.clone();
            changed[offset] ^= 1;
            corruptions.push(changed);
        }
        let mut oversized = packed.clone();
        oversized[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
        corruptions.push(oversized);
        let mut trailing = packed.clone();
        trailing.push(0);
        let payload_len = u64::try_from(trailing.len() - 24).unwrap();
        trailing[16..24].copy_from_slice(&payload_len.to_le_bytes());
        corruptions.push(trailing);
        for changed in corruptions {
            // SAFETY: rejection probe only; no returned proof can execute.
            assert!(
                unsafe {
                    mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                        &changed, &catalog,
                    )
                }
                .is_err()
            );
        }
        let resources = std::fs::read(project.root().join("native-resources.json")).unwrap();
        let resource_digest = Sha256::digest(&resources)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(project.pins()["native-resources.json"], resource_digest);
        let prepared = CanonicalNativeResourceMetadata::prepare(artifact, usize::MAX).unwrap();
        assert_eq!(resources, prepared.encode_build_owned_image().unwrap());
        // SAFETY: this same checked build produced both the image and Object.
        let restored_resources = unsafe {
            CanonicalNativeResourceMetadata::decode_build_owned_image(
                &resources,
                artifact.revision(),
                artifact.schemas().clone(),
            )
        }
        .unwrap();
        assert_eq!(
            resources,
            restored_resources.encode_build_owned_image().unwrap()
        );
        assert_eq!(
            restored_resources.plan().requirements().len(),
            artifact.requirements().len()
        );
        let main = std::fs::read_to_string(project.root().join("src/main.rs")).unwrap();
        assert!(!main.contains("decode_program_artifact"));
        assert!(!main.contains("CanonicalNativeResourceMetadata::prepare"));
        assert!(!main.contains("lower_native_program_artifact"));
        assert!(main.contains("LinkedProgramArtifact::from_build_owned_image"));
        assert!(main.contains("include_bytes!(\"../native-descriptor.bin\")"));
        assert!(!main.contains("include_bytes!(\"../native-descriptor.json\")"));
        let object = mech_gpu::native_artifact::emit_program_object(artifact, &catalog).unwrap();
        assert_eq!(
            image,
            object.descriptor().encode_build_owned_image().unwrap()
        );
        assert_eq!(
            std::fs::read(project.root().join("program.o")).unwrap(),
            object.bytes()
        );
        assert_eq!(
            std::fs::read(project.root().join("object-identity.bin")).unwrap(),
            object.identity().bytes()
        );
        images.push(image);
    }
    assert_eq!(images[0], images[1]);
}

#[test]
fn native_descriptor_images_preserve_layouts_without_process_pointers() {
    for source in [
        FULL,
        ECHO,
        CLOSED_FAIL,
        CLOSED_LIVE,
        FAIL,
        "18446744073709551617u128\n",
        "340282366920938463463374607431768211455u128\n",
    ] {
        let (variants, catalog) = artifacts(source);
        let images = variants
            .iter()
            .map(|artifact| {
                let jit = JitProgramArtifact::compile(artifact, &catalog).unwrap();
                let descriptor = jit.descriptor();
                let bytes = descriptor.encode_build_owned_image().unwrap();
                assert_eq!(bytes, descriptor.encode_build_owned_image().unwrap());
                // SAFETY: unchanged output from this checked JIT descriptor;
                // restored metadata is compared only and never executed here.
                let restored = unsafe {
                    mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                        &bytes, &catalog,
                    )
                }.unwrap();
                assert_eq!(bytes, restored.encode_build_owned_image().unwrap());
                let image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(image["format"], "mech.build-owned-native-descriptor");
                assert_eq!(image["version"], 1);
                let body = &image["descriptor"];
                assert_eq!(body["stack"], descriptor.stack_bytes());
                assert_eq!(body["code"], descriptor.code_bytes());
                assert_eq!(
                    body["identity"],
                    serde_json::json!(descriptor.typed_ir_sha256())
                );
                assert!(!body["schemas"].as_array().unwrap().is_empty());
                assert_eq!(
                    body["visits"].as_array().unwrap().len(),
                    descriptor.plan().blocks().len()
                );
                assert_eq!(
                    body["calls"].as_array().unwrap().len(),
                    descriptor.plan().blocks().len()
                );
                for call in body["calls"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| !v.is_null())
                {
                    assert!(call.get("template").is_some());
                    for process_local in ["factory", "cache_identity", "schemas"] {
                        assert!(call.get(process_local).is_none(), "{process_local}");
                    }
                    // SAFETY: metadata-only round trip of the unchanged JIT
                    // descriptor. No restored proof escapes to execution.
                    let restored = unsafe {
                        mech_engine::native_artifact::NativePreparedLibraryCall::from_build_owned_image(
                            call,
                            std::sync::Arc::new(artifact.schemas().clone()),
                            &catalog,
                        )
                    }
                    .unwrap();
                    assert_eq!(*call, serde_json::to_value(restored).unwrap());
                    for field in ["output_parameters", "input_keys", "factory"] {
                        let mut changed = call.clone();
                        changed[field] = serde_json::json!(999999);
                        // SAFETY: rejection probe only, never an executable.
                        assert!(unsafe {
                            mech_engine::native_artifact::NativePreparedLibraryCall::from_build_owned_image(
                                &changed,
                                std::sync::Arc::new(artifact.schemas().clone()),
                                &catalog,
                            )
                        }.is_err());
                    }
                }
                let plan: serde_json::Value = serde_json::from_slice(
                    &encode_native_program_plan_image(descriptor.plan()).unwrap(),
                )
                .unwrap();
                assert_eq!(body["plan"], plan["plan"]);
                bytes
            })
            .collect::<Vec<_>>();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0], images[1]);
    }
}

#[test]
fn native_plan_images_preserve_original_and_decoded_artifacts() {
    for source in [FULL, ECHO, CLOSED_FAIL, CLOSED_LIVE, FAIL] {
        let (variants, catalog) = artifacts(source);
        let images = variants
            .iter()
            .map(|artifact| {
                let plan = lower_native_program_artifact(artifact, &catalog).unwrap();
                let bytes = encode_native_program_plan_image(&plan).unwrap();
                // SAFETY: unchanged image of this artifact's just-lowered plan,
                // paired with its exact schemas and catalog; metadata-only test.
                let restored = unsafe {
                    mech_engine::native_artifact::decode_build_owned_native_plan_image(
                        &bytes,
                        artifact.schemas(),
                        &catalog,
                    )
                }
                .unwrap();
                assert_eq!(bytes, encode_native_program_plan_image(&restored).unwrap());
                assert_eq!(bytes, encode_native_program_plan_image(&plan).unwrap());
                let image: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(image["format"], "mech.build-owned-native-plan");
                assert_eq!(image["version"], 1);
                for field in ["blocks", "values", "observations", "effects"] {
                    assert!(
                        !image["plan"][field].as_array().unwrap().is_empty(),
                        "{field}"
                    );
                }
                bytes
            })
            .collect::<Vec<_>>();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0], images[1]);
    }
}

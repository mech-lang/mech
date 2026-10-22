//! Build-side images are deterministic metadata, not an execution authority.
use super::*;
use mech_engine::native_artifact::{
    encode_native_program_plan_image, lower_native_program_artifact,
};

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
    for source in [FULL, ECHO, CLOSED_FAIL, CLOSED_LIVE, FAIL] {
        let (variants, catalog) = artifacts(source);
        let images = variants
            .iter()
            .map(|artifact| {
                let jit = JitProgramArtifact::compile(artifact, &catalog).unwrap();
                let descriptor = jit.descriptor();
                let bytes = descriptor.encode_build_owned_image().unwrap();
                assert_eq!(bytes, descriptor.encode_build_owned_image().unwrap());
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

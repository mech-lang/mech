//! Wire/hash negatives for current canonical Record-to-Map views.
//! Forged recipes are inspected only; none may reach generated execution.
use super::{NativeProgramDescriptor, identity};
use crate::native_artifact::program::prepare::borrowed_record_map::Layout;
use crate::native_artifact::{JitProgramArtifact, emit_program_object};
use mech_engine::ProgramArtifact;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const SOURCE: &str = include_str!("../../../../tests/fixtures/native/retained-compound-map.mec");

fn artifacts() -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x42524d494d47), Revision(1), SOURCE).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let original = mech_engine::CanonicalSourceFrontend
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

fn hash(descriptor: &NativeProgramDescriptor, borrowed: &Layout) -> [u8; 32] {
    let metadata = descriptor.metadata();
    let plan = descriptor.plan();
    identity::typed_ir_sha256(
        plan,
        &plan.text_line_retention_frontiers(),
        &metadata.selected_joins.regions,
        &metadata.line_matrices.regions,
        metadata.map_owner_words.as_ref(),
        &metadata.frozen_maps,
        borrowed,
        &metadata.string_map_regions,
    )
}

#[test]
fn borrowed_record_map_images_rederive_and_hash_complete_current_source_recipes() {
    let catalog = mech_stdlib::source_catalog();
    let mut previous = None;
    for artifact in artifacts() {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let descriptor = jit.descriptor();
        let owners = descriptor.metadata().map_owner_words.as_ref().unwrap();
        let derived = Layout::derive(descriptor.plan(), owners).unwrap();
        assert_eq!(derived, descriptor.metadata().borrowed_record_maps);
        assert!(!derived.regions.is_empty(), "fixture must select the view");
        assert_eq!(*descriptor.typed_ir_sha256(), hash(descriptor, &derived));
        let original = serde_json::to_value(&derived).unwrap();
        if let Some(previous) = &previous {
            assert_eq!(previous, &original, "source/bytecode proof differs");
        }
        previous = Some(original.clone());
        let encoded = descriptor.encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(image["descriptor"]["borrowed_record_maps"], original);
        // SAFETY: exact unchanged image from this checked build; the restored
        // descriptor remains confined to the current compiled owner's lifetime.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                .unwrap();
        assert_eq!(restored.metadata().borrowed_record_maps, derived);
        assert_eq!(restored.typed_ir_sha256(), descriptor.typed_ir_sha256());

        let mut changes = Vec::new();
        for field in [
            "owner", "exit", "source", "ordinal", "output", "project", "read",
        ] {
            let mut changed = original.clone();
            let word = &mut changed["regions"][0][field];
            *word = (word.as_u64().unwrap() + 1).into();
            changes.push(changed);
        }
        let mut iteration = original.clone();
        let word = &mut iteration["regions"][0]["iteration"];
        *word = if word.is_null() {
            0.into()
        } else {
            serde_json::Value::Null
        };
        changes.push(iteration);
        // This is a real, same-schema private returned Record, not an invalid
        // value index. Only the canonical exit result may authorize the view.
        let first = &derived.regions[0];
        let private = owners.regions()[first.owner].tail().returned().get();
        assert_ne!(private, first.source);
        assert_eq!(
            descriptor.plan().values()[private as usize].schema(),
            descriptor.plan().values()[first.source as usize].schema()
        );
        let mut private_root = original.clone();
        private_root["regions"][0]["source"] = private.into();
        changes.push(private_root);
        for field in ["projects", "reads"] {
            let mut missing = original.clone();
            let words = missing[field].as_array_mut().unwrap();
            let current = words.iter_mut().find(|word| !word.is_null()).unwrap();
            *current = serde_json::Value::Null;
            changes.push(missing);
            let mut extra = original.clone();
            extra[field]
                .as_array_mut()
                .unwrap()
                .push(serde_json::Value::Null);
            changes.push(extra);
        }
        let mut duplicate = original.clone();
        duplicate["regions"]
            .as_array_mut()
            .unwrap()
            .push(original["regions"][0].clone());
        changes.push(duplicate);
        changes.push(serde_json::to_value(Layout::default()).unwrap());
        for changed in changes {
            let proof: Layout = serde_json::from_value(changed.clone()).unwrap();
            assert_ne!(hash(descriptor, &proof), *descriptor.typed_ir_sha256());
            let mut changed_image = image.clone();
            changed_image["descriptor"]["borrowed_record_maps"] = changed;
            let encoded = serde_json::to_vec(&changed_image).unwrap();
            // SAFETY: isolated malformed wire-only diagnostic; no returned
            // descriptor is attached to an Object or used for execution.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                    .is_err()
            );
        }
        let mut missing = image.clone();
        missing["descriptor"]
            .as_object_mut()
            .unwrap()
            .remove("borrowed_record_maps");
        let mut unknown = image;
        unknown["descriptor"]["borrowed_record_maps"]["permit"] = true.into();
        let mut malformed_images = vec![missing, unknown];
        for field in ["regions", "projects", "reads"] {
            let mut missing = serde_json::from_slice::<serde_json::Value>(&encoded).unwrap();
            missing["descriptor"]["borrowed_record_maps"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            malformed_images.push(missing);
        }
        for malformed in malformed_images {
            let encoded = serde_json::to_vec(&malformed).unwrap();
            // SAFETY: malformed metadata is never attached or executed.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                    .is_err()
            );
        }
    }
}

#[test]
fn borrowed_record_map_object_identity_rejects_ir151_and_work103_images() {
    let catalog = mech_stdlib::source_catalog();
    assert!(identity::NATIVE_PROGRAM_TYPED_IR_VERSION > 151);
    assert!(identity::NATIVE_PROGRAM_WORK_POLICY_VERSION > 103);
    for artifact in artifacts() {
        let object = emit_program_object(&artifact, &catalog).unwrap();
        let identity = object.identity();
        crate::native_artifact::program::linked::prepare_descriptor(
            &artifact,
            &catalog,
            identity.bytes(),
        )
        .unwrap();
        for (old_ir, old_work) in [(true, false), (false, true), (true, true)] {
            let mut old = *identity.bytes();
            if old_ir {
                old[16..24].copy_from_slice(&151u64.to_le_bytes());
            }
            if old_work {
                old[32..40].copy_from_slice(&103u64.to_le_bytes());
            }
            assert!(
                crate::native_artifact::program::linked::prepare_descriptor(
                    &artifact, &catalog, &old,
                )
                .is_err()
            );
        }
    }
}

//! Hash the complete same-plan inventories, independently of capture eligibility.
use super::super::super::{JitProgramArtifact, program_identity};
use super::call_cache_tests::artifacts;

#[test]
fn map_owner_descriptor_images_preserve_complete_same_build_layout() {
    use crate::native_artifact::NativeProgramDescriptor;
    let source = include_str!("../../tests/fixtures/native/map-owner-binding-state.mec");
    let catalog = mech_stdlib::source_catalog();
    let mut previous = None;
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let descriptor = jit.descriptor();
        assert_eq!(descriptor.map_owner_words.as_ref().unwrap().regions().len(), 1);
        let expected = serde_json::to_value(descriptor.plan.map_owner_words().unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&descriptor.map_owner_words).unwrap(), expected);
        let encoded = descriptor.encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(image["descriptor"]["map_owner_words"], expected);
        // SAFETY: the unchanged image belongs to this exact checked JIT build.
        // The restored descriptor is confined to this owner's lifetime.
        let restored = unsafe {
            NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog)
        }.unwrap();
        assert_eq!(serde_json::to_value(&restored.map_owner_words).unwrap(), expected);
        assert_eq!(restored.typed_ir_sha256(), descriptor.typed_ir_sha256());
        if let Some(prior) = &previous {
            assert_eq!(prior, &expected, "source/decoded layout differs");
        }
        previous = Some(expected);

        // Missing or extra wire fields are format errors, not defaults that
        // may silently select an older owner's transport protocol.
        let mut missing = image.clone();
        missing["descriptor"].as_object_mut().unwrap().remove("map_owner_words");
        let missing = serde_json::to_vec(&missing).unwrap();
        // SAFETY: malformed wire-only diagnostic; no result may execute.
        assert!(unsafe {
            NativeProgramDescriptor::decode_build_owned_image(&missing, &catalog)
        }.is_err());
        let mut extra = image;
        extra["descriptor"]["map_owner_words"]["permit"] = true.into();
        let extra = serde_json::to_vec(&extra).unwrap();
        // SAFETY: malformed wire-only diagnostic; no result may execute.
        assert!(unsafe {
            NativeProgramDescriptor::decode_build_owned_image(&extra, &catalog)
        }.is_err());
    }
}

#[test]
fn map_owner_identity_covers_unused_candidates_words_and_site_inventories() {
    use mech_engine::native_artifact::NativeMapOwnerWords;
    let source = include_str!("../../tests/fixtures/native/map-owner-binding-state.mec");
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let descriptor = jit.descriptor();
        let plan = &descriptor.plan;
        let frontiers = plan.text_line_retention_frontiers();
        let joins = plan.selected_text_joins();
        let lines = plan.text_line_matrix_regions();
        let original = serde_json::to_value(&descriptor.map_owner_words).unwrap();
        let mut changes = Vec::new();
        for path in [
            "/frame_words", "/aggregates/words", "/regions/0/iteration",
            "/regions/0/tail/seed", "/regions/0/tail/exit",
        ] {
            let mut changed = original.clone();
            let field = changed.pointer_mut(path).unwrap();
            *field = (field.as_u64().unwrap() + 1).into();
            changes.push(changed);
        }
        for path in [
            "/regions", "/regions/0/values", "/regions/0/writes", "/regions/0/reads",
            "/initializers", "/exits", "/aggregates/slots", "/aggregates/actions",
        ] {
            let mut changed = original.clone();
            let items = changed.pointer_mut(path).unwrap().as_array_mut().unwrap();
            assert!(!items.is_empty(), "fixture must exercise {path}");
            items.push(items[0].clone());
            changes.push(changed);
        }
        let mut empty = original.clone();
        empty["regions"] = serde_json::json!([]);
        changes.push(empty);
        let mut constant = original.clone();
        constant["aggregates"]["constants"].as_array_mut().unwrap()
            .push(serde_json::json!([0, [u64::MAX]]));
        changes.push(constant);
        for changed in changes {
            // SAFETY: isolated wire-only corruption for hashing. It is never
            // attached to a descriptor, used for lowering or executed.
            let changed = unsafe { NativeMapOwnerWords::from_build_owned_image(&changed) }.unwrap();
            assert_ne!(
                *descriptor.typed_ir_sha256(),
                program_identity::typed_ir_sha256(plan, &frontiers, &joins, &lines, Some(&changed)),
                "a changed candidate/layout must change Object association",
            );
        }
    }
}

#[test]
fn shared_descriptor_proofs_preserve_fresh_identity_and_complete_inventory() {
    for (name, source, expected_joins, expected_lines, expected_ranges) in [
        ("scalar", "input<u64> + 1u64\n", 0, 0, 0),
        (
            "indices",
            "+> text\nwords := input<[string]>\nindices := positions<[u64]>\ntext/join(words[indices], separator<string>)\n",
            1,
            0,
            0,
        ),
        (
            "line-length",
            "+> text\n+> sequence\nsequence/length(text/lines(input<string>))\n",
            0,
            1,
            0,
        ),
        (
            "line-range",
            "+> text\nrows := text/lines(input<string>)\ntext/join(rows[first<u64>..last<u64>], \"\")\n",
            1,
            1,
            1,
        ),
        (
            "multiple-lines-and-joins",
            "+> text\nleft := text/lines(a<string>)\nright := text/lines(b<string>)\nx := text/join(left[first<u64>..last<u64>], \"\")\ny := text/join(right[first..last], \"|\")\n(x,y)\n",
            2,
            2,
            2,
        ),
    ] {
        let mut original_identity = None;
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let descriptor = jit.descriptor();
            let plan = &descriptor.plan;
            let frontiers = plan.text_line_retention_frontiers();
            let joins = plan.selected_text_joins();
            let lines = plan.text_line_matrix_regions();
            let owners = plan.map_owner_words().unwrap();
            assert!(owners.regions().is_empty());
            assert!(descriptor.map_owner_words.is_none());
            let owners = None;
            assert_eq!(joins.len(), expected_joins, "{name}");
            assert_eq!(lines.len(), expected_lines, "{name}");
            assert_eq!(
                joins.iter().filter(|join| join.range().is_some()).count(),
                expected_ranges,
                "{name}"
            );
            let fresh = program_identity::typed_ir_sha256(plan, &frontiers, &joins, &lines, owners);
            assert_eq!(*descriptor.typed_ir_sha256(), fresh, "{name}");
            assert_eq!(descriptor.selected_joins.regions.len(), joins.len());
            assert_eq!(descriptor.line_matrices.regions.len(), lines.len());
            if let Some(original) = original_identity {
                assert_eq!(original, fresh, "original/decoded {name}");
            } else {
                original_identity = Some(fresh);
            }
            // Omitting, duplicating or reordering proofs must affect identity
            // even when a prefix layout cannot use all of those proofs.
            if !joins.is_empty() {
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &[], &lines, owners)
                );
                let mut duplicated = joins.clone();
                duplicated.extend(joins.iter().cloned());
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &duplicated, &lines, owners)
                );
            }
            if !lines.is_empty() {
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &joins, &[], owners)
                );
                let mut duplicated = lines.clone();
                duplicated.extend(lines.iter().cloned());
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &joins, &duplicated, owners)
                );
            }
            if joins.len() > 1 {
                let mut reversed = joins.clone();
                reversed.reverse();
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &reversed, &lines, owners)
                );
            }
            if lines.len() > 1 {
                let mut reversed = lines.clone();
                reversed.reverse();
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &joins, &reversed, owners)
                );
            }
        }
    }
}

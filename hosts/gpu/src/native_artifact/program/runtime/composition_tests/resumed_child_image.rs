//! Restored metadata cannot widen a secondary's source, storage or prices.
use super::*;

#[test]
fn resumed_child_images_reconstruct_entries_ownership_offsets_and_prices() {
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &catalog,
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = native.descriptor().metadata();
        let (id, region) = metadata
            .closed_tails
            .regions
            .iter()
            .enumerate()
            .find(|(id, region)| {
                metadata.closed_tails.sites[region.tail.header() as usize] != Some(*id)
            })
            .expect("a separately selected complete child");
        let primary = metadata.closed_tails.sites[region.tail.header() as usize].unwrap();
        let proof = region.numeric.as_ref().unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&native.descriptor().encode_build_owned_image().unwrap())
                .unwrap();
        let base = format!("/descriptor/closed_tails/regions/{id}");
        let parent_exits = image
            .pointer(&format!(
                "/descriptor/closed_tails/regions/{primary}/numeric/exits"
            ))
            .unwrap()
            .clone();
        assert!(!parent_exits.as_array().unwrap().is_empty());
        for (path, replacement) in [
            (
                format!(
                    "/descriptor/closed_tails/initializers/{}",
                    region.tail.initialize()
                ),
                serde_json::json!(primary),
            ),
            (
                format!("/descriptor/closed_tails/exits/{}", region.tail.exit()),
                serde_json::json!(primary),
            ),
            (
                format!("/descriptor/closed_tails/sites/{}", region.tail.header()),
                serde_json::json!(id),
            ),
            (format!("{base}/words"), serde_json::json!(region.words + 1)),
            (
                format!("{base}/shadow"),
                serde_json::json!(region.shadow.unwrap() + 1),
            ),
            (
                format!("{base}/compute"),
                serde_json::json!(region.compute + 1),
            ),
            (
                format!("{base}/numeric/sites"),
                serde_json::json!(&proof.sites[1..]),
            ),
            (
                format!("{base}/numeric/prepaid_sites"),
                serde_json::json!([region.tail.header()]),
            ),
            (format!("{base}/numeric/exits"), parent_exits),
        ] {
            let mut bad = image.clone();
            let current = bad.pointer_mut(&path).unwrap();
            assert_ne!(*current, replacement);
            *current = replacement;
            assert!(
                super::super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&bad).unwrap(),
                ),
                "{path}"
            );
        }
        let identity = crate::native_artifact::program::image::identity::NativeProgramObjectIdentity::from_descriptor(native.descriptor()).unwrap();
        assert_eq!(
            u64::from_le_bytes(identity.bytes()[16..24].try_into().unwrap()),
            177
        );
        for old_revision in [175u64, 176] {
            let mut old = *identity.bytes();
            old[16..24].copy_from_slice(&old_revision.to_le_bytes());
            assert!(
                crate::native_artifact::program::linked::prepare_descriptor(
                    &artifact, &catalog, &old,
                )
                .is_err(),
                "old IR{old_revision} Object must be rejected"
            );
        }
    }
}

//! Each numeric exit preserves its current consumers, not every historical lane.
use super::*;

const GROWING: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-nested-phase-growing.mec");

#[test]
fn numeric_exit_families_preserve_only_consumed_physical_words() {
    let mut kinds = std::collections::BTreeSet::new();
    for source in [
        mutable_children::SOURCE,
        GROWING,
        include_str!("../../../../../tests/fixtures/native/numeric-nested-rebase.mec"),
    ] {
        for artifact in artifacts(source) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let report = object.metadata();
            let rows = report["compiled_numeric_write_loops"].as_array().unwrap();
            let row = rows
                .iter()
                .max_by_key(|r| r["register_words"].as_u64().unwrap())
                .unwrap();
            let all = row["register_words"].as_u64().unwrap();
            let exits = row
                .get("boundary_spill_words")
                .and_then(|v| v.as_array())
                .expect("the actual emitted exit families need exact physical inventories");
            for exit in exits {
                let count = exit[2].as_u64().unwrap();
                assert!(count > 0 && count < all, "{row}");
                kinds.insert(exit[0].as_str().unwrap().to_owned());
            }
        }
    }
    for kind in ["header", "exit", "failure", "ordinary", "child"] {
        assert!(
            kinds.contains(kind),
            "missing executed emission family {kind}: {kinds:?}"
        );
    }
}

#[test]
fn physical_exit_values_are_reconstructed_from_canonical_source() {
    for artifact in artifacts(mutable_children::SOURCE) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let regions = image
            .pointer("/descriptor/closed_tails/regions")
            .unwrap()
            .as_array()
            .unwrap();
        let (id, region) = regions
            .iter()
            .enumerate()
            .find(|(_, r)| {
                r.pointer("/numeric/phase/checked_children") == Some(&serde_json::json!(true))
            })
            .unwrap();
        for suffix in [
            "boundary_values/header",
            "boundary_values/candidates",
            "boundary_values/exit",
            "exits/0/values",
        ] {
            let values = region
                .pointer(&format!("/numeric/{suffix}"))
                .unwrap()
                .as_array()
                .unwrap();
            assert!(!values.is_empty());
            let mut duplicate = values.clone();
            duplicate.push(values[0].clone());
            for replacement in [serde_json::json!([]), serde_json::json!(duplicate)] {
                let mut bad = image.clone();
                *bad.pointer_mut(&format!(
                    "/descriptor/closed_tails/regions/{id}/numeric/{suffix}"
                ))
                .unwrap() = replacement;
                assert!(
                    super::super::tests::rejects_malformed_helper_image(
                        &serde_json::to_vec(&bad).unwrap()
                    ),
                    "{suffix}"
                );
            }
        }
    }
}

#[test]
fn physical_exit_transport_keeps_finite_admission_publication_and_retry() {
    resources::finite_boundaries(GROWING, |artifact, memory| {
        inputs(
            artifact,
            &[
                ("nodes", Data::Matrix([Data::U64(9)].into())),
                ("steps", Data::U64(3)),
                ("allowed", Data::Bool(true)),
                ("deny-at", Data::U64(u64::MAX)),
                ("divisor", Data::U64(1)),
            ],
            memory,
        )
    });
}

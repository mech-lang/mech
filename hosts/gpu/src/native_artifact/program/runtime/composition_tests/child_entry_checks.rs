//! Predicate proofs use the arguments just transferred at this child edge.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn entry_checks(metadata: &serde_json::Value) -> Vec<&serde_json::Value> {
    [
        "compiled_readonly_numeric_loops",
        "compiled_numeric_write_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().unwrap())
    .flat_map(|region| region["proven_child_entry_checks"].as_array().unwrap())
    .collect()
}

#[test]
fn numeric_child_entry_checks_use_checked_current_arguments_and_keep_full_windows() {
    let source = captured_windows::four_source();
    for artifact in artifacts(&source) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let checks = entry_checks(&metadata);
        assert!(
            checks.iter().any(|row| row[1]["Le"][0]["Constant"] == 8),
            "the successful end=start+32 must establish the word-width lower bound: {checks:?}"
        );
        assert!(
            checks
                .iter()
                .any(|row| row[1]["Le"][0]["Word"].is_number()
                    && row[1]["Le"][1]["Word"].is_number()),
            "the same checked addition must establish start<=end: {checks:?}"
        );
        assert!(
            checks.iter().all(|row| row[1].get("Fits").is_none()),
            "a smaller end does not prove either complete maximum window: {checks:?}"
        );
    }
    let wrapped = source.replace(
        "end := start + 32u64",
        "end := bits/wrapping-add(start, 32u64)",
    );
    assert_ne!(source, wrapped);
    for artifact in artifacts(&wrapped) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        assert!(
            !metadata["compiled_readonly_numeric_loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            entry_checks(&metadata)
                .iter()
                .all(|row| row[1]["Le"][0]["Constant"] != 8),
            "a ring sum alone has no word-width lower bound"
        );
    }
}

#[test]
fn numeric_child_entry_checks_keep_views_overflow_publication_aliases_and_recovery() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&captured_windows::four_source()) {
        let catalog = mech_stdlib::source_catalog();
        let pair = [
            JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap(),
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap(),
        ];
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            let cases = [
                ([1, 256], vec![0u8; 256], [128, 48, 0, 64, 2], Some(128)),
                ([256, 1], vec![0u8; 256], [112, 32, 8, 40, 3], Some(120)),
                (
                    [1, 256],
                    vec![0u8; 256],
                    [128, 48, u64::MAX - 16, 64, 1],
                    None,
                ),
                ([1, 7], vec![0u8; 7], [0, 0, 0, 16, 1], Some(16)),
                ([16, 16], vec![0u8; 256], [128, 48, 0, 64, 2], Some(128)),
                (
                    [1, 0],
                    vec![],
                    [u64::MAX, u64::MAX, u64::MAX, u64::MAX, 0],
                    Some(0),
                ),
                (
                    [1, 256],
                    {
                        let mut bytes = vec![0u8; 256];
                        bytes[176] = 1;
                        bytes
                    },
                    [128, 48, 0, 64, 2],
                    Some(96),
                ),
                ([1, 256], vec![0u8; 256], [128, 48, 0, 64, 2], Some(128)),
            ];
            for (dimensions, bytes, coordinates, expected) in cases {
                let args = captured_windows::arguments(
                    &artifact,
                    &memory,
                    dimensions,
                    &bytes,
                    coordinates,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut prior_error = None;
                for (side, jit) in pair.iter().enumerate() {
                    let prior = output[side].clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let result = if restored {
                        observe::run_restored(
                            jit,
                            &args,
                            &mut output[side],
                            mode.execution(),
                            &memory,
                        )
                        .0
                    } else {
                        observe::run(jit, &args, &mut output[side], mode.execution(), &memory).0
                    };
                    if let Some(number) = expected {
                        result.unwrap();
                        assert_eq!(
                            output[side]
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(number)
                        );
                    } else {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains("Arithmetic"), "{error}");
                        if side == 0 {
                            prior_error = Some(error.clone());
                        }
                        assert_eq!(Some(error), prior_error);
                        read_packs::unchanged_publication(&output[side], &prior);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_child_entry_checks_preserve_exact_finite_prefixes_and_retry() {
    resources::finite_boundaries(&captured_windows::four_source(), |artifact, memory| {
        captured_windows::arguments(artifact, memory, [1, 256], &[0; 256], [128, 48, 0, 64, 2])
    });
}

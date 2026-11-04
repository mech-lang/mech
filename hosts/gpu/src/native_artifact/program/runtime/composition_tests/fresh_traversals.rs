//! A fresh source traversal must not retain dead historical scalar state.
use super::*;

#[test]
fn fresh_traversal_metadata_omits_historical_temporary_inputs() {
    for artifact in artifacts(mutable_children::SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let report = object.metadata();
        let rows = report["compiled_numeric_write_loops"].as_array().unwrap();
        let largest = rows
            .iter()
            .max_by_key(|r| r["register_words"].as_u64().unwrap())
            .unwrap();
        let fresh = largest
            .get("fresh_traversal_words")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(
            fresh > 0 && fresh < largest["register_words"].as_u64().unwrap(),
            "a nested current-buffer computation must define non-input temporary SSA lanes freshly at its lexical root header: {largest}"
        );
    }
}

#[test]
fn fresh_traversals_keep_current_boolean_arguments_private_writes_errors_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for artifact in artifacts(mutable_children::SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let mut expected_error = None;
        for (reference, jit) in [(true, &ordinary), (false, &native)] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (rounds, amount, prime, reject, fails) in [
                    (2u64, 1u32, true, u64::MAX, false),
                    (3, 2, false, u64::MAX, false),
                    (2, 1, true, 1, true),
                    (0, 3, false, 0, false),
                    (3, 1, false, u64::MAX, false),
                    (2, 2, true, u64::MAX, false),
                ] {
                    let args = inputs(
                        &artifact,
                        &[
                            ("values", Data::Matrix([4, 8, 12].map(Data::U32).into())),
                            ("rounds", Data::U64(rounds)),
                            ("amount", Data::U32(amount)),
                            ("prime", Data::Bool(prime)),
                            ("reject", Data::U64(reject)),
                        ],
                        &memory,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut output, mode.execution(), &memory)
                    };
                    if fails {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains("Integrity"), "{error}");
                        assert_eq!(error, *expected_error.get_or_insert(error.clone()));
                        assert!(output.iter().zip(&prior).all(|(v, p)| {
                            v.as_ref()
                                .unwrap()
                                .shares_frozen_storage(p.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        let delta = rounds as u32 * amount;
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Matrix(
                                [4 + delta * (1 + u32::from(prime)), 8 + delta, 12 + delta]
                                    .map(Data::U32)
                                    .into()
                            )
                        );
                        if !reference && rounds > 0 {
                            assert!(counts.phases > 0, "{counts:?}");
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn fresh_value_scope_is_reconstructed_from_the_source_image() {
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
                r.pointer("/numeric/fresh_values")
                    .and_then(|v| v.as_array())
                    .is_some_and(|v| v.len() > 1)
            })
            .expect("source-derived fresh traversal values must be present");
        let values = region["numeric"]["fresh_values"].as_array().unwrap();
        let mut reordered = values.clone();
        reordered.reverse();
        let mut duplicate = values.clone();
        duplicate.push(values[0].clone());
        for value in [
            serde_json::json!(reordered),
            serde_json::json!(duplicate),
            serde_json::json!(&values[1..]),
            serde_json::json!([region["tail"]["carrier"]]),
        ] {
            let mut bad = image.clone();
            bad["descriptor"]["closed_tails"]["regions"][id]["numeric"]["fresh_values"] = value;
            assert!(super::super::tests::rejects_malformed_helper_image(
                &serde_json::to_vec(&bad).unwrap()
            ));
        }
    }
}
